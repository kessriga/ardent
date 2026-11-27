use std::env;
use std::path::PathBuf;

pub(crate) fn home(variable: &str, default: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| {
            env::home_dir()
                .filter(|dir| dir.is_absolute())
                .map(|home| home.join(default))
        })
}

pub(crate) fn config_home() -> Option<PathBuf> {
    home("XDG_CONFIG_HOME", ".config")
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    config_home().map(|home| home.join("ardent"))
}
