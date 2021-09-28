use super::{
    uses::{
        contained_path, fmt_local_path, fmt_path, fmt_tag, print_stdout, raw_local_path, ternary,
        Border, Cell, Clap, ColorChoice, Colorize, HashMap, Justify, Separator, Style, Table,
    },
    App,
};

// It seems that 'name' has to be defined to use 'requires' or 'conflicts_with'
#[derive(Clap, Debug, Clone, PartialEq)]
pub(crate) enum ListObject {
    Tags {
        #[clap(long = "completions", short = 'c', hidden = true)]
        for_completions: bool,
        /// Use border separators when formatting output
        #[clap(
            long,
            short,
            long_about = "\
            Use a border around the perimeter of the formatted tags, as well as in-between the \
                          lines."
        )]
        border:          bool,
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

#[derive(Clap, Debug, Clone, PartialEq)]
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
        let colorchoice = match self.color_when {
            ref s if s == "always" => ColorChoice::Always,
            ref s if s == "never" => ColorChoice::Never,
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
                    // Skips paths that are not contained within one another to respect the `-d`
                    // flag Global is just another way to specify -d ~ (list files locally
                    // by default)
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
                    print_stdout(if border {
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice)
                    } else {
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    })
                    .expect("Unable to print table");
                }
            },
            ListObject::Tags {
                for_completions,
                border,
            } => {
                let mut utags = Vec::new();
                for (&id, file) in self.registry.list_entries_and_ids() {
                    if !self.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }
                    let tags = self
                        .registry
                        .list_entry_tags(id)
                        .map(|tags| {
                            tags.iter().fold(String::new(), |mut acc, t| {
                                acc.push_str(&format!(
                                    "{} ",
                                    if opts.raw {
                                        t.name().white()
                                    } else {
                                        fmt_tag(t)
                                    }
                                ));
                                acc
                            })
                        })
                        .unwrap_or_default()
                        .clone();
                    utags.push(tags);
                }

                utags
                    .iter()
                    .fold(HashMap::new(), |mut acc, t| {
                        *acc.entry(t.clone()).or_insert(0) += 1;
                        acc
                    })
                    .iter()
                    .for_each(|(tag, count)| {
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
                    });

                if for_completions {
                    for tag in utags {
                        println!("{}", tag);
                    }
                } else {
                    print_stdout(if border {
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice)
                    } else {
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    })
                    .expect("Unable to print table");
                }
            },
        }
    }
}
