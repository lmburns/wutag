//! `edit` - Edit a `Tag` by changing its `Color`

use super::App;
use crate::{
    utils::{color, fmt},
    wutag_error,
    xattr::tag::DirEntryExt,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, Table,
};
use colored::Color;

// TODO: Change xattr
// TODO: Add a pattern to the query

/// Arguments used for the `edit` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct EditOpts {
    /// Set the color of the tag. See --help for format
    #[clap(
        name = "color",
        long = "color",
        short = 'C',
        validator = |t| color::parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_help = "\
            Set the color of the tag to the specified color. Accepted values are hex \
            colors like '0x000000' or '#1F1F1F' or just plain 'ff000a'. The \
            colors are case insensitive meaning '1f1f1f' is equivalent to '1F1F1F'."
    )]
    pub(crate) color: Option<String>,

    /// New name to replace tag with
    #[clap(
        name = "rename",
        long = "rename",
        short = 'r',
        required_unless_present = "color",
        long_help = "Rename a tag. If both color and rename are present, the rename is carried \
                     out first"
    )]
    pub(crate) rename: Option<String>,

    /// Indicate that the item given is a value
    #[clap(name = "value", long = "value", short = 'V', takes_value = false)]
    pub(crate) value: bool,

    /// Tag to modify
    #[clap(name = "tag", takes_value = true)]
    pub(crate) tag: String,

    /// A glob, regular expression, or fixed-string
    #[clap(
        name = "pattern",
        takes_value = true,
        conflicts_with = "global",
        required_unless_present = "global",
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: Option<String>,
}

impl App {
    /// Edit a `Tag` by changing its color
    pub(crate) fn edit(&mut self, opts: &EditOpts) -> Result<()> {
        log::debug!("EditOpts: {:#?}", opts);

        let mut table = vec![];
        // let re = regex_builder(
        //     &{
        //         if self.pat_regex {
        //             String::from(&opts.pattern)
        //         } else if self.fixed_string {
        //             regex::escape(&opts.pattern)
        //         } else {
        //             glob_builder(&opts.pattern, self.wildcard_matches_sep)
        //         }
        //     },
        //     self.case_insensitive,
        //     self.case_sensitive,
        // );

        let reg = self.registry.lock().expect("poisoned lock");

        // if self.global {
        //     // let exclude_pattern = regex_builder(
        //     //     self.exclude.join("|").as_str(),
        //     //     self.case_insensitive,
        //     //     self.case_sensitive,
        //     // );
        // }

        if let Ok(mut old_tag) = reg.tag_by_name(&opts.tag) {
            let old_tag_color = old_tag.color();
            if let Some(rename) = &opts.rename {
                match reg.update_tag_name(&old_tag, rename) {
                    Ok(new_tag) => {
                        table.push(vec![
                            self.fmt_tag(&old_tag)
                                .to_string()
                                .cell()
                                .justify(Justify::Right),
                            "==>".cell().justify(Justify::Center),
                            self.fmt_tag(&new_tag)
                                .to_string()
                                .cell()
                                .justify(Justify::Left),
                        ]);
                    },
                    Err(e) => {
                        wutag_error!(
                            "{} ==> {}: failed to update tag name: {}",
                            self.fmt_tag(&old_tag),
                            rename.bold(),
                            e
                        );
                    },
                }

                // This needs to be called in two places because once if both the tag name is
                // set and the color, the new tag name needs to have its color
                // changed
                if let Some(color) = opts.color.as_ref() {
                    let rename = reg.tag_by_name(rename)?;
                    match reg.update_tag_color(rename.id(), color) {
                        Ok(new_tag) => {
                            let new_tag = reg.tag(new_tag.id())?;
                            old_tag.set_color(color);
                            table.push(vec![
                                fmt::string(new_tag.name(), &self.tag_effect, old_tag_color)
                                    .to_string()
                                    .cell()
                                    .justify(Justify::Right),
                                "==>".cell().justify(Justify::Center),
                                self.fmt_tag(&new_tag)
                                    .to_string()
                                    .cell()
                                    .justify(Justify::Left),
                            ]);
                        },
                        Err(e) => {
                            wutag_error!(
                                "{} ==> {}: failed to update tag color: {}",
                                self.fmt_tag(&old_tag),
                                old_tag.name().color(
                                    color::parse_color(&color).unwrap_or(Color::BrightWhite)
                                ),
                                e
                            );
                        },
                    }
                }
            } else if let Some(color) = opts.color.as_ref() {
                match reg.update_tag_color(old_tag.id(), color) {
                    Ok(new_tag) => {
                        let new_tag = reg.tag(new_tag.id())?;
                        old_tag.set_color(color);
                        table.push(vec![
                            fmt::string(old_tag.name(), &self.tag_effect, old_tag_color)
                                .to_string()
                                .cell()
                                .justify(Justify::Right),
                            "==>".cell().justify(Justify::Center),
                            self.fmt_tag(&new_tag)
                                .to_string()
                                .cell()
                                .justify(Justify::Left),
                        ]);
                    },
                    Err(e) => {
                        wutag_error!(
                            "{} ==> {}: failed to update tag color: {}",
                            self.fmt_tag(&old_tag),
                            old_tag
                                .name()
                                .color(color::parse_color(&color).unwrap_or(Color::BrightWhite)),
                            e
                        );
                    },
                }
            }

            let ftags = reg.filetags_by_tagid(old_tag.id())?;
            for ftag in ftags.iter() {
                if let Ok(file) = reg.file(ftag.file_id()) {
                    let rename = reg.tag(old_tag.id())?;
                    if let Err(e) = (&file.path()).update_tag(&rename) {
                        wutag_error!(
                            "{}: failed to write new xattr: {}",
                            self.fmt_path(file.path()),
                            e
                        );
                    } else {
                        log::debug!("{}: writing new xattr", file.path().to_string_lossy());
                    }
                }
            }

            if !self.quiet {
                print_stdout(
                    table
                        .table()
                        .border(Border::builder().build())
                        .separator(Separator::builder().build()),
                )
                .expect("failed to print table");
            }
        }

        Ok(())
    }
}
