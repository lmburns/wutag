use super::{uses::*, App};

#[derive(Clap, Debug, Clone)]
pub struct EditOpts {
    #[clap(
        name = "color",
        long, short,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
    )]
    /// Set the color of the tag to the specified color. Accepted values are hex
    /// colors like '0x000000' or '#1F1F1F' or just plain 'ff000a'. The
    /// colors are case insensitive meaning '1f1f1f' is equivalent to
    /// '1F1F1F'.
    pub color: String,
    /// The tag to edit
    #[clap(name = "tag", long, short = 't')]
    pub tag:   String,
}

impl App {
    pub(crate) fn edit(&mut self, opts: &EditOpts) {
        let color = match parse_color(&opts.color) {
            Ok(color) => color,
            Err(e) => {
                eprintln!("{}", fmt_err(e));
                return;
            },
        };
        let old_tag = self.registry.get_tag(&opts.tag).cloned();
        if self.registry.update_tag_color(&opts.tag, color) {
            if let Some(old_tag) = old_tag {
                let new_tag = self.registry.get_tag(&opts.tag);
                println!("{} ==> {}", fmt_tag(&old_tag), fmt_tag(new_tag.unwrap()))
            }
        }

        log::debug!("Saving registry...");
        self.save_registry();
    }
}
