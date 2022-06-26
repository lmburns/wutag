//! Search for tags or values within the database

use super::App;
use crate::{
    consts::{EXEC_BATCH_EXPL, EXEC_EXPL},
    exe::{
        job::{self, ChannelResult},
        CommandTemplate,
    },
    utils::{glob_builder, regex_builder},
};
use clap::{Args, ValueHint};
use crossbeam_channel as channel;
use std::sync::Arc;

// TODO: Search for files with xattrs

/// Options used for the `search` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct SearchOpts {
    /// No colored output. Should be detected automatically on pipe
    #[clap(
        name = "raw",
        long = "raw",
        short = 'r',
        long_help = "No colored output. Should be detected automatically on a pipe following this \
                     command. This can also be controlled by the 'NO_COLOR' environment variable, or the \
                     flag '--color={never,auto}'"
    )]
    pub(crate) raw: bool,

    /// Display only files in the search results
    #[clap(name = "only-files", long = "only-files", short = 'f')]
    pub(crate) only_files: bool,

    /// Execute a command on each individual file
    #[clap(
        name = "exec",
        long = "exec",
        short = 'x',
        takes_value = true,
        min_values = 1,
        value_name = "cmd",
        value_terminator = ";",
        allow_hyphen_values = true,
        // multiple_occurrences = true,
        long_help = EXEC_EXPL.as_ref(),
        value_hint = ValueHint::CommandName,
    )]
    pub(crate) execute: Option<Vec<String>>,

    /// Execute a command on the batch of matching files
    #[clap(
        name = "exec-batch",
        long = "exec-batch",
        short = 'X',
        takes_value = true,
        min_values = 1,
        value_name = "cmd",
        value_terminator = ";",
        allow_hyphen_values = true,
        // multiple_occurrences = true,
        conflicts_with = "exec",
        long_help = EXEC_BATCH_EXPL.as_ref(),
        value_hint = ValueHint::CommandName,
    )]
    pub(crate) execute_batch: Option<Vec<String>>,

    /// Display tags and files on separate lines
    #[clap(
        name = "garrulous",
        long = "garrulous",
        short = 'G',
        conflicts_with = "only-files"
    )]
    pub(crate) garrulous: bool,

    // XXX: Implement
    /// Files matching all tags (instead of any)
    #[clap(
        name = "all",
        long = "all",
        short = 'a',
        requires = "tags",
        long_help = "The files that result must contain all matching tags. The default behavior is if the \
                     file contains any tag"
    )]
    pub(crate) all: bool,

    // XXX: Implement
    /// Files matching all and only all tags
    #[clap(
        name = "only-all",
        long = "only-all",
        short = 'A',
        conflicts_with = "all",
        requires = "tags",
        long_help = "The files that result must contain all matching tags and only those matching tags"
    )]
    pub(crate) only_all: bool,

    /// Display values along with the tags
    #[clap(
        name = "with_values",
        long = "with-values",
        short = 'V',
        // requires = "with_tags"
    )]
    pub(crate) with_values: bool,

    /// Search just by tags or along with a tag(s)
    #[clap(
        name = "tags",
        long = "tags",
        short = 't',
        long_help = "\
        Limit search results even further by using a tag. To search just by tags use 'search '*' --tag \
                     <tag>'"
    )]
    pub(crate) tags: Vec<String>,

    /// A glob, regular expression, or fixed-string
    #[clap(
        name = "pattern",
        takes_value = true,
        required = true,
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
}

impl App {
    /// `search` subcommand
    ///
    /// Find files that match a given query
    pub(crate) fn search(&self, opts: &SearchOpts) {
        log::debug!("SearchOpts: {:#?}", opts);

        let re = regex_builder(
            &{
                if self.pat_regex {
                    String::from(&opts.pattern)
                } else if self.fixed_string {
                    regex::escape(&opts.pattern)
                } else {
                    glob_builder(&opts.pattern, self.wildcard_matches_sep)
                }
            },
            self.case_insensitive,
            self.case_sensitive,
        );

        let command = opts.execute.as_ref().map_or_else(
            || {
                opts.execute_batch
                    .as_ref()
                    .map(|cmd| CommandTemplate::new_batch(cmd).expect("invalid batch command"))
            },
            |cmd| Some(CommandTemplate::new(cmd)),
        );

        // let command = if let Some(cmd) = &opts.execute {
        //     Some(CommandTemplate::new(cmd))
        // } else if let Some(cmd) = &opts.execute_batch {
        //     Some(CommandTemplate::new_batch(cmd).expect("Invalid batch command"))
        // } else {
        //     None
        // };

        let app = Arc::new(self.clone());
        let opts = Arc::new(opts.clone());
        let re = Arc::new(re);
        let command = command.map(Arc::new);

        let (tx, rx) = channel::unbounded::<ChannelResult>();

        let rec = job::receiver(&app, &opts, command, rx);
        job::sender(&app, &opts, &re, tx);
        rec.join().expect("failed to join receiver `JoinHandle`");
    }
}
