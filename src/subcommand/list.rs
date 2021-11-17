// TODO: list files relative to directory as an option
// TODO: add sorting option by tag etc

use super::{
    uses::{
        contained_path, fmt_local_path, fmt_path, fmt_tag, print_stdout, raw_local_path, ternary,
        Args, Border, Cell, ColorChoice, Colorize, HashMap, Justify, Separator, Style, Subcommand,
        Table,
    },
    App,
};
use itertools::Itertools;

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum ListObject {
    Tags {
        /// Do not display tag count
        #[clap(name = "no-count", long = "no-count", short = 'c')]
        no_count: bool,

        /// Only display unique occurences. (See --help)
        #[clap(
            long = "unique",
            short = 'u',
            long_about = "When using -cu, unique combinations of tags on files will be displayed. \
                          When using -1u, unique individual tags will be displayed and counted. \
                          When using -1cu, unique individual tags will be displayed one per line"
        )]
        unique: bool,

        /// Sort the output alphabetically (no-count), numerically otherwise
        #[clap(long, short = 's')]
        sort: bool,

        /// Display one tag per line instead of tags on files
        #[clap(
            long = "one-per-line",
            short = '1',
            long_about = "Display one tag per line. Usually tags are displayed as unique \
                          combinations to individual files. That is, if a file is is unique by \
                          having two tags, those two tags will be displayed together and be \
                          counted as one"
        )]
        one_per_line: bool,

        /// Use border separators when formatting output
        #[clap(
            long,
            short,
            conflicts_with = "no-count",
            long_about = "\
            Use a border around the perimeter of the formatted tags, as well as in-between the \
                          lines."
        )]
        border: bool,
    },
    Files {
        /// Display tags along with the files
        #[clap(name = "with_tags", long = "with-tags", short = 't')]
        with_tags: bool,
        /// Format the tags and files output into columns
        #[clap(
            name = "formatted",
            long = "format",
            short,
            conflicts_with = "garrulous",
            requires = "with_tags",
            long_about = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        formatted: bool,
        /// Use border separators when formatting output
        #[clap(
            long,
            short,
            requires = "formatted",
            long_about = "\
            Use a border around the perimeter of the formatted tags, as well as in-between the \
                          lines."
        )]
        border:    bool,
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
}

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ListOpts {
    /// The object to list. Valid values are: 'tags', 'files'.
    #[clap(subcommand)]
    pub(crate) object: ListObject,
    /// If provided output will be raw so that it can be easily piped to other
    /// commands
    #[clap(long, short)]
    pub(crate) raw:    bool,
}

impl App {
    pub(crate) fn list(&self, opts: &ListOpts) {
        log::debug!("ListOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        let mut table = vec![];
        let colorchoice = match self.color_when.as_ref() {
            "always" => ColorChoice::Always,
            "never" => ColorChoice::Never,
            _ => ColorChoice::Auto,
        };

        match opts.object {
            ListObject::Files {
                with_tags,
                formatted,
                border,
                garrulous,
            } => {
                let global_opts = |local: String, global: String| {
                    if garrulous {
                        ternary!(self.global, println!("{}", global), println!("{}", local));
                    } else if self.global {
                        print!("{}", global);
                    } else {
                        print!("{}", local);
                    }
                };

                for (id, file) in self.registry.list_entries_and_ids() {
                    // Skips paths that are not contained within current directory to respect the
                    // `-d` flag. Global is just another way to specify -d=~
                    // (list files locally by default, i.e., no subcommand is given)
                    if !self.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }

                    if opts.raw {
                        global_opts(
                            raw_local_path(file.path(), &self.base_dir),
                            file.path().display().to_string(),
                        );
                    } else if !formatted {
                        global_opts(
                            fmt_local_path(
                                file.path(),
                                &self.base_dir,
                                self.base_color,
                                self.ls_colors,
                            ),
                            fmt_path(file.path(), self.base_color, self.ls_colors),
                        );
                    }

                    if with_tags {
                        let tags = self
                            .registry
                            .list_entry_tags(*id)
                            .unwrap_or_default()
                            .iter()
                            .map(|t| {
                                if opts.raw {
                                    t.name().to_owned()
                                } else {
                                    fmt_tag(t).to_string()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ");

                        if formatted {
                            table.push(vec![
                                ternary!(
                                    self.global,
                                    fmt_path(file.path(), self.base_color, self.ls_colors),
                                    fmt_local_path(
                                        file.path(),
                                        &self.base_dir,
                                        self.base_color,
                                        self.ls_colors,
                                    )
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
                    print_stdout(ternary!(
                        border,
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice),
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    ))
                    .expect("Unable to print table");
                }
            },
            ListObject::Tags {
                no_count,
                border,
                one_per_line,
                unique,
                sort,
            } => {
                let mut utags = Vec::new();
                for (&id, file) in self.registry.list_entries_and_ids() {
                    if !self.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }

                    macro_rules! raw {
                        ($t:ident) => {
                            if opts.raw {
                                $t.name().white()
                            } else {
                                fmt_tag($t)
                            }
                        };
                    }

                    if one_per_line {
                        self.registry.list_entry_tags(id).iter().for_each(|tags| {
                            tags.iter().for_each(|t| utags.push(format!("{}", raw!(t))));
                        });
                    } else {
                        let tags = self
                            .registry
                            .list_entry_tags(id)
                            .map(|tags| {
                                tags.iter().fold(String::new(), |mut acc, t| {
                                    acc.push_str(&format!("{} ", raw!(t)));
                                    acc
                                })
                            })
                            .unwrap_or_default()
                            .clone();

                        utags.push(tags);
                    }
                }

                let mut vec = utags
                    .iter()
                    .fold(HashMap::new(), |mut acc, t| {
                        *acc.entry(t.clone()).or_insert(0) += 1;
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
                        ternary!(
                            opts.raw,
                            count.to_string().white(),
                            count.to_string().green().bold()
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
                    print_stdout(ternary!(
                        border,
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice),
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    ))
                    .expect("Unable to print table");
                }
            },
        }
    }
}
