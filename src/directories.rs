//! Retrieve all relevant directories to this project

use directories::{BaseDirs, ProjectDirs, UserDirs};
use once_cell::sync::Lazy;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Get the base [`WutagDirs`]
pub(crate) static PROJECT_DIRS: Lazy<WutagDirs> =
    Lazy::new(|| WutagDirs::new().expect("failed to get `WutagDirs`"));

/// Get the project directories relevant to [`wutag`]. This treats `macOS`
/// directories in the same way that `Linux` is treated. That is, all `XDG`
/// directories will be the same.
///
/// More information on directory conversion usage can be found in
/// [`build_alias_hash`] and [`alias_replace`]
///
/// [`wutag`]: crate
/// [`build_alias_hash`]: ./config/struct.UiConfig.html#method.build_alias_hash
/// [`alias_replace`]: ./ui/ui_app/struct.UiApp.html#method.alias_replace
#[derive(Debug, Clone)]
pub(crate) struct WutagDirs {
    // === Main project directories ===
    /// User's `$HOME` directory
    home_dir:   PathBuf,
    /// User's `$XDG_CACHE_HOME/wutag` directory
    #[allow(dead_code)]
    cache_dir:  PathBuf,
    /// User's `$XDG_CONFIG_HOME/wutag` directory
    config_dir: PathBuf,
    /// User's `$XDG_DATA_HOME/wutag` directory
    data_dir:   PathBuf,

    // === Directories used for `alias_hash` ===
    /// User's `$XDG_MUSIC_DIR` directory
    hash_audio_dir:      PathBuf,
    /// User's `$XDG_CACHE_HOME` directory
    hash_cache_dir:      PathBuf,
    /// User's `$XDG_CONFIG_HOME` directory
    hash_config_dir:     PathBuf,
    /// User's `$XDG_DATA_HOME` directory
    hash_data_dir:       PathBuf,
    /// User's `$XDG_DESKTOP_DIR` directory
    hash_desktop_dir:    PathBuf,
    /// User's `$XDG_DOCUMENTS_DIR` directory
    hash_document_dir:   PathBuf,
    /// User's `$XDG_DOWNLOAD_DIR` directory
    hash_download_dir:   PathBuf,
    /// User's `$XDG_BIN_HOME` directory
    hash_executable_dir: PathBuf,
    /// User's `$XDG_FONTS_HOME` directory
    hash_font_dir:       PathBuf,
    /// User's `$XDG_PICTURES_DIR` directory
    hash_picture_dir:    PathBuf,
    /// User's `$XDG_PUBLICSHARE_DIR` directory
    hash_public_dir:     PathBuf,
    /// User's `$XDG_TEMPLATES_DIR` directory
    hash_template_dir:   PathBuf,
    /// User's `$XDG_VIDEOS_DIR` directory
    hash_video_dir:      PathBuf,
}

impl WutagDirs {
    /// Create a new [`WutagDirs`]
    fn new() -> Option<Self> {
        Some(Self {
            home_dir:   Self::get_home_dir()?,
            cache_dir:  Self::get_cache_dir()?,
            config_dir: Self::get_config_dir()?,
            data_dir:   Self::get_data_dir()?,

            hash_audio_dir:      Self::get_hash_audio_dir()?,
            hash_cache_dir:      Self::get_hash_cache_dir()?,
            hash_config_dir:     Self::get_hash_config_dir()?,
            hash_data_dir:       Self::get_hash_data_dir()?,
            hash_desktop_dir:    Self::get_hash_desktop_dir()?,
            hash_document_dir:   Self::get_hash_document_dir()?,
            hash_download_dir:   Self::get_hash_download_dir()?,
            hash_executable_dir: Self::get_hash_executable_dir()?,
            hash_font_dir:       Self::get_hash_font_dir()?,
            hash_picture_dir:    Self::get_hash_picture_dir()?,
            hash_public_dir:     Self::get_hash_public_dir()?,
            hash_template_dir:   Self::get_hash_template_dir()?,
            hash_video_dir:      Self::get_hash_video_dir()?,
        })
    }

    /// Wrapper function that makes it easier to get directories
    #[allow(clippy::unwrap_used, clippy::unnecessary_unwrap)]
    fn get_dir(wtag_var: &str, var: &str, join: Option<&str>, dirf: PathBuf) -> Option<PathBuf> {
        // if let Some(dir) = env::var_os(wtag_var).map(PathBuf::from) {
        //     Some(dir)
        // } else if cfg!(target_os = "macos") && join.is_some() {
        //     env::var_os(var)
        //         .map(PathBuf::from)
        //         .filter(|p| p.is_absolute())
        //         .or_else(|| BaseDirs::new().map(|p|
        // p.home_dir().join(join.unwrap())))         .map(|p|
        // p.join(env!("CARGO_PKG_NAME"))) } else {
        //     Some(dirf)
        // }

        env::var_os(wtag_var).map(PathBuf::from).map_or_else(
            || {
                if cfg!(target_os = "macos") && join.is_some() {
                    env::var_os(var)
                        .map(PathBuf::from)
                        .filter(|p| p.is_absolute())
                        .or_else(|| BaseDirs::new().map(|p| p.home_dir().join(join.unwrap())))
                        .map(|p| p.join(env!("CARGO_PKG_NAME")))
                } else {
                    Some(dirf)
                }
            },
            Some,
        )
    }

    /// Get the `home` directory
    fn get_home_dir() -> Option<PathBuf> {
        BaseDirs::new().map(|p| p.home_dir().to_path_buf())
    }

    // ================== Config Dirs =====================

    /// Get the `cache` directory
    fn get_cache_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_CACHE_DIR",
            "XDG_CACHE_HOME",
            Some(".cache"),
            get_project_dirs().cache_dir().to_path_buf(),
        )
    }

    /// Get the `config` directory
    fn get_config_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            Some(".config"),
            get_project_dirs().config_dir().to_path_buf(),
        )
    }

    /// Get the `data` directory
    fn get_data_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_DATA_DIR",
            "XDG_DATA_HOME",
            Some(".local/share"),
            get_project_dirs().data_dir().to_path_buf(),
        )
    }

    // =================== Alias Hash =====================

    /// Get the alias hash `audio` directory
    fn get_hash_audio_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_AUDIO_DIR",
            "XDG_MUSIC_DIR",
            None,
            UserDirs::new().and_then(|p| p.audio_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `cache` directory
    fn get_hash_cache_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_ALIAS_CACHE_DIR",
            "XDG_CACHE_HOME",
            Some(".cache"),
            BaseDirs::new().map(|p| p.cache_dir().to_path_buf())?,
        )
    }

    /// Get the alias hash `config` directory
    fn get_hash_config_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_ALIAS_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            Some(".config"),
            BaseDirs::new().map(|p| p.config_dir().to_path_buf())?,
        )
    }

    /// Get the alias hash `data` directory
    fn get_hash_data_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_ALIAS_DATA_DIR",
            "XDG_DATA_HOME",
            Some(".local/share"),
            BaseDirs::new().map(|p| p.data_dir().to_path_buf())?,
        )
    }

    /// Get the alias hash `desktop` directory
    fn get_hash_desktop_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_DESKTOP_DIR",
            "XDG_DESKTOP_DIR",
            None,
            UserDirs::new().and_then(|p| p.desktop_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `document` directory
    fn get_hash_document_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_DOCUMENTS_DIR",
            "XDG_DOCUMENTS_DIR",
            None,
            UserDirs::new().and_then(|p| p.document_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `download` directory
    fn get_hash_download_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_DOWNLOAD_DIR",
            "XDG_DOWNLOAD_DIR",
            None,
            UserDirs::new().and_then(|p| p.download_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `executable` directory
    fn get_hash_executable_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_BIN_DIR",
            "XDG_BIN_HOME",
            Some(".local/bin"),
            BaseDirs::new().and_then(|p| p.executable_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `font` directory
    fn get_hash_font_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_FONTS_DIR",
            "XDG_FONTS_HOME",
            Some(".local/share/fonts"),
            UserDirs::new().and_then(|p| p.font_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `picture` directory
    fn get_hash_picture_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_PICTURES_DIR",
            "XDG_PICTURES_DIR",
            None,
            UserDirs::new().and_then(|p| p.picture_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `public` directory
    fn get_hash_public_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_PUBLIC_DIR",
            "XDG_PUBLICSHARE_DIR",
            None,
            UserDirs::new().and_then(|p| p.public_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `template` directory
    fn get_hash_template_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_TEMPLATE_DIR",
            "XDG_TEMPLATES_DIR",
            Some("Templates"),
            UserDirs::new().and_then(|p| p.template_dir().map(Path::to_path_buf))?,
        )
    }

    /// Get the alias hash `video` directory
    fn get_hash_video_dir() -> Option<PathBuf> {
        Self::get_dir(
            "WUTAG_VIDEOS_DIR",
            "XDG_VIDEOS_DIR",
            None,
            UserDirs::new().and_then(|p| p.video_dir().map(Path::to_path_buf))?,
        )
    }

    // -
    // ================== Public Funcs ====================
    // -

    // ================== Config Dirs =====================

    /// Get cache directory
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get configuration directory
    #[must_use]
    pub(crate) fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get local data directory
    #[must_use]
    pub(crate) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get cache directory
    #[must_use]
    pub(crate) fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    // =================== Alias Hash =====================

    /// Get audio directory
    #[must_use]
    pub(crate) fn hash_audio_dir(&self) -> &Path {
        &self.hash_audio_dir
    }

    /// Get cache directory
    #[must_use]
    pub(crate) fn hash_cache_dir(&self) -> &Path {
        &self.hash_cache_dir
    }

    /// Get config directory
    #[must_use]
    pub(crate) fn hash_config_dir(&self) -> &Path {
        &self.hash_config_dir
    }

    /// Get data directory
    #[must_use]
    pub(crate) fn hash_data_dir(&self) -> &Path {
        &self.hash_data_dir
    }

    /// Get desktop directory
    #[must_use]
    pub(crate) fn hash_desktop_dir(&self) -> &Path {
        &self.hash_desktop_dir
    }

    /// Get document directory
    #[must_use]
    pub(crate) fn hash_document_dir(&self) -> &Path {
        &self.hash_document_dir
    }

    /// Get download directory
    #[must_use]
    pub(crate) fn hash_download_dir(&self) -> &Path {
        &self.hash_download_dir
    }

    /// Get executable directory
    #[must_use]
    pub(crate) fn hash_executable_dir(&self) -> &Path {
        &self.hash_executable_dir
    }

    /// Get font directory
    #[must_use]
    pub(crate) fn hash_font_dir(&self) -> &Path {
        &self.hash_font_dir
    }

    /// Get picture directory
    #[must_use]
    pub(crate) fn hash_picture_dir(&self) -> &Path {
        &self.hash_picture_dir
    }

    /// Get public directory
    #[must_use]
    pub(crate) fn hash_public_dir(&self) -> &Path {
        &self.hash_public_dir
    }

    /// Get template directory
    #[must_use]
    pub(crate) fn hash_template_dir(&self) -> &Path {
        &self.hash_template_dir
    }

    /// Get video directory
    #[must_use]
    pub(crate) fn hash_video_dir(&self) -> &Path {
        &self.hash_video_dir
    }
}

/// Get all user project directories (not for `macOS`)
pub(crate) fn get_project_dirs() -> ProjectDirs {
    log::trace!("determining project default folders");
    ProjectDirs::from("com", "lmburns", "wutag")
        .expect("could not detect user home directory to place program files")
}
