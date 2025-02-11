#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![deny(
    clippy::all,
    clippy::correctness,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    clippy::pedantic,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bad_style,
    const_err,
    // dead_code,
    keyword_idents,
    improper_ctypes,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    noop_method_call,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    pointer_structural_match,
    private_in_public,
    semicolon_in_expressions_from_macros,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unaligned_references,
    unconditional_recursion,
    unreachable_pub,
    unsafe_code,
    // unused,
    // unused_allocation,
    // unused_comparisons,
    // unused_extern_crates,
    // unused_import_braces,
    // unused_lifetimes,
    // unused_parens,
    // unused_qualifications,
    variant_size_differences,
    while_true
)]
#![allow(
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::shadow_reuse,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::single_match_else,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::upper_case_acronyms
)]

mod comp_helper;
mod config;
mod consts;
#[cfg(feature = "encrypt-gpgme")]
mod encryption;
mod exe;
mod filesystem;
mod macros;
mod opt;
mod registry;
mod subcommand;
#[cfg(feature = "ui")]
mod ui;
mod util;

use colored::Colorize;
use config::Config;
use opt::Opts;
use subcommand::App;

fn main() {
    let config = Config::load_default_location().unwrap_or_default();
    let args = Opts::get_args();
    util::initialize_logging(&args);

    if let Err(e) = App::run(args, &config) {
        wutag_error!("{}", e);
    }
}
