#![allow(unused)]
//! `edit` - Edit a `Tag` by changing its `Color`

use super::App;
use crate::{
    bold_entry, filesystem as wfs,
    registry::types::{Tag, Value},
    utils::{color, crawler, fmt, glob_builder, regex_builder},
    wutag_error, wutag_info,
    xattr::tag::DirEntryExt,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, CellStruct, Table,
};
use colored::{Color, ColoredString};
use std::{borrow::Cow, ffi::OsStr, fmt::Debug, path::PathBuf, sync::Arc};

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
        long_help = "Rename a tag. If both color and rename are present, the rename is carried out first"
    )]
    pub(crate) rename: Option<String>,

    /// Modify all tags that have the given value
    #[clap(name = "all", long = "all", short = 'a', takes_value = false)]
    pub(crate) all: bool,

    // XXX: Implement
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
        // conflicts_with = "all",
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
}

impl App {
    /// Edit a `Tag` by changing its color
    pub(crate) fn edit(&mut self, opts: &EditOpts) -> Result<()> {
        log::debug!("EditOpts: {:#?}", opts);

        let mut table = vec![];
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

        let reg = self.registry.lock().expect("poisoned lock");

        let tag = (!opts.value)
            .then(|| reg.tag_by_name(&opts.tag))
            .transpose()
            .unwrap_or(None);

        let value = opts
            .value
            .then(|| reg.value_by_name(&opts.tag))
            .transpose()
            .unwrap_or(None);

        let formatted_vec = |s1: ColoredString, s2: ColoredString| -> Vec<CellStruct> {
            vec![
                s1.to_string().cell().justify(Justify::Right),
                "==>".cell().justify(Justify::Center),
                s2.to_string().cell().justify(Justify::Left),
            ]
        };

        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );

            if let Some(mut old_tag) = tag {
                let old_tag_color = old_tag.color();
                if let Some(rename) = &opts.rename {
                    // TODO: Check whether tag exists before doing all this stuff
                    match reg.update_tag_name(&old_tag, rename) {
                        Ok(new_tag) => {
                            table.push(formatted_vec(self.fmt_tag(&old_tag), self.fmt_tag(&new_tag)));
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

                    // This needs to be called in two places because if both the tag name
                    // and the color are set, the new tag name needs to have its color
                    // changed
                    if let Some(color) = opts.color.as_ref() {
                        let rename = reg.tag_by_name(rename)?;
                        match reg.update_tag_color(rename.id(), color) {
                            Ok(new_tag) => {
                                let new_tag = reg.tag(new_tag.id())?;
                                old_tag.set_color(color);
                                table.push(formatted_vec(
                                    fmt::string(new_tag.name(), &self.tag_effect, old_tag_color),
                                    self.fmt_tag(&new_tag),
                                ));
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
                                self.fmt_tag(&new_tag).to_string().cell().justify(Justify::Left),
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

                // for file in reg.files(None)?.iter() {
                //     let path = &file.path();
                //
                //     let search_str: Cow<OsStr> = Cow::Owned(path.as_os_str().to_os_string());
                //     let search_bytes = wfs::osstr_to_bytes(search_str.as_ref());
                //     if !self.exclude.is_empty() && exclude_pattern.is_match(&search_bytes) {
                //         continue;
                //     }
                //
                //     if let Some(ref ext) = self.extension {
                //         if !ext.is_match(&search_bytes) {
                //             continue;
                //         }
                //     }
                //
                //     if re.is_match(&search_bytes) {
                //         qprint!(self, "{}:", self.fmt_path(path));
                //     }
                // }

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
        } else {
            drop(reg);

            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");
                    let path = &self.resolve_symlink(entry.path())?;

                    // if let Ok(mut old_tag) = reg.tag_by_name(&opts.tag) {
                    //     let old_tag_color = old_tag.color();
                    //     if let Some(rename) = &opts.rename {
                    //         match reg.update_tag_name(&old_tag, rename) {
                    //             Ok(new_tag) => {
                    //                 table.push(vec![
                    //                     self.fmt_tag(&old_tag)
                    //                         .to_string()
                    //                         .cell()
                    //                         .justify(Justify::Right),
                    //                     "==>".cell().justify(Justify::Center),
                    //                     self.fmt_tag(&new_tag)
                    //                         .to_string()
                    //                         .cell()
                    //                         .justify(Justify::Left),
                    //                 ]);
                    //             },
                    //             Err(e) => {
                    //                 wutag_error!(
                    //                     "{} ==> {}: failed to update tag name: {}",
                    //                     self.fmt_tag(&old_tag),
                    //                     rename.bold(),
                    //                     e
                    //                 );
                    //             },
                    //         }
                    //
                    //         // This needs to be called in two places because once if both the tag
                    //         // name is set and the color, the new tag
                    //         // name needs to have its color changed
                    //         if let Some(color) = opts.color.as_ref() {
                    //             let rename = reg.tag_by_name(rename)?;
                    //             match reg.update_tag_color(rename.id(), color) {
                    //                 Ok(new_tag) => {
                    //                     let new_tag = reg.tag(new_tag.id())?;
                    //                     old_tag.set_color(color);
                    //                     table.push(vec![
                    //                         fmt::string(
                    //                             new_tag.name(),
                    //                             &self.tag_effect,
                    //                             old_tag_color,
                    //                         )
                    //                         .to_string()
                    //                         .cell()
                    //                         .justify(Justify::Right),
                    //                         "==>".cell().justify(Justify::Center),
                    //                         self.fmt_tag(&new_tag)
                    //                             .to_string()
                    //                             .cell()
                    //                             .justify(Justify::Left),
                    //                     ]);
                    //                 },
                    //                 Err(e) => {
                    //                     wutag_error!(
                    //                         "{} ==> {}: failed to update tag color: {}",
                    //                         self.fmt_tag(&old_tag),
                    //                         old_tag.name().color(
                    //                             color::parse_color(&color)
                    //                                 .unwrap_or(Color::BrightWhite)
                    //                         ),
                    //                         e
                    //                     );
                    //                 },
                    //             }
                    //         }
                    //     }
                    // }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
