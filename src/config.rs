use crate::util::macos_dirs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

const CONFIG_FILE: &str = "wutag.yml";

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Config {
    pub(crate) max_depth:  Option<usize>,
    pub(crate) base_color: Option<String>,
    pub(crate) colors:     Option<Vec<String>>,
}

impl Config {
    /// Loads Config from provided `path` by appending
    /// [CONFIG_FILE](CONFIG_FILE) name to it and reading the file.
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            fs::create_dir_all(path).context("unable to create config directory")?;
        }

        let path = path.join(CONFIG_FILE);
        if !path.is_file() {
            let initialization = "---\nmax_depth: 2\n...";

            let mut config_file: fs::File = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .with_context(|| format!("could not create wutag config: '{}'", path.display()))?;

            config_file
                .write_all(initialization.as_bytes())
                .with_context(|| format!("could not create wutag config: '{}'", path.display()))?;
            config_file.flush()?;
        }

        serde_yaml::from_slice(&fs::read(path).context("failed to read config file")?)
            .context("failed to deserialize config file")
    }

    /// Loads config file from home directory of user executing the program
    pub(crate) fn load_default_location() -> Result<Self> {
        Self::load(
            macos_dirs(dirs::config_dir(), ".config")
                .context("configuration directory not found")?
                .join("wutag"),
        )
    }
}
