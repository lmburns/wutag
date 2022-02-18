// TODO: Manual
// TODO: confirm all options work

use super::{
    uses::{
        contained_path, fmt_local_path, fmt_path, fs, print_stdout, systemtime_to_datetime, Args,
        Border, Cell, Colorize, Context, Justify, Result, Separator, Table, ValueHint,
    },
    App,
};

use lexiclean::Lexiclean;

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct RepairOpts {
    /// Do not actually update the registry
    #[clap(short = 'd', long = "dry-run")]
    pub(crate) dry_run:    bool,
    /// Remove files from the registry that no longer exist on the system
    #[clap(short = 'R', long = "remove")]
    pub(crate) remove:     bool,
    /// Restrict the repairing to the current directory, or the path given with
    /// -d
    #[clap(short = 'r', long = "restrict")]
    pub(crate) restrict:   bool,
    /// Manually set the file's new location
    #[clap(
        short = 'm',
        long = "manual",
        takes_value = true,
        number_of_values = 2,
        value_terminator = ";",
        value_hint = ValueHint::FilePath,
        validator = |t| fs::metadata(t.split_whitespace().collect::<Vec<_>>()[1])
                            .map_err(|_| "must be a valid path")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
    )]
    pub(crate) manual:     Option<Vec<String>>,
    /// Update the hashsum of all files, including unmodified files
    #[clap(short = 'u', long = "unmodified", takes_value = true)]
    pub(crate) unmodified: bool,
}

impl App {
    pub(crate) fn repair(&mut self, opts: &RepairOpts) -> Result<()> {
        log::debug!("RepairOpts: {:#?}", opts);

        let mut table = vec![];
        let mut removed = false;

        for (id, entry) in self
            .oregistry
            .list_entries_and_ids()
            .map(|(i, e)| (*i, e.clone()))
            .collect::<Vec<(_, _)>>()
        {
            if (!self.global || opts.restrict) && !contained_path(entry.path(), &self.base_dir) {
                continue;
            }

            let exists = entry.path().lexiclean().exists();

            if exists && (entry.changed_since()? || opts.unmodified) {
                table.push(vec![
                    if self.global || !opts.restrict {
                        fmt_path(entry.path(), self.base_color, self.ls_colors)
                    } else {
                        fmt_local_path(
                            entry.path(),
                            &self.base_dir,
                            self.base_color,
                            self.ls_colors,
                        )
                    }
                    .cell(),
                    systemtime_to_datetime(*entry.modtime()).red().cell(),
                    "=>".yellow().cell().justify(Justify::Center),
                    systemtime_to_datetime(entry.get_current_modtime()?)
                        .green()
                        .bold()
                        .cell(),
                ]);

                if !opts.dry_run {
                    self.oregistry.repair_registry(id)?;
                }
            }

            if !exists && opts.remove {
                if !opts.dry_run {
                    self.oregistry.clear_entry(id);
                }

                if !self.quiet {
                    println!(
                        "{}: {}",
                        "Removed".red().bold(),
                        fmt_path(entry.path(), self.base_color, self.ls_colors),
                    );
                }
                removed = true;
            }
        }

        if !self.quiet {
            if removed && !table.is_empty() {
                println!("\n{}:", "Updated".purple().bold());
            }
            print_stdout(
                table
                    .table()
                    .border(Border::builder().build())
                    .separator(Separator::builder().build()),
            )
            .context("failed to print table")?;
        }

        log::debug!("Saving registry...");
        self.save_registry();
        Ok(())
    }
}
