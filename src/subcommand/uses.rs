pub(crate) use anyhow::{Context, Result};
pub(crate) use atty::Stream;
pub(crate) use clap::{ArgSettings, Args, Subcommand, ValueHint};

pub(crate) use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, ColorChoice, Style, Table,
};
pub(crate) use colored::{Color, Colorize};
pub(crate) use crossbeam_channel as channel;
pub(crate) use lexiclean::Lexiclean;
pub(crate) use rayon::prelude::*;
pub(crate) use regex::{
    bytes::{RegexSet, RegexSetBuilder},
    Captures, Regex,
};

pub(crate) use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    env,
    ffi::OsStr,
    fs, io,
    io::{prelude::*, BufRead, BufReader},
    path::PathBuf,
    process,
    sync::Arc,
};

pub(crate) use crate::{
    bold_entry, comp_helper,
    config::{Config, EncryptConfig},
    consts::*,
    err,
    exe::{
        job::{receiver, sender, WorkerResult},
        CommandTemplate,
    },
    filesystem::{contained_path, create_temp_path, osstr_to_bytes, FileTypes},
    global_opts,
    opt::{Command, Opts},
    registry::{self, EntryData, TagRegistry},
    ternary, ui,
    util::{
        collect_stdin_paths, fmt_err, fmt_local_path, fmt_ok, fmt_path, fmt_tag, gen_completions,
        glob_builder, parse_path, raw_local_path, reg_ok, regex_builder, replace,
        systemtime_to_datetime,
    },
    wutag_error, wutag_fatal, wutag_info,
};

pub(crate) use wutag_core::{
    color::{parse_color, parse_color_cli_table},
    tag::{clear_tags, has_tags, list_tags, DirEntryExt, Tag, DEFAULT_COLOR},
};
