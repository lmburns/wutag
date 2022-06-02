//! `list` - List `Tag`s or `File`s within the database

#![allow(unused)]

// TODO: list files relative to directory as an option
// TODO: take into account color of tag for combinations

use super::App;
use crate::{
    consts::SORT_FILES_EXPL,
    filesystem::contained_path,
    global_opts,
    registry::{
        querier::Query,
        types::{Sort, Tag},
    },
    utils::fmt,
    wutag_error,
};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, CellStruct, ColorChoice, Style, Table,
};
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;

/// Subcommands used for the `list` subcommand
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum ListObject {
    // ╭──────────────────────────────────────────────────────────╮
    // │                           Tags                           │
    // ╰──────────────────────────────────────────────────────────╯
    /// List the `Tags` within the database
    Tags {
        /// Display values along with the tags
        #[clap(name = "with_values", long = "with-values", short = 'V')]
        with_values: bool,

        /// Do not display tag count
        #[clap(name = "no-count", long = "no-count", short = 'c')]
        no_count: bool,

        /// Only display unique occurrences. (See --help)
        #[clap(
            name = "unique",
            long = "unique",
            short = 'u',
            long_help = "When using -cu, unique combinations of tags on files will be displayed. \
                         When using -1u, unique individual tags will be displayed and counted. \
                         When using -1cu, unique individual tags will be displayed one per line"
        )]
        unique: bool,

        /// Sort the tag output. This is more limited than listing files
        #[clap(
            name = "sort",
            long = "sort",
            short = 's',
            long_help = "Sort alphabetically with `no-count`, otherwise it is numerically sorted"
        )]
        sort: bool,

        /// Display one tag per line instead of tags on files
        #[clap(
            name = "one_per_line",
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
            name = "border",
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

        /// Display values along with the tags
        #[clap(
            name = "with_values",
            long = "with-values",
            short = 'V',
            requires = "with_tags"
        )]
        with_values: bool,

        /// Format the tags and files output into columns
        #[clap(
            name = "formatted",
            long = "format",
            short = 'f',
            conflicts_with = "garrulous",
            requires = "with_tags",
            long_help = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        formatted: bool,

        /// Use border separators when formatting output
        #[clap(
            name = "border",
            long,
            short = 'b',
            requires = "formatted",
            long_help = "\
            Use a border around the perimeter of the formatted output, as well as in-between the \
                         lines."
        )]
        border: bool,

        /// Display tags and files on separate lines
        #[clap(
            name = "garrulous",
            long = "garrulous",
            short = 'G',
            conflicts_with = "formatted",
            requires = "with_tags"
        )]
        garrulous: bool,

        /// Sort the file paths. See --help for all ways to sort
        #[clap(
            name = "sort",
            long = "sort",
            short = 's',
            default_value = "none",
            value_name = "method",
            long_help = <String as AsRef<str>>::as_ref(&SORT_FILES_EXPL)
        )]
        sort: Sort,

        /// Display paths relative to current directory (requires --global)
        #[clap(
            name = "relative",
            long = "relative",
            short = 'r',
            requires = "global",
            long_help = "Show paths relative to the current directory. This will only work if the \
                         --global option is given"
        )]
        relative: bool,

        /// Show duplicate file entries
        #[clap(
            name = "duplicates",
            alias = "dupes",
            long = "duplicates",
            short = 'd',
            conflicts_with = "sort",
            long_help = "Display duplicate files based on their blake3 hash"
        )]
        duplicates: bool,
    },
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
    pub(crate) raw: bool,
}

impl App {
    /// List `Tags` or `Files` in the database
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn list(&self, opts: &ListOpts) -> Result<()> {
        log::debug!("ListOpts: {:#?}", opts);

        let mut table = Vec::<Vec<CellStruct>>::new();
        let colorchoice: ColorChoice = self.color_when.into();
        let reg = self.registry.lock().expect("poisoned lock");

        /// If the `raw` option is given, do not colorize
        let raw = |t: &Tag, with_values: bool| {
            let tag = if opts.raw {
                t.name().clone()
            } else {
                self.fmt_tag(t).to_string()
            };

            if with_values {
                // FIX: As of now, only one value per tag because of xattr ???
                let values = reg.values_by_tagid(t.id()).map_or_else(
                    |_| String::from(""),
                    |values| {
                        format!(
                            "{}{}",
                            tern::t!(values.is_empty() ? "" : "="),
                            values
                                .iter()
                                .map(|value| {
                                    let v = value.name();
                                    tern::t!(
                                        opts.raw
                                            ? v.clone()
                                            : v.bold().to_string()
                                    )
                                })
                                .join(",")
                        )
                    },
                );
                format!("{}{}", tag, values)
            } else {
                tag
            }
        };

        match opts.object {
            ListObject::Files {
                with_tags,
                with_values,
                formatted,
                border,
                garrulous,
                sort,
                relative,
                duplicates,
            } => {
                let mut curr_hash = None;
                let files = if duplicates {
                    reg.duplicate_files()?
                } else {
                    reg.files(Some(sort))?
                };

                // TODO: Maybe use this instead of checking global contained_path
                // let files = if duplicates {
                //     reg.duplicate_files()?
                // } else if self.global {
                //     reg.files_by_directory(&self.base_dir)?
                // } else {
                //     reg.files(Some(sort))?
                // };

                for file in files.iter() {
                    // Skips paths that are not contained within current directory to respect the
                    // `-d` flag. Global is just another way to specify -d=~ (list files locally by
                    // default, i.e., no subcommand is given)
                    if !self.global && !contained_path(&file.path(), &self.base_dir) {
                        continue;
                    }

                    // Relative cannot be true without self.global
                    let path = if relative {
                        pathdiff::diff_paths(file.path(), &self.base_dir)
                            .unwrap_or_else(|| file.path())
                    } else {
                        file.path()
                    };

                    if duplicates {
                        let hash = file.hash();
                        if Some(hash) != curr_hash {
                            println!(
                                "{}",
                                tern::t!(opts.raw ? hash.clone() : hash.yellow().bold().to_string())
                            );
                        }
                        curr_hash = Some(hash);
                    }

                    // The first argument to `global_opts` is the local file path
                    // Since --relative requires global, the first branch of path
                    // above will never be ran without --global
                    if opts.raw {
                        global_opts!(
                            self.fmt_raw_local_path(&path),
                            path.display().to_string(),
                            self.global,
                            garrulous
                        );
                    } else if !formatted {
                        global_opts!(
                            self.fmt_local_path(&path),
                            self.fmt_path(&path),
                            self.global,
                            garrulous
                        );
                    }

                    if with_tags {
                        let tags = reg
                            .tags_for_file(file)?
                            .iter()
                            .map(|t| raw(t, with_values))
                            .collect::<Vec<_>>()
                            .join(" ");

                        if formatted {
                            table.push(vec![
                                tern::t!(
                                    self.global
                                        ? self.fmt_path(&path)
                                        : self.fmt_local_path(&path)
                                )
                                .cell(),
                                tags.cell().justify(Justify::Right),
                            ]);
                        } else if garrulous {
                            println!("\t{}", tags);
                        } else {
                            println!(": {}", tags);
                        }
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
                with_values,
                no_count,
                border,
                one_per_line,
                unique,
                sort,
            } => {
                let mut utags = Vec::new();

                let files = reg.files(None)?;

                for file in files.iter() {
                    if !self.global && !contained_path(&file.path(), &self.base_dir) {
                        continue;
                    }

                    if one_per_line {
                        reg.tags_for_file(file)?.iter().for_each(|tag| {
                            utags.push(raw(tag, with_values));
                        });
                    } else {
                        let tags = reg
                            .tags_for_file(file)
                            .map(|tags| {
                                tags.iter().fold(String::new(), |mut acc, t| {
                                    acc.push_str(&format!("{} ", raw(t, with_values)));
                                    acc
                                })
                            })
                            .unwrap_or_default()
                            .clone();

                        utags.push(tags);
                    }
                }

                // TODO: May need to recalculate count after incorporating value display
                let mut vec = utags
                    .iter()
                    .fold(HashMap::new(), |mut acc, t| {
                        *acc.entry(t.clone()).or_insert(0_i32) += 1_i32;
                        acc
                    })
                    .iter()
                    .map(|(s, i)| (s.clone(), *i))
                    .collect::<Vec<(String, i32)>>();

                // Sort numerically if count is included
                if sort {
                    vec = vec.iter().sorted_by_key(|a| -a.1).cloned().collect();
                }

                for (tag, count) in vec {
                    table.push(vec![
                        tag.cell(),
                        tern::t!(
                            opts.raw
                                ? count.to_string().white()
                                : count.to_string().green().bold()
                        )
                        .cell()
                        .justify(Justify::Right),
                    ]);
                }

                if no_count {
                    if unique {
                        utags = utags.iter().unique().cloned().collect_vec();
                    }
                    // Sort alphabetically if no count
                    if sort {
                        utags = utags
                            .iter()
                            .sorted_unstable_by(|a, b| {
                                /// Strip ansi escape sequences from a string
                                macro_rules! strip_ansi {
                                    ($cmp:ident) => {
                                        &String::from_utf8(
                                            strip_ansi_escapes::strip($cmp.as_bytes())
                                                .unwrap_or_default(),
                                        )
                                        .expect("invalid UTF-8")
                                        .to_ascii_lowercase()
                                    };
                                }

                                Ord::cmp(strip_ansi!(b), strip_ansi!(a))
                            })
                            .rev()
                            .cloned()
                            .collect_vec();
                    }
                    for tag in utags {
                        println!("{}", tag);
                    }
                } else {
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
                    .expect("Unable to print table");
                }
            },
        }

        Ok(())
    }
}
