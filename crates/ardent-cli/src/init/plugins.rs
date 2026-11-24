use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use ardent_core::layout;
use dialoguer::MultiSelect;

pub(super) struct ReleasePlugin {
    pub(super) name: &'static str,
    files: &'static [&'static str],
}

const RELEASE: &[ReleasePlugin] = &[
    ReleasePlugin {
        name: "markdown",
        files: &["plugin.yaml", "format.wasm"],
    },
    ReleasePlugin {
        name: "rust",
        files: &["plugin.yaml", "format.wasm", "README.md"],
    },
];

pub(super) fn select(names: Option<&str>) -> Result<Vec<&'static ReleasePlugin>, String> {
    let available: Vec<_> = RELEASE.iter().map(|plugin| plugin.name).collect();
    if let Some(names) = names {
        let mut selected: Vec<&ReleasePlugin> = Vec::new();
        if names.is_empty() {
            return Ok(selected);
        }
        for name in names.split(',').map(str::trim) {
            let plugin = RELEASE
                .iter()
                .find(|plugin| plugin.name == name)
                .ok_or_else(|| {
                    format!(
                        "unknown format plugin {name:?}. Available plugins: {}",
                        available.join(", ")
                    )
                })?;
            if !selected.iter().any(|picked| picked.name == name) {
                selected.push(plugin);
            }
        }
        return Ok(selected);
    }

    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(
            "pass --plugins (for example, --plugins markdown,rust) outside a terminal".into(),
        );
    }
    let indices = MultiSelect::new()
        .with_prompt("Format plugins (↑/↓ to move, Space to select, Enter to confirm, q to cancel)")
        .items(&available)
        .interact_opt()
        .map_err(|err| {
            format!("cannot read plugin selection: {err}. Use --plugins to select by name")
        })?
        .ok_or("plugin selection cancelled")?;
    Ok(indices.into_iter().map(|index| &RELEASE[index]).collect())
}

pub(super) fn release_dir() -> Result<PathBuf, (PathBuf, io::Error)> {
    let executable =
        env::current_exe().map_err(|err| (PathBuf::from("current executable"), err))?;
    let dir = executable.with_file_name("plugins");
    #[cfg(debug_assertions)]
    if !dir.try_exists().map_err(|err| (dir.clone(), err))? {
        return Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins"));
    }
    Ok(dir)
}

impl ReleasePlugin {
    pub(super) fn install(
        &self,
        source: &Path,
        ardent_dir: &Path,
    ) -> Result<(), (PathBuf, io::Error)> {
        for file in self.files {
            let path = source.join(self.name).join(file);
            let bytes = fs::read(&path).map_err(|err| (path, err))?;
            let target = format!("plugins/{}/{file}", self.name);
            layout::write_atomic(ardent_dir, &target, &bytes)
                .map_err(|err| (ardent_dir.join(target), err))?;
        }
        Ok(())
    }
}
