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
    util::{crawler, fmt_err, fmt_path, fmt_tag, glob_builder, regex_builder},
    wutag_error, wutag_info,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use colored::Colorize;
use std::{borrow::Cow, ffi::OsStr, sync::Arc};
use wutag_core::tag::list_tags;

/// Arguments to the `rm` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
    /// Follow symlinks before removing tags
    #[clap(
        name = "follow-symlinks",
        long,
        short = 'f',
        takes_value = false,
        long_help = "Should the symlink be dereferenced before the tag is removed from the file"
    )]
    pub(crate) follow_symlinks: bool,

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
        conflicts_with_all = &["values", "tags"],
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "\
            Delete a tag and its value. If a value is left dangling (i.e., doesn't correspond \
            to a tag), it will remain in the database if `keep_dangling` is true in the \
            configuration file. Otherwise, it will be removed",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    // XXX: Implement
    /// Indicate the item(s) in the given list are values intead of tags
    #[clap(
        name = "values",
        long,
        short = 'V',
        takes_value = false,
        long_help = "By default the item(s) after the pattern are tag(s). To instead act on \
                     values, pass this flag"
    )]
    pub(crate) values: bool,

    // /// Specify value(s) to remove from the database. See --help for more
    // #[clap(
    //     name = "values",
    //     long,
    //     short = 'V',
    //     // requires = "tag",
    //     takes_value = true,
    //     multiple_values = true,
    //     value_terminator = ";",
    //     long_help = "Specify a list of values to remove from the database"
    // )]
    // pub(crate) values: Vec<String>,

    // XXX: Implement
    // /// Specify a tag to remove from the database (default behavior)
    // #[clap(
    //     name = "tag",
    //     long,
    //     short = 't',
    //     takes_value = true,
    //     long_help = "Specify a tag to remove from the database. When deleting a tag, all files \
    //                  which have this tag and no other tags will be removed from the database"
    // )]
    // pub(crate) tag: Option<String>,

    // /// Do not remove the tag or value from the database
    // #[clap(
    //     name = "untag",
    //     long,
    //     short = 'u',
    //     takes_value = false,
    //     long_help = "Normal behavior is to remove the tag or value from the database if there
    // are \                  no other items that correspond to that tag or value. "
    // )]
    // pub(crate) delete: bool,
    // -
    /// A glob pattern like "*.png" (or regex).
    #[clap(name = "pattern", takes_value = true, required = true)]
    pub(crate) pattern: String,

    /// Tags or values (requires --values) to remove from the matching pattern
    #[clap(
        name = "tags",
        takes_value = true,
        required_unless_present_any = &["all"]
    )]
    pub(crate) tags: Vec<String>,
}

macro_rules! red_entry {
    ($t:ident) => {
        $t.name().red().bold()
    };
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
            .map(|item| {
                if opts.values {
                    (
                        Tag::null(""),
                        reg.value_by_name(item, false)
                            .unwrap_or_else(|_| Value::new_noid(item)),
                    )
                } else {
                    (
                        reg.tag_by_name(item).unwrap_or_else(|_| Tag::null(item)),
                        Value::new_noid(""),
                    )
                }
            })
            .collect::<Vec<_>>();

        combos.append(&mut tags);

        // let mut values = opts
        //     .values
        //     .iter()
        //     .map(|value| {
        //         (
        //             Tag::null(""),
        //             reg.value_by_name(value, false)
        //                 .unwrap_or_else(|_| Value::new_noid(value)),
        //         )
        //     })
        //     .collect::<Vec<_>>();
        //
        // combos.append(&mut values);

        println!("COMBOS: {:#?}", combos);

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

            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                opts.follow_symlinks,
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisioned lock");
                    // let tv_pairs = vec![];

                    // println!("FILE: {:#?}", reg.file_by_path(entry.path()));
                    if let Ok(file) = reg.file_by_path(entry.path()) {
                        println!("INIT");

                        let all_tags = reg.tags_for_file(&file)?;

                        println!("ALL  TAGS: {:#?}", all_tags);

                        // ------------Check------------   --Result--     ----Passed----
                        // Tag id, name = Value id, name => TAG, VALUE => Pass tag, value
                        // Tag id, name = Value name     => TAG        => Pass tag, value
                        // Tag name     = Value id, name => VALUE      => Pass tag, value
                        // Tag name     = Value name     => NOTHING    => Pass tag, value
                        // Tag id, name = Value          => TAG        => Pass tag
                        // Tag name     = Value          => NOTHING    => Pass tag
                        // Tag          = Value id, name => VALUE      => Pass value
                        // Tag          = Value name     => NOTHING    => Pass value

                        for (tag, value) in &combos {
                            println!("TAG: {:#?}", tag);
                            println!("value: {:#?}", value);

                            match (
                                tag.is_null_id(),
                                tag.is_null_name(),
                                value.is_null_id(),
                                value.is_null_name(),
                            ) {
                                // Tag and value were passed, both were found in registry
                                (false, false, false, false) => {
                                    // TODO: Remove tag
                                    // TODO: Remove value
                                    wutag_info!("== ffff ==");
                                    println!("OK TAG OK VALUE");
                                },
                                // Tag and value were passed, tag was found in registry
                                (false, false, true, false) => {
                                    // TODO: Remove tag
                                    wutag_info!("== fftf ==");
                                    wutag_error!(
                                        "value ({}) is not found in the registry",
                                        value.name().bold()
                                    );

                                    println!("OK TAG");

                                    if all_tags.is_empty() {
                                        return Ok(());
                                    }
                                },
                                // Tag and value were passed, value was found in registry
                                (true, false, false, false) => {
                                    // TODO: Remove value
                                    wutag_info!("== tfff ==");
                                    wutag_error!(
                                        "tag ({}) is not found in the registry",
                                        red_entry!(tag)
                                    );

                                    println!("OK VALUE");
                                },
                                // Tag and value were passed, neither found in registry
                                (true, false, true, false) => {
                                    wutag_info!("== tftf ==");
                                    wutag_error!(
                                        "tag ({}) and value ({}) are both not found in the \
                                         registry",
                                        red_entry!(tag),
                                        value.name().bold()
                                    );
                                    continue;
                                },
                                // Only tag was passed, found in registry
                                (false, false, true, true) => {
                                    // TODO: Remove tag
                                    wutag_info!("== fftt ==");
                                    println!("OK TAG");

                                    if all_tags.is_empty() {
                                        return Ok(());
                                    }

                                    if let Err(e) =
                                        reg.delete_filetag(file.id(), tag.id(), value.id())
                                    {
                                        wutag_error!("{}", e);
                                    }
                                },
                                // Only tag was passed, not found in registry
                                (true, false, true, true) => {
                                    wutag_info!("== tftt ==");
                                    wutag_error!(
                                        "tag ({}) is not found in the registry",
                                        red_entry!(tag)
                                    );
                                    continue;
                                },
                                // Only value was passed, found in registry
                                (true, true, false, false) => {
                                    // TODO: Remove value
                                    wutag_info!("== ttff ==");
                                    println!("OK VALUE");
                                },
                                // Only value was passed, not found in registry
                                (true, true, true, false) => {
                                    wutag_info!("== tttf ==");
                                    wutag_error!(
                                        "value ({}) is not found in the registry",
                                        value.name().bold()
                                    );
                                    continue;
                                },
                                // Only occurs if tag or value name == "" (should be unreachable)
                                #[allow(clippy::unnested_or_patterns)]
                                (true, true, true, true)
                                | (false, true, _, _)
                                | (false, false, false, true)
                                | (true, false, false, true)
                                | (true, true, false, true) => {
                                    wutag_error!(
                                        "you shouldn't use empty strings for tag or value names"
                                    );
                                    continue;
                                },
                            }
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
