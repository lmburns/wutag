//! `list` - List `Tag`s or `File`s within the database

#![allow(unused)]

// TODO: list files relative to directory as an option
// TODO: take into account color of tag for combinations

use super::{
    uses::{
        contained_path, fmt_local_path, fmt_path, fmt_tag, global_opts, raw_local_path, Args,
        ColorChoice, Colorize, HashMap, Subcommand,
    },
    App,
};

use crate::registry::querier::Query;
use anyhow::{Context, Result};

use cli_table::CellStruct;
use itertools::Itertools;

#[cfg(feature = "prettify")]
use super::uses::{print_stdout, Border, Cell, Justify, Separator, Style, Table};

/// Subcommands used for the `list` subcommand
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum ListObject {
    // ╭──────────────────────────────────────────────────────────╮
    // │                           Tags                           │
    // ╰──────────────────────────────────────────────────────────╯
    /// List the `Tags` within the database
    Tags {
        /// Do not display tag count
        #[clap(name = "no-count", long = "no-count", short = 'c')]
        no_count: bool,

        /// Only display unique occurences. (See --help)
        #[clap(
            long = "unique",
            short = 'u',
            long_help = "When using -cu, unique combinations of tags on files will be displayed. \
                         When using -1u, unique individual tags will be displayed and counted. \
                         When using -1cu, unique individual tags will be displayed one per line"
        )]
        unique: bool,

        // TODO: Add sort kinds
        /// Sort the output
        #[clap(
            long = "sort",
            short = 's',
            long_help = "Sort alphabetically with `no-count`, otherwise it is numerically sorted"
        )]
        sort: bool,

        /// Do not show implied tags
        #[clap(
            long = "implied",
            short = 'i',
            long_help = "Only display tags that were explicitly set"
        )]
        explicit: bool,

        /// Display one tag per line instead of tags on files
        #[clap(
            long = "one-per-line",
            short = '1',
            long_help = "Display one tag per line. Usually tags are displayed as unique \
                         combinations to individual files. That is, if a file is is unique by \
                         having two tags, those two tags will be displayed together and be \
                         counted as one"
        )]
        one_per_line: bool,

        /// Use border separators when formatting output
        #[clap(
            long = "border",
            short = 'b',
            conflicts_with = "no-count",
            long_help = "\
            Use a border around the perimeter of the formatted tags, as well as in-between the \
                         lines."
        )]
        border: bool,
    },

    // ╭──────────────────────────────────────────────────────────╮
    // │                          Files                           │
    // ╰──────────────────────────────────────────────────────────╯
    /// List the `Files` within the database
    Files {
        /// Display tags along with the files
        #[clap(name = "with_tags", long = "with-tags", short = 't')]
        with_tags: bool,

        /// Format the tags and files output into columns
        #[cfg(feature = "prettify")]
        #[clap(
            name = "formatted",
            long = "format",
            short,
            conflicts_with = "garrulous",
            requires = "with_tags",
            long_help = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        formatted: bool,

        /// Use border separators when formatting output
        #[clap(
            long,
            short,
            requires = "formatted",
            long_help = "\
            Use a border around the perimeter of the formatted output, as well as in-between the \
                         lines."
        )]
        border: bool,

        /// Display tags and files on separate lines
        #[clap(
            name = "garrulous",
            long,
            short = 'G',
            conflicts_with = "formatted",
            requires = "with_tags"
        )]
        garrulous: bool,
    },
    // ╭──────────────────────────────────────────────────────────╮
    // │                          Values                          │
    // ╰──────────────────────────────────────────────────────────╯
    // Values,
}

/// Arguments used for the `list` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ListOpts {
    /// Object to list: 'tags', 'files'.
    #[clap(subcommand)]
    pub(crate) object: ListObject,
    /// Output will not be colorized
    #[clap(
        long = "raw",
        short = 'r',
        long_help = "Output of command will not be colorized. This is equivalent to `NO_COLOR=1 \
                     wutag <cmd>`"
    )]
    pub(crate) raw:    bool,
}

impl App {
    /// List `Tags` or `Files` in the database
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn list(&self, opts: &ListOpts) -> Result<()> {
        log::debug!("ListOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());

        let mut table = Vec::<Vec<CellStruct>>::new();
        let colorchoice = match self.color_when.as_ref() {
            "always" => ColorChoice::Always,
            "never" => ColorChoice::Never,
            _ => ColorChoice::Auto,
        };

        let reg = self.registry.lock().expect("poisioned lock");

        match opts.object {
            ListObject::Files {
                with_tags,
                formatted,
                border,
                garrulous,
            } => {
                let files = reg.files(None)?;

                // let query = Query::new("XXX", None);
                // if let Ok(q) = query.parse() {
                //     println!("{:#?}", q.parsed());
                // }

                for file in files.iter() {
                    // Skips paths that are not contained within current directory to respect the
                    // `-d` flag. Global is just another way to specify -d=~ (list files locally by
                    // default, i.e., no subcommand is given)
                    if !self.global && !contained_path(&file.path(), &self.base_dir) {
                        continue;
                    }

                    if opts.raw {
                        global_opts!(
                            raw_local_path(&file.path(), &self.base_dir),
                            file.path().display().to_string(),
                            self,
                            garrulous
                        );
                    } else if !formatted {
                        global_opts!(
                            fmt_local_path(
                                &file.path(),
                                &self.base_dir,
                                self.base_color,
                                self.ls_colors,
                            ),
                            fmt_path(file.path(), self.base_color, self.ls_colors),
                            self,
                            garrulous
                        );
                    }
                    if with_tags {
                        // let tags = self
                        //     .oregistry
                        //     .list_entry_tags(*id)
                        //     .unwrap_or_default()
                        //     .iter()
                        //     .map(|t| {
                        //         if opts.raw {
                        //             t.name().to_owned()
                        //         } else {
                        //             fmt_tag(t).to_string()
                        //         }
                        //     })
                        //     .collect::<Vec<_>>()
                        //     .join(" ");
                        //
                        // if formatted {
                        //     table.push(vec![
                        //         tern::t!(
                        //          self.global
                        //   ? fmt_path(file.path(), self.base_color,
                        // self.ls_colors)
                        //   : fmt_local_path(
                        //          &file.path(),
                        //          &self.base_dir,
                        //          self.base_color,
                        //          self.ls_colors,
                        //         )
                        //         )
                        //         .cell(),
                        //         tags.cell().justify(Justify::Right),
                        //     ]);
                        // } else if garrulous {
                        //     println!("\t{}", tags);
                        // } else {
                        //     println!(": {}", tags);
                        // }
                    } else {
                        println!();
                    }
                }

                if formatted {
                    print_stdout(tern::t!(
                        border
                        ? table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice)
                        : table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    ))
                    .context("unable to print table")?;
                }
            },
            ListObject::Tags {
                no_count,
                border,
                one_per_line,
                unique,
                sort,
                explicit,
            } => {},
        }

        Ok(())
    }
}
