use super::{uses::*, App};

#[derive(Clap, Clone, Debug)]
pub struct SearchOpts {
    /// If provided output will be raw so that it can be easily piped to other
    /// commands
    #[clap(long, short)]
    pub raw: bool,

    // /// If set to 'true' all entries containing any of provided tags will be
    // /// returned
    // #[clap(long, short)]
    // pub any:     bool,
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
    pub execute:       Option<Vec<String>>,
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
    pub execute_batch: Option<Vec<String>>,
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
    pub tags:          Vec<String>,
    /// Pattern to search tagged files
    #[clap(name = "pattern")]
    pub pattern:       String,
}

impl App {
    pub(crate) fn search(&self, opts: &SearchOpts) {
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive);
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
        sender(&app, &opts, re, tx);
        rec.join().unwrap();
    }
}
