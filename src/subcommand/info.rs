#![allow(unused)]
use super::{
    uses::{
        fmt_tag, Args, Border, Cell, ColorChoice, Colorize, HashMap, Justify, Separator, Style,
        Subcommand, Table,
    },
    App,
};

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct InfoOpts {
    /// TO BE IMPLEMENTED
    /// Do not use color in output
    #[clap(long, short)]
    pub(crate) raw: bool,
}

impl App {
    pub(crate) fn info(&mut self, opts: &InfoOpts) {
        log::debug!("InfoOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        println!(
            "{}: {}",
            "Registry".magenta(),
            self.registry.path.display().to_string().green()
        );

        // let (w, _) = crossterm::terminal::size().unwrap_or((80, 40));
        //
        // let mut table = vec![];
        //
        // for tag in self.registry.list_tags() {
        //     let mut row = vec![];
        //     let mut cumulative = 0;
        //
        //     cumulative += tag.name().len();
        //
        //     if cumulative < ((w - 4) / 2) as usize {
        //         row.push(tag);
        //     } else {
        //         table.push(row.clone().iter().map(|t|
        // t.cell().justify(Justify::Left)));         row.clear();
        //     }
        // }

        self.registry
            .list_tags()
            .map(|tag| {
                if opts.raw {
                    tag.name().white()
                } else {
                    fmt_tag(tag)
                }
            })
            .for_each(|t| println!("{}", t));
    }
}
