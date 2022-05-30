use super::App;
use crate::{
    bold_entry, comp_helper,
    utils::{gen_completions, replace},
    wutag_info, Opts,
};
use clap::{Args, CommandFactory, ValueHint};
use clap_complete::{
    shells::{Bash, Elvish, Fish, PowerShell, Zsh},
    Shell,
};
use lexiclean::Lexiclean;
use std::{fs, io, io::Write, path::PathBuf};

// Shell::arg_values()

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct CompletionsOpts {
    /// Shell to print completions. Available shells are: bash, elvish, fish,
    /// powershell, zsh
    #[clap(
        long,
        possible_values = &["bash", "zsh", "powershell", "elvish", "fish"]
    )]
    pub(crate) shell: Shell,

    /// Directory to output completions to
    #[clap(
        long, short = 'd',
        number_of_values = 1,
        value_hint = ValueHint::DirPath,
        validator = |t| fs::metadata(t)
                            .map_err(|_| "must be a valid path")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
    )]
    pub(crate) dir: Option<PathBuf>,
}

impl App {
    // TODO: Refactor
    #[allow(clippy::unused_self)]
    pub(crate) fn print_completions(&self, opts: &CompletionsOpts) {
        log::debug!("CompletionOpts: {:#?}", opts);
        let mut app = Opts::command();

        let buffer = Vec::new();
        let mut cursor = io::Cursor::new(buffer);

        match opts.shell {
            Shell::Bash => gen_completions(Bash, &mut app, &mut cursor),
            Shell::Elvish => gen_completions(Elvish, &mut app, &mut cursor),
            Shell::Fish => gen_completions(Fish, &mut app, &mut cursor),
            Shell::PowerShell => gen_completions(PowerShell, &mut app, &mut cursor),
            Shell::Zsh => gen_completions(Zsh, &mut app, &mut cursor),
            _ => (),
        }

        let buffer = cursor.into_inner();
        let mut script = String::from_utf8(buffer).expect("clap completion not UTF-8");

        // Replace Zsh completion output to make it better
        // I may work on other shells, but am very familiar with Zsh, so that is why it
        // is the only one so far
        match opts.shell {
            Shell::Zsh =>
                for (needle, replacement) in comp_helper::ZSH_COMPLETION_REP {
                    replace(&mut script, needle, replacement)
                        .expect("Failed to replace completion script");
                },
            _ => println!(),
        }

        let out = format!("{}\n", script.trim());

        if let Some(dir) = &opts.dir {
            let filename = match opts.shell {
                Shell::Bash => "wutag.bash",
                Shell::Elvish => "wutag.elvish",
                Shell::Fish => "wutag.fish",
                Shell::PowerShell => "_wutag.ps1",
                Shell::Zsh => "_wutag",
                _ => unreachable!(),
            };
            let outdir = &dir.join(filename).lexiclean();
            fs::write(outdir, out).expect("failed to write completion script");
            wutag_info!(
                "{} completions were printed to {}",
                opts.shell,
                bold_entry!(outdir)
            );
        } else {
            write!(Box::new(io::stdout()), "{}", out)
                .expect("unable to write completions to stdout");
        }
    }
}
