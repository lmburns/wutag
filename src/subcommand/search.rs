use super::{
    uses::{
        channel, glob_builder, receiver, regex_builder, sender, Arc, Args, CommandTemplate,
        ValueHint, WorkerResult, EXEC_BATCH_EXPL, EXEC_EXPL,
    },
    App,
};

#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct SearchOpts {
    /// No colored output. Should be detected automatically on pipe
    #[clap(
        long = "raw",
        short = 'r',
        long_about = "No colored output. Should be detected automatically on a pipe following \
                      this command. This can also be controlled by the 'NO_COLOR' environment \
                      variable, or the flag '--color={never,auto}'"
    )]
    pub(crate) raw: bool,

    /// Display only files in the search results
    #[clap(name = "only-files", long, short = 'f')]
    pub(crate) only_files: bool,

    /// Execute a command on each individual file
    #[rustfmt::skip]
    #[clap(
        name = "exec",
        long = "exec", short = 'x',
        takes_value = true,
        min_values = 1,
        value_name = "cmd",
        value_terminator = ";",
        allow_hyphen_values = true,
        conflicts_with = "exec-batch",
        long_about = EXEC_EXPL.as_ref(),
        value_hint = ValueHint::CommandName,
    )]
    pub(crate) execute:       Option<Vec<String>>,
    /// Execute a command on the batch of matching files
    #[clap(
        name = "exec-batch",
        long = "exec-batch", short = 'X',
        takes_value = true,
        min_values = 1,
        value_name = "cmd",
        value_terminator = ";",
        allow_hyphen_values = true,
        conflicts_with = "exec",
        long_about = EXEC_BATCH_EXPL.as_ref(),
        value_hint = ValueHint::CommandName,
    )]
    pub(crate) execute_batch: Option<Vec<String>>,

    /// Display tags and files on separate lines
    #[clap(name = "garrulous", long, short = 'G', conflicts_with = "only-files")]
    pub(crate) garrulous: bool,

    /// Files matching all tags (instead of any)
    #[clap(
        name = "all",
        long,
        short = 'a',
        requires = "tags",
        long_about = "The files that result must contain all matching tags. The default behavior \
                      is if the file contains any tag"
    )]
    pub(crate) all: bool,

    /// Files matching all and only all tags
    #[clap(
        name = "only-all",
        long = "only-all",
        short = 'A',
        conflicts_with = "all",
        requires = "tags",
        long_about = "The files that result must contain all matching tags and only those \
                      matching tags"
    )]
    pub(crate) only_all: bool,

    /// Search just by tags or along with a tag(s)
    #[clap(
        name = "tags",
        long,
        short,
        long_about = "\
        Limit search results even further by using a tag. To search just by tags use 'wutag search \
                      '*' --tag <tag>'
        "
    )]
    pub(crate) tags: Vec<String>,

    /// Pattern to search tagged files
    #[clap(name = "pattern")]
    pub(crate) pattern: String,
}

impl App {
    pub(crate) fn search(&self, opts: &SearchOpts) {
        log::debug!("SearchOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        #[allow(clippy::manual_map)]
        let command = if let Some(cmd) = &opts.execute {
            Some(CommandTemplate::new(cmd))
        } else if let Some(cmd) = &opts.execute_batch {
            Some(CommandTemplate::new_batch(cmd).expect("Invalid batch command"))
        } else {
            None
        };

        let app = Arc::new(self.clone());
        let opts = Arc::new(opts.clone());
        let re = Arc::new(re);
        let command = command.map(Arc::new);

        let (tx, rx) = channel::unbounded::<WorkerResult>();

        let rec = receiver(&app, &opts, command, rx);
        sender(&app, &opts, &re, tx);
        rec.join().unwrap();
    }
}
