//! Tag files colorfully

#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![deny(
    clippy::all,
    // clippy::cargo,
    clippy::complexity,
    clippy::correctness,
    clippy::nursery,
    clippy::pedantic,
    clippy::perf,
    // clippy::restriction,
    clippy::style,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bad_style,
    const_err,
    // dead_code,
    ellipsis_inclusive_range_patterns,
    exported_private_dependencies,
    ill_formed_attribute_input,
    improper_ctypes,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    // missing_debug_implementations,
    missing_docs,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    noop_method_call,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    pointer_structural_match,
    private_in_public,
    pub_use_of_private_extern_crate,
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
    // Fill out documentation
    // clippy::missing_docs_in_private_items,

    // Find this problem
    clippy::pattern_type_mismatch,

    // ?
    clippy::redundant_pub_crate,

    clippy::as_conversions,
    clippy::blanket_clippy_restriction_lints,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cognitive_complexity,
    clippy::create_dir,
    clippy::doc_markdown,
    clippy::else_if_without_else,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::expect_used,
    clippy::exit,
    clippy::implicit_return,
    clippy::indexing_slicing,
    clippy::integer_arithmetic,
    clippy::integer_division,
    clippy::mod_module_files,
    clippy::multiple_inherent_impl,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::string_add,
    clippy::string_slice,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::upper_case_acronyms,
    clippy::unreachable,
    clippy::unwrap_in_result
    // clippy::single_match_else,
)]
#![cfg_attr(
    any(test),
    allow(
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        clippy::unwrap_used,
        clippy::wildcard_enum_match_arm,
    )
)]

mod comp_helper;
mod config;
mod consts;
mod directories;
#[cfg(feature = "encrypt-gpgme")]
mod encryption;
mod exe;
mod filesystem;
mod macros;
mod opt;
mod oregistry;
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
