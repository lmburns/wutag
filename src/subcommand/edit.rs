//! `edit` - Edit a `Tag` by changing its `Color`

use super::App;
use crate::util::fmt_tag;
use clap::Args;
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, Table,
};
use wutag_core::color::parse_color;

/// Arguments used for the `edit` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct EditOpts {
    /// Set the color of the tag to the specified color. Accepted values are hex
    /// colors like '0x000000' or '#1F1F1F' or just plain 'ff000a'. The
    /// colors are case insensitive meaning '1f1f1f' is equivalent to
    /// '1F1F1F'.
    #[clap(
        name = "color",
        long, short,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
    )]
    pub(crate) color: Option<String>,

    #[clap(
        name = "rename",
        long,
        short,
        required_unless_present = "color",
        long_help = "Rename a tag. If both color and rename are present, the rename is carried \
                     out first"
    )]
    /// New name to replace tag with
    pub(crate) rename: Option<String>,

    /// The tag to edit
    #[clap(name = "tag")]
    pub(crate) tag: String,
}

impl App {
    /// Edit a `Tag` by changing its color
    pub(crate) fn edit(&mut self, opts: &EditOpts) {
        log::debug!("EditOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());

        let mut table = vec![];

        let color = &opts.color.as_ref().map(|c| parse_color(&c)).transpose();

        /// Macro to update the `Tag`s color
        macro_rules! update_color {
            ($tag:expr, $color:expr) => {
                let old_tag = self.oregistry.get_tag($tag).cloned();
                if self.oregistry.update_tag_color($tag, $color) {
                    if let Some(ref old_tag) = old_tag {
                        let new_tag = self.oregistry.get_tag($tag);
                        table.push(vec![
                            fmt_tag(old_tag).to_string().cell().justify(Justify::Right),
                            "==>".cell().justify(Justify::Center),
                            fmt_tag(new_tag.unwrap())
                                .to_string()
                                .cell()
                                .justify(Justify::Left),
                        ]);
                    }
                }
            };
        }

        let old_tag = self.oregistry.get_tag(&opts.tag).cloned();

        if let Some(rename) = &opts.rename {
            if self.oregistry.update_tag_name(&opts.tag, rename) {
                if let Some(ref old_tag) = old_tag {
                    let new_tag = self.oregistry.get_tag(&rename);
                    table.push(vec![
                        fmt_tag(old_tag).to_string().cell().justify(Justify::Right),
                        "==>".cell().justify(Justify::Center),
                        fmt_tag(new_tag.expect("failed to get `new_tag`"))
                            .to_string()
                            .cell()
                            .justify(Justify::Left),
                    ]);
                }
            }
            if let Ok(Some(col)) = color {
                update_color!(rename, *col);
            }
        } else if let Ok(Some(col)) = color {
            update_color!(&opts.tag, *col);
        }

        if !self.quiet {
            print_stdout(
                table
                    .table()
                    .border(Border::builder().build())
                    .separator(Separator::builder().build()),
            )
            .expect("unable to print table");
        }

        log::debug!("Saving registry...");
        self.save_registry();
    }
}
