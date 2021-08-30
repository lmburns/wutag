use super::{
    uses::{wutag_error, Colorize},
    App,
};

impl App {
    pub(crate) fn clean_cache(&mut self) {
        self.registry.clear();
        if let Err(e) = self.registry.save() {
            wutag_error!("{:?}", e);
        } else {
            println!(
                "{} {}: {}",
                "\u{2714}".green().bold(),
                "CACHE CLEARED".red().bold(),
                self.registry
                    .path
                    .clone()
                    .display()
                    .to_string()
                    .green()
                    .bold()
            );
        }
    }
}
