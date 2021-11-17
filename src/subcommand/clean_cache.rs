use super::{
    uses::{io, wutag_error, wutag_fatal, wutag_info, BufRead, BufReader, Colorize, Write},
    App,
};

impl App {
    pub(crate) fn clean_cache(&mut self) {
        macro_rules! prompt {
            ($dis:ident, $path:expr) => {
                $dis!(
                    "Are you sure you want to clean the registry?\n\t- {} [{}/{}] ",
                    $path.display().to_string().yellow().bold(),
                    "y".green().bold(),
                    "N".red().bold()
                )
            };
        }

        let prompt = {
            prompt!(print, self.registry.path);

            if io::stdout().flush().is_err() {
                prompt!(println, self.registry.path);
            }

            let mut input = String::new();
            let mut stdin = BufReader::new(io::stdin());

            if let Err(e) = stdin.read_line(&mut input) {
                wutag_fatal!("{}", e);
            }

            // stdin
            //     .bytes()
            //     .next()
            //     .and_then(Result::ok)
            //     .map(char::from)
            //     .map_or(false, |c| {
            //          (c == 'y' || c == 'Y')
            //     })

            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "ye" | "1" => true,
                "n" | "0" => false,
                s => s.starts_with("yes") || s.starts_with("true"),
            }
        };

        if prompt {
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
        } else {
            wutag_info!(
                "{} was not cleared",
                self.registry.path.display().to_string().yellow().bold()
            );
        }
    }
}
