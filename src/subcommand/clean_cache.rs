use super::{uses::*, App};

impl App {
    pub(crate) fn clean_cache(&mut self) {
        self.registry.clear();
        if let Err(e) = self.registry.save() {
            wutag_error!("{:?}", e);
        } else {
            println!(
                "{} {}: {}",
                "✔".green().bold(),
                "CACHE CLEARED".red().bold(),
                self.registry
                    .path
                    .to_path_buf()
                    .display()
                    .to_string()
                    .green()
                    .bold()
            );
        }
    }
}
