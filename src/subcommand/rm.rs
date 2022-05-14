#![allow(unused)]

/// Remove subcommand
use super::{debug_registry_path, parse_tag_val, App};
use crate::{
    err,
    filesystem::osstr_to_bytes,
    registry::types::{
        tag::{DirEntryExt, Tag, TagValueCombo},
        value::Value,
        ID,
    },
    util::{fmt_err, fmt_path, fmt_tag, fmt_tag_old, glob_builder, reg_ok, regex_builder},
    wutag_error,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use colored::Colorize;
use std::{borrow::Cow, ffi::OsStr, sync::Arc};
use wutag_core::tag::list_tags;

/// Arguments to the `rm` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
    // XXX: Implement
    /// Remove all tags on a file
    #[clap(
        name = "all",
        long,
        short = 'a',
        takes_value = false,
        long_help = "Remove all tags on the matches from the glob or regex. By default this only \
                     untags the file and leaves the tag in the database. "
    )]
    pub(crate) all: bool,

    // XXX: Implement
    /// Specify any number of tag=value pairs to delete
    #[clap(
        name = "pairs",
        long,
        short = 'p',
        takes_value = true,
        conflicts_with = "value",
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "\
            Delete a tag and its value. If a value is left dangling (i.e., doesn't correspond \
            to a tag), it will remain in the database if `keep_dangling` is true in the \
            configuration file. Otherwise, it will be removed",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    // XXX: Implement
    /// Specify a value to remove from from a tag. Requires --tag
    #[clap(
        name = "value",
        long,
        short = 'V',
        takes_value = true,
        requires = "tag",
        long_help = "Specify a value to remove from a tag. Values are basically tags for tags, so \
                     a tag is "
    )]
    pub(crate) value: Option<String>,

    #[clap(
        name = "values",
        long,
        short = 'J',
        takes_value = true,
        requires = "tag",
        long_help = "Specify a value to remove from a tag. Values are basically tags for tags, so \
                     a tag is "
    )]
    pub(crate) values: Vec<String>,

    // XXX: Implement
    /// Specify a tag to remove from the database (default behavior)
    #[clap(
        name = "tag",
        long,
        short = 't',
        takes_value = true,
        long_help = "Specify a tag to remove from the database. When deleting a tag, all files \
                     which have this tag and no other tags will be removed from the database"
    )]
    pub(crate) tag: Option<String>,

    /// Do not remove the tag or value from the database
    #[clap(
        name = "untag",
        long,
        short = 'u',
        takes_value = false,
        long_help = "Normal behavior is to remove the tag or value from the database if there are \
                     no other items that correspond to that tag or value. "
    )]
    pub(crate) delete: bool,

    /// A glob pattern like "*.png" (or regex).
    #[clap(name = "pattern", takes_value = true, required = true)]
    pub(crate) pattern: String,

    /// Tags to remove from the matching pattern
    #[clap(
        name = "tags",
        takes_value = true,
        required_unless_present_any = &["all", "value"]
    )]
    pub(crate) tags: Vec<String>,
}

// TODO: Add stdin method for remove

impl App {
    /// `rm` subcommand
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn rm(&mut self, opts: &RmOpts) -> Result<()> {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("RmOpts: {:#?}", opts);
        debug_registry_path(&self.registry);

        println!("RmOpts: {:#?}", opts);

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        let reg = self.registry.lock().expect("poisioned lock");

        let mut combos = opts
            .pairs
            .iter()
            .map(|(t, v)| {
                (
                    reg.tag_by_name(t).unwrap_or_else(|_| Tag::null(t)),
                    reg.value_by_name(v, false)
                        .unwrap_or_else(|_| Value::new_noid(v)),
                )
            })
            .collect::<Vec<_>>();

        let mut tags = opts
            .tags
            .iter()
            .map(|tag| {
                (
                    reg.tag_by_name(tag).unwrap_or_else(|_| Tag::null(tag)),
                    Value::new_noid(""),
                )
            })
            .collect::<Vec<_>>();

        combos.append(&mut tags);

        let mut values = opts
            .values
            .iter()
            .map(|value| {
                (
                    Tag::null(""),
                    reg.value_by_name(value, false)
                        .unwrap_or_else(|_| Value::new_noid(value)),
                )
            })
            .collect::<Vec<_>>();

        combos.append(&mut values);

        // let mut combos = opts
        //     .pairs
        //     .iter()
        //     .map(|(t, v)| -> Result<TagValueCombo> {
        //         let tag = reg.tag_by_name(t).or_else(|_| {
        //             log::debug!("creating new tag: {}", t);
        //             let tag = opts.color.as_ref().map_or_else(
        //                 || Tag::random_noid(t, &self.colors),
        //                 |color| Tag::new_noid(t, color),
        //             );
        //
        //             reg.insert_tag(&tag)
        //         })?;
        //
        //         let value = reg.value_by_name(v, false).or_else(|_| {
        //             log::debug!("creating new value: {}", v);
        //             reg.insert_value(v)
        //         })?;
        //
        //         let combo = TagValueCombo::new(tag.id(), value.id());
        //
        //         Ok(combo)
        //     })
        //     .collect::<Result<Vec<_>>>()?;

        if self.global {
            // let ctags = opts.tags.iter().collect::<Vec<_>>();
            // let exclude_pattern = regex_builder(
            //     self.exclude.join("|").as_str(),
            //     self.case_insensitive,
            //     self.case_sensitive,
            // );
            //
            // for (&id, entry) in self.oregistry.clone().list_entries_and_ids()
            // {     let search_str: Cow<OsStr> =
            // Cow::Owned(entry.path().as_os_str().to_os_string());
            //     let search_bytes = osstr_to_bytes(search_str.as_ref());
            //     if !self.exclude.is_empty() &&
            // exclude_pattern.is_match(&search_bytes) {
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
            //         list_tags(entry.path())
            //             .map(|tags| {
            //                 tags.iter().fold(Vec::new(), |mut acc, tag| {
            //                     acc.push((
            //                         ctags.iter().find(|c| **c ==
            // &tag.to_string()),
            // tag.clone(),                     ));
            //                     acc
            //                 })
            //             })
            //             .unwrap_or_default()
            //             .iter()
            //             .for_each(|(search, realtag)| {
            //                 if search.is_some() {
            //                     // println!("SEARCH: {:?} REAL: {:?}",
            // search, realtag);
            // self.oregistry.untag_by_name(search.unwrap(), id);
            //                     if !self.quiet {
            //                         println!(
            //                             "{}:",
            //                             fmt_path(entry.path(),
            // self.base_color, self.ls_colors)
            // );                     }
            //
            //                     if let Err(e) =
            // realtag.remove_from(entry.path()) {
            // err!('\t', e, entry);                     } else if
            // !self.quiet {                         print!("\t{}
            // {}", "X".bold().red(), fmt_tag_old(realtag));
            //                     }
            //
            //                     if !self.quiet {
            //                         println!();
            //                     }
            //                 }
            //             });
            //     }
            //     log::debug!("Saving registry...");
            // self.save_registry();
            // }
        } else {
            drop(reg);

            // TODO: Check if xattr is present but not in the registry

            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                // TODO: Add CLI option for symlinks
                true,
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisioned lock");
                    let file = reg.file_by_path(entry.path());
                    // let tv_pairs = vec![];

                    if let Ok(file) = reg.file_by_path(entry.path()) {
                        // let all = reg.tags_for_file(&file)?;
                        // if all.is_empty() {
                        //     return Ok(());
                        // }

                        for (tag, value) in &combos {
                            println!("TAG: {:#?}", tag);
                            println!("value: {:#?}", value);

                            // if tag.is_null_id() {
                            //     if value.is_null() {
                            //         wutag_error!(
                            //             "tag {} not found in the registry",
                            //             tag.name().bold()
                            //         );
                            //         continue;
                            //     }
                            //
                            //     // DO value stuff here
                            // } else if value.is_null_id() {
                            //     if tag.is_null() {
                            //         wutag_error!(
                            //             "value {} not found in the registry",
                            //             value.name().bold()
                            //         );
                            //         continue;
                            //     }
                            //
                            //     // do tag stuff here
                            // }
                        }
                    }

                    // let tags = opts
                    //     .tags
                    //     .iter()
                    //     .map(|tag| {
                    //         if let Some(id) = id {
                    //             self.oregistry.untag_by_name(tag, id);
                    //         }
                    //         entry.get_tag(tag)
                    //     })
                    //     .collect::<Vec<_>>();

                    // if tags.is_empty() {
                    //     return Ok(());
                    // }
                    //
                    // if !self.quiet {
                    //     println!(
                    //         "{}:",
                    //         fmt_path(entry.path(), self.base_color, self.ls_colors)
                    //     );
                    // }
                    // for tag in tags {
                    //     let tag = match tag {
                    //         Ok(tag) => tag,
                    //         Err(e) => {
                    //             err!('\t', e, entry);
                    //             return Ok(());
                    //         },
                    //     };
                    //     if let Err(e) = entry.untag(&tag) {
                    //         err!('\t', e, entry);
                    //     } else {
                    //         print!("\t{} {}", "X".bold().red(), fmt_tag_old(&tag));
                    //     }
                    // }
                    // if !self.quiet {
                    //     println!();
                    // }
                    // log::debug!("Saving registry...");
                    // self.save_registry();

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
