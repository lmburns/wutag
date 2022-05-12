#![allow(unused)]

/// Remove subcommand
use super::{debug_registry_path, parse_tag_val, App};
use crate::{
    err,
    filesystem::osstr_to_bytes,
    util::{fmt_err, fmt_path, fmt_tag, fmt_tag_old, glob_builder, reg_ok, regex_builder},
};
use clap::{Args, ValueHint};
use colored::Colorize;
use std::{borrow::Cow, ffi::OsStr, sync::Arc};
use wutag_core::tag::{list_tags, DirEntryExt};

/// Arguments to the `rm` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
    /// Delete all tags on a file
    #[clap(
        name = "all",
        long,
        short = 'a',
        takes_value = false,
        long_help = "Delete all tags on the matching pattern's results"
    )]
    pub(crate) all: bool,

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

    /// Specify a value to remove
    #[clap(
        name = "value",
        long,
        short = 'V',
        takes_value = true,
        long_help = "Specify a value to remove from the database"
    )]
    pub(crate) value: Option<String>,

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
    pub(crate) fn rm(&mut self, opts: &RmOpts) {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("RmOpts: {:#?}", opts);
        debug_registry_path(&self.registry);
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        let reg = self.registry.lock().expect("poisioned lock");

        println!("RmOpts: {:#?}", opts);

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

            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisioned lock");

                    let file = reg.file_by_path(entry.path());
                    println!("entry: {:#?}", file);

                    let tags = opts
                        .tags
                        .iter()
                        .map(|tag| reg.tag_by_name(tag))
                        .collect::<Vec<_>>();

                    println!("TAGS: {:#?}", tags);

                    // let tags = opts.tags.iter().map(|tag| {
                    //     if let Some(f) = file {
                    //     }
                    // });

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
    }
}
