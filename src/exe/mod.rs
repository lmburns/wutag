mod command;
pub(crate) mod exits;
pub(crate) mod input;
pub(crate) mod job;
pub(crate) mod token;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;

pub(crate) use self::{
    command::execute_command,
    exits::ExitCode,
    input::{
        basename, dirname, remove_extension, strip_current_dir, wutag_clear_tag, wutag_colored_dir,
        wutag_cp_tag, wutag_dir, wutag_remove_tag, wutag_set_tag,
    },
    token::Token,
};

/// Execution mode of the command
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExecutionMode {
    /// Command is executed for each search result
    OneByOne,
    /// Command is run for a batch of results at once
    Batch,
}

/// Represents a template that is utilized to generate command strings.
///
/// The template is meant to be coupled with an input in order to generate a
/// command. The `generate_and_execute()` method will be used to generate a
/// command and execute it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandTemplate {
    args: Vec<ArgumentTemplate>,
    mode: ExecutionMode,
}

impl CommandTemplate {
    pub(crate) fn new<I, S>(input: I) -> CommandTemplate
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::build(input, ExecutionMode::OneByOne)
    }

    pub(crate) fn new_batch<I, S>(input: I) -> Result<CommandTemplate>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let cmd = Self::build(input, ExecutionMode::Batch);
        if cmd.number_of_tokens() > 1 {
            return Err(anyhow!("Only one placeholder allowed for batch commands"));
        }
        if cmd.args[0].has_tokens() {
            return Err(anyhow!(
                "First argument of exec-batch is expected to be a fixed executable"
            ));
        }
        Ok(cmd)
    }

    fn build<I, S>(input: I, mode: ExecutionMode) -> CommandTemplate
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        static PLACEHOLDER_PATTERN: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\{(/?\.?.?|//|@?[srxc]?)\}").unwrap());

        let mut args = Vec::new();
        let mut has_placeholder = false;

        for arg in input {
            let arg = arg.as_ref();

            let mut tokens = Vec::new();
            let mut start = 0;

            for placeholder in PLACEHOLDER_PATTERN.find_iter(arg) {
                // Leading text before the placeholder.
                if placeholder.start() > start {
                    tokens.push(Token::Text(arg[start..placeholder.start()].to_owned()));
                }

                start = placeholder.end();

                match placeholder.as_str() {
                    "{}" => tokens.push(Token::Placeholder),
                    "{.}" => tokens.push(Token::NoExt),
                    "{/}" => tokens.push(Token::Basename),
                    "{//}" => tokens.push(Token::Parent),
                    "{/.}" => tokens.push(Token::BasenameNoExt),
                    "{..}" => tokens.push(Token::Wutag),
                    "{@}" => tokens.push(Token::WutagColored),
                    "{@s}" => tokens.push(Token::WutagSet),
                    "{@r}" => tokens.push(Token::WutagRemove),
                    "{@x}" => tokens.push(Token::WutagClear),
                    "{@c}" => tokens.push(Token::WutagCp),
                    _ => unreachable!("Unhandled placeholder"),
                }

                has_placeholder = true;
            }

            // Without a placeholder, the argument is just fixed text.
            if tokens.is_empty() {
                args.push(ArgumentTemplate::Text(arg.to_owned()));
                continue;
            }

            if start < arg.len() {
                // Trailing text after last placeholder.
                tokens.push(Token::Text(arg[start..].to_owned()));
            }

            args.push(ArgumentTemplate::Tokens(tokens));
        }

        // If a placeholder token was not supplied, append one at the end of the
        // command.
        if !has_placeholder {
            args.push(ArgumentTemplate::Tokens(vec![Token::Placeholder]));
        }

        CommandTemplate { args, mode }
    }

    fn number_of_tokens(&self) -> usize {
        self.args.iter().filter(|arg| arg.has_tokens()).count()
    }

    fn split_first_arg(&self, input: impl AsRef<Path>) -> Vec<ArgumentTemplate> {
        let input = input.as_ref();
        let mut cloned_args = self.args.clone();
        log::debug!("Cloned args: {:?}", cloned_args);
        cloned_args.remove(0);
        log::debug!("Cloned args: removed arg: {:?}", cloned_args);

        let mut new_args = self.args[0]
            .clone()
            .generate(input)
            .to_str()
            .unwrap()
            .split(' ')
            .map(String::from)
            .map(ArgumentTemplate::Text)
            .collect::<Vec<ArgumentTemplate>>();

        log::debug!("New args: {:?}", new_args);
        new_args.append(&mut cloned_args);
        log::debug!("New args after append: {:?}", new_args);
        new_args
    }

    /// Generates and executes a command.
    ///
    /// Using the internal `args` field, and a supplied `input` variable, a
    /// `Command` will be build. Once all arguments have been processed, the
    /// command is executed.
    pub(crate) fn generate_and_execute(&self, input: &Path, out_perm: Arc<Mutex<()>>) -> ExitCode {
        let input = strip_current_dir(input);

        log::debug!("=== Args before ===: {:#?}", self.args);
        let args = if self.args[0].contains_wutag() {
            self.split_first_arg(&input)
        } else {
            self.args.clone()
        };
        log::debug!("=== Args after ===: {:#?}", args);

        let mut cmd = Command::new(args[0].generate(&input));
        for arg in &args[1..] {
            cmd.arg(arg.generate(&input));
        }

        log::debug!("=== Final command ===: {:#?}", cmd);
        execute_command(cmd, &out_perm)
    }

    pub(crate) fn in_batch_mode(&self) -> bool {
        self.mode == ExecutionMode::Batch
    }

    pub(crate) fn generate_and_execute_batch<I>(&self, paths: I) -> ExitCode
    where
        I: Iterator<Item = PathBuf>,
    {
        // FIX: Have to change batch limit of 1 token
        let mut cmd = Command::new(self.args[0].generate(""));
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let mut paths: Vec<_> = paths.collect();
        let mut has_path = false;

        for arg in &self.args[1..] {
            if arg.has_tokens() {
                paths.sort();

                // A single `Tokens` is expected
                // So we can directly consume the iterator once and for all
                for path in &mut paths {
                    cmd.arg(arg.generate(strip_current_dir(path)));
                    has_path = true;
                }
            } else {
                cmd.arg(arg.generate(""));
            }
        }

        if has_path {
            execute_command(cmd, &Mutex::new(()))
        } else {
            ExitCode::Success
        }
    }
}

/// Represents a template for a single command argument.
///
/// The argument is either a collection of `Token`s including at least one
/// placeholder variant, or a fixed text.
#[derive(Clone, Debug, PartialEq)]
enum ArgumentTemplate {
    Tokens(Vec<Token>),
    Text(String),
}

impl ArgumentTemplate {
    pub(crate) fn has_tokens(&self) -> bool {
        matches!(self, ArgumentTemplate::Tokens(_))
    }

    pub(crate) fn contains_wutag(&self) -> bool {
        if let ArgumentTemplate::Tokens(ref tokens) = *self {
            tokens[0] == Token::Wutag
                || tokens[0] == Token::WutagColored
                || tokens[0] == Token::WutagSet
                || tokens[0] == Token::WutagRemove
                || tokens[0] == Token::WutagClear
                || tokens[0] == Token::WutagCp
        } else {
            false
        }
    }

    /// Generate an argument from this template. If path_separator is Some, then
    /// it will replace the path separator in all placeholder tokens. Text
    /// arguments and tokens are not affected by path separator
    /// substitution.
    pub(crate) fn generate(&self, path: impl AsRef<Path>) -> OsString {
        use self::Token::{
            Basename, BasenameNoExt, NoExt, Parent, Placeholder, Text, Wutag, WutagClear,
            WutagColored, WutagCp, WutagRemove, WutagSet,
        };
        let path = path.as_ref();

        match *self {
            ArgumentTemplate::Tokens(ref tokens) => {
                let mut s = OsString::new();
                for token in tokens {
                    match *token {
                        Basename => s.push(basename(path)),
                        BasenameNoExt => s.push(&remove_extension(basename(path).as_ref())),
                        NoExt => s.push(&remove_extension(path)),
                        Parent => s.push(&dirname(path)),
                        Placeholder => s.push(path),
                        Wutag => s.push(&wutag_dir(path)),
                        WutagColored => s.push(&wutag_colored_dir(path)),
                        WutagSet => s.push(&wutag_set_tag(path)),
                        WutagRemove => s.push(&wutag_remove_tag(path)),
                        WutagClear => s.push(&wutag_clear_tag(path)),
                        WutagCp => s.push(&wutag_cp_tag(path)),
                        Text(ref string) => s.push(string),
                    }
                }
                s
            },
            ArgumentTemplate::Text(ref text) => OsString::from(text),
        }
    }
}
