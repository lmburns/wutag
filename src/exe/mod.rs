#![allow(unused)]
//! Execute a command on a result from a `search`

mod command;
pub(crate) mod exits;
pub(crate) mod input;
pub(crate) mod job;
pub(crate) mod token;

use std::{
    ffi::OsString,
    io, iter,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
};

use crate::regex;
use anyhow::{anyhow, Result};
use argmax::Command;

pub(crate) use self::{
    command::{execute_commands, handle_cmd_error},
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
    Single,
    /// Command is run for a batch of results at once
    Batch,
}

/// Representation of multiple exec commands
#[derive(Debug, Clone, PartialEq)]
pub struct CommandSet {
    /// Mode that commands are
    mode:     ExecutionMode,
    /// The actual commands to be executed
    commands: Vec<CommandTemplate>,
}

impl CommandSet {
    /// Create a new [`CommandSet`]
    pub(crate) fn new<I, S>(input: I) -> Result<CommandSet>
    where
        I: IntoIterator<Item = Vec<S>>,
        S: AsRef<str>,
    {
        Ok(CommandSet {
            mode:     ExecutionMode::Single,
            commands: input
                .into_iter()
                .map(CommandTemplate::new)
                .collect::<Result<_>>()?,
        })
    }

    /// Create a new batch [`CommandSet`]
    pub(crate) fn new_batch<I, S>(input: I) -> Result<CommandSet>
    where
        I: IntoIterator<Item = Vec<S>>,
        S: AsRef<str>,
    {
        Ok(CommandSet {
            mode:     ExecutionMode::Batch,
            commands: input
                .into_iter()
                .map(|args| {
                    let cmd = CommandTemplate::new(args)?;

                    if cmd.number_of_tokens() > 1 {
                        return Err(anyhow!("only one placeholder allowed for batch commands"));
                    }
                    if cmd.args[0].has_tokens() {
                        return Err(anyhow!(
                            "first argument of --exec-batch is expected to be an executable"
                        ));
                    }
                    Ok(cmd)
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }

    /// Is the command in batch mode?
    pub(crate) fn in_batch_mode(&self) -> bool {
        self.mode == ExecutionMode::Batch
    }

    // /// Generates and executes a command.
    // ///
    // /// Using the internal `args` field, and a supplied `input` variable, a
    // /// `Command` will be build. Once all arguments have been processed, the
    // /// command is executed.
    // pub(crate) fn generate_and_execute(&self, input: &Path, out_perm:
    // &Arc<Mutex<()>>) -> ExitCode {     let input = strip_current_dir(input);
    //
    //     let args = if self.args[0].contains_wutag() {
    //         self.split_first_arg(&input)
    //     } else {
    //         self.args.clone()
    //     };
    //
    //     let mut cmd = Command::new(args[0].generate(&input));
    //     for arg in &args[1..] {
    //         cmd.arg(arg.generate(&input));
    //     }
    //
    //     execute_command(cmd, out_perm)
    // }

    pub(crate) fn execute(
        &self,
        input: &Path,
        out_perm: Arc<Mutex<()>>,
        buffer_output: bool,
    ) -> ExitCode {
        let commands = self.commands.iter().map(|c| c.generate(input));
        execute_commands(commands, &out_perm, buffer_output)
    }

    // pub(crate) fn generate_and_execute_batch<I>(&self, paths: I) -> ExitCode
    // where
    //     I: Iterator<Item = PathBuf>,
    // {
    //     // FIX: Have to change batch limit of 1 token
    //     let mut cmd = Command::new(self.args[0].generate(""));
    //     cmd.stdin(Stdio::inherit());
    //     cmd.stdout(Stdio::inherit());
    //     cmd.stderr(Stdio::inherit());
    //
    //     let mut paths = paths.collect::<Vec<_>>();
    //     let mut has_path = false;
    //
    //     for arg in &self.args[1..] {
    //         if arg.has_tokens() {
    //             paths.sort();
    //
    //             // A single `Tokens` is expected
    //             // So we can directly consume the iterator once and for all
    //             for path in &mut paths {
    //                 cmd.arg(arg.generate(strip_current_dir(path)));
    //                 has_path = true;
    //             }
    //         } else {
    //             cmd.arg(arg.generate(""));
    //         }
    //     }
    //
    //     if has_path {
    //         execute_command(cmd, &Mutex::new(()))
    //     } else {
    //         ExitCode::Success
    //     }
    // }

    /// Generate the command and execute it if it is a `batch`
    pub(crate) fn execute_batch<I>(&self, paths: I, limit: usize) -> ExitCode
    where
        I: Iterator<Item = PathBuf>,
    {
        let builders = self
            .commands
            .iter()
            .map(|c| CommandBuilder::new(c, limit))
            .collect::<Result<Vec<_>>>();

        match builders {
            Ok(mut builders) => {
                for path in paths {
                    for builder in &mut builders {
                        if let Err(e) = builder.push(&path) {
                            return handle_cmd_error(Some(&builder.cmd), e.into());
                        }
                    }
                }

                for builder in &mut builders {
                    if let Err(e) = builder.finish() {
                        return handle_cmd_error(Some(&builder.cmd), e.into());
                    }
                }

                ExitCode::Success
            },
            Err(e) => handle_cmd_error(None, e),
        }
    }
}

/// Template that generates [`Command`] strings
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandTemplate {
    /// Arguments to the command
    args: Vec<ArgumentTemplate>,
}

impl CommandTemplate {
    /// Create a new [`CommandTemplate`]
    fn new<I, S>(input: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = Vec::new();
        let mut has_placeholder = false;

        // Regular expression to match `Token` patterns
        let reg = regex!(r"\{(/?\.?.?|//|@?[srxc]?)\}");

        for arg in input {
            let arg = arg.as_ref();

            let mut tokens = Vec::new();
            let mut start = 0;

            for placeholder in reg.find_iter(arg) {
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

        if args.is_empty() {}

        // If a placeholder token was not supplied, append one at the end of the
        // command.
        if !has_placeholder {
            args.push(ArgumentTemplate::Tokens(vec![Token::Placeholder]));
        }

        Ok(Self { args })
    }

    /// Return the number of [`Token`]s within the command
    fn number_of_tokens(&self) -> usize {
        self.args.iter().filter(|arg| arg.has_tokens()).count()
    }

    /// Generates and executes a command
    fn generate(&self, input: &Path) -> Result<Command> {
        let mut cmd = Command::new(self.args[0].generate(&input));

        // let args = if self.args[0].contains_wutag() {
        //     self.split_first_arg(&input)
        // } else {
        //     self.args.clone()
        // };

        for arg in &self.args[1..] {
            cmd.try_arg(arg.generate(&input))?;
        }
        Ok(cmd)
    }

    /// Split the first argument in the command
    fn split_first_arg<P: AsRef<Path>>(&self, input: P) -> Vec<ArgumentTemplate> {
        let input = input.as_ref();
        let mut cloned_args = self.args.clone();
        log::debug!("Cloned args: {:?}", cloned_args);
        cloned_args.remove(0);
        log::debug!("Cloned args: removed arg: {:?}", cloned_args);

        let mut new_args = self.args[0]
            .clone()
            .generate(input)
            .to_string_lossy()
            .split(' ')
            .map(String::from)
            .map(ArgumentTemplate::Text)
            .collect::<Vec<ArgumentTemplate>>();

        log::debug!("New args: {:?}", new_args);
        new_args.append(&mut cloned_args);
        log::debug!("New args after append: {:?}", new_args);
        new_args
    }

    // fn generate(&self, input: &Path, path_separator: Option<&str>) ->
    // Result<Command> {     let mut cmd =
    // Command::new(self.args[0].generate(&input, path_separator));     for arg
    // in &self.args[1..] {         cmd.try_arg(arg.generate(&input,
    // path_separator))?;     }
    //     Ok(cmd)
    // }

    // /// Generates and executes a command.
    // ///
    // /// Using the internal `args` field, and a supplied `input` variable, a
    // /// `Command` will be build. Once all arguments have been processed, the
    // /// command is executed.
    // pub(crate) fn generate_and_execute(&self, input: &Path, out_perm:
    // &Arc<Mutex<()>>) -> ExitCode {     let input = strip_current_dir(input);
    //
    //     log::debug!("=== Args before ===: {:#?}", self.args);
    //     let args = if self.args[0].contains_wutag() {
    //         self.split_first_arg(&input)
    //     } else {
    //         self.args.clone()
    //     };
    //     log::debug!("=== Args after ===: {:#?}", args);
    //
    //     let mut cmd = Command::new(args[0].generate(&input));
    //     for arg in &args[1..] {
    //         cmd.arg(arg.generate(&input));
    //     }
    //
    //     log::debug!("=== Final command ===: {:#?}", cmd);
    //     execute_command(cmd, out_perm)
    // }

    // /// Generate the command and execute it if it is a `batch`
    // pub(crate) fn generate_and_execute_batch<I>(&self, paths: I) -> ExitCode
    // where
    //     I: Iterator<Item = PathBuf>,
    // {
    //     // FIX: Have to change batch limit of 1 token
    //     let mut cmd = Command::new(self.args[0].generate(""));
    //     cmd.stdin(Stdio::inherit());
    //     cmd.stdout(Stdio::inherit());
    //     cmd.stderr(Stdio::inherit());
    //
    //     let mut paths = paths.collect::<Vec<_>>();
    //     let mut has_path = false;
    //
    //     for arg in &self.args[1..] {
    //         if arg.has_tokens() {
    //             paths.sort();
    //
    //             // A single `Tokens` is expected
    //             // So we can directly consume the iterator once and for all
    //             for path in &mut paths {
    //                 cmd.arg(arg.generate(strip_current_dir(path)));
    //                 has_path = true;
    //             }
    //         } else {
    //             cmd.arg(arg.generate(""));
    //         }
    //     }
    //
    //     if has_path {
    //         execute_command(cmd, &Mutex::new(()))
    //     } else {
    //         ExitCode::Success
    //     }
    // }
}

// ╭──────────────────────────────────────────────────────────╮
// │                      CommandBuilder                      │
// ╰──────────────────────────────────────────────────────────╯

// A command that is to be ran in batch mode
#[derive(Debug)]
struct CommandBuilder {
    /// Arguments added before the command has began building
    pre_args:  Vec<OsString>,
    /// Arguments that will be exec TODO:
    path_arg:  ArgumentTemplate,
    /// Arguments added after the command has finished building
    post_args: Vec<OsString>,
    /// The command that is to be executed
    cmd:       Command,
    /// Current number of arguments
    count:     usize,
    /// Maximum number of arguments that are allowed
    limit:     usize,
}

impl CommandBuilder {
    /// Create a new [`CommandBuilder`]
    fn new(template: &CommandTemplate, limit: usize) -> Result<Self> {
        let mut pre_args = vec![];
        let mut path_arg = None;
        let mut post_args = vec![];

        for arg in &template.args {
            if arg.has_tokens() {
                path_arg = Some(arg.clone());
            } else if path_arg == None {
                pre_args.push(arg.generate(""));
            } else {
                post_args.push(arg.generate(""));
            }
        }

        let cmd = Self::new_command(&pre_args)?;

        Ok(Self {
            pre_args,
            path_arg: path_arg.unwrap(),
            post_args,
            cmd,
            count: 0,
            limit,
        })
    }

    /// Create a new [`Command`]
    fn new_command(pre_args: &[OsString]) -> io::Result<Command> {
        let mut cmd = Command::new(&pre_args[0]);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        cmd.try_args(&pre_args[1..])?;
        Ok(cmd)
    }

    /// Add an argument to the [`Command`]
    ///
    /// If the limit is reached or the argument will not fit, the command is
    /// finished
    fn push(&mut self, path: &Path) -> io::Result<()> {
        if self.limit > 0 && self.count >= self.limit {
            self.finish()?;
        }

        let arg = self.path_arg.generate(path);
        if !self
            .cmd
            .args_would_fit(iter::once(&arg).chain(&self.post_args))
        {
            self.finish()?;
        }

        self.cmd.try_arg(arg)?;
        self.count += 1;
        Ok(())
    }

    /// Finish the building of the [`Command`]
    fn finish(&mut self) -> io::Result<()> {
        if self.count > 0 {
            self.cmd.try_args(&self.post_args)?;
            self.cmd.status()?;

            self.cmd = Self::new_command(&self.pre_args)?;
            self.count = 0;
        }

        Ok(())
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │                     ArgumentTemplate                     │
// ╰──────────────────────────────────────────────────────────╯

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
    /// Does the [`ArgumentTemplate`] contain any [`Token`]s
    pub(crate) const fn has_tokens(&self) -> bool {
        matches!(self, ArgumentTemplate::Tokens(_))
    }

    /// Does the [`ArgumentTemplate`] contain a `wutag` command
    pub(crate) fn contains_wutag(&self) -> bool {
        if let ArgumentTemplate::Tokens(ref tokens) = *self {
            if tokens.get(0).is_none() {
                return false;
            }

            matches!(
                tokens[0],
                Token::Wutag
                    | Token::WutagColored
                    | Token::WutagSet
                    | Token::WutagRemove
                    | Token::WutagClear
                    | Token::WutagCp
            )
        } else {
            false
        }
    }

    /// Generate an argument from this template. If path_separator is Some, then
    /// it will replace the path separator in all placeholder tokens. Text
    /// arguments and tokens are not affected by path separator
    /// substitution.
    pub(crate) fn generate<P: AsRef<Path>>(&self, path: P) -> OsString {
        use self::Token as T;
        let path = path.as_ref();

        match *self {
            ArgumentTemplate::Tokens(ref tokens) => {
                let mut s = OsString::new();
                for token in tokens {
                    match *token {
                        T::Basename => s.push(basename(path)),
                        T::BasenameNoExt => s.push(&remove_extension(basename(path).as_ref())),
                        T::NoExt => s.push(&remove_extension(path)),
                        T::Parent => s.push(&dirname(path)),
                        T::Placeholder => s.push(path),
                        T::Wutag => s.push(&wutag_dir(path)),
                        T::WutagColored => s.push(&wutag_colored_dir(path)),
                        T::WutagSet => s.push(&wutag_set_tag(path)),
                        T::WutagRemove => s.push(&wutag_remove_tag(path)),
                        T::WutagClear => s.push(&wutag_clear_tag(path)),
                        T::WutagCp => s.push(&wutag_cp_tag(path)),
                        T::Text(ref string) => s.push(string),
                    }
                }
                s
            },
            ArgumentTemplate::Text(ref text) => OsString::from(text),
        }
    }
}
