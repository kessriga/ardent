use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use ardent_core::layout;
use console::{Key, Term};

pub(super) struct ReleasePlugin {
    pub(super) name: &'static str,
    required_files: &'static [&'static str],
}

const RELEASE: &[ReleasePlugin] = &[
    ReleasePlugin {
        name: "markdown",
        required_files: &["plugin.yaml", "format.wasm"],
    },
    ReleasePlugin {
        name: "rust",
        required_files: &["plugin.yaml", "format.wasm", "README.md"],
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

    let term = Term::stderr();
    if !io::stdin().is_terminal() || !term.is_term() {
        return Err(
            "pass --plugins (for example, --plugins markdown,rust) outside a terminal".into(),
        );
    }
    pick(term)
        .map_err(|err| {
            format!("cannot read plugin selection: {err}. Use --plugins to select by name")
        })?
        .ok_or_else(|| "plugin selection cancelled".into())
}

fn pick(mut term: Term) -> io::Result<Option<Vec<&'static ReleasePlugin>>> {
    term.write_line(
        "Format plugins (arrows to move, Space to select, Enter to confirm, q to cancel)",
    )?;
    let mut checked = [false; RELEASE.len()];
    let mut focus = 0;
    let confirmed = loop {
        show_choices(&mut term, &checked, focus)?;
        match term.read_key_raw()? {
            Key::ArrowUp | Key::ArrowLeft | Key::BackTab | Key::Char('k') => {
                focus = (focus + RELEASE.len() - 1) % RELEASE.len();
            }
            Key::ArrowDown | Key::ArrowRight | Key::Tab | Key::Char('j') => {
                focus = (focus + 1) % RELEASE.len();
            }
            Key::Char(' ') => checked[focus] = !checked[focus],
            Key::Char('a') => {
                let select_all = checked.contains(&false);
                checked.fill(select_all);
            }
            Key::Enter => break true,
            Key::Char('q') | Key::Escape | Key::CtrlC => break false,
            _ => {}
        }
    };
    term.write_line("")?;
    Ok(confirmed.then(|| {
        RELEASE
            .iter()
            .zip(checked)
            .filter_map(|(plugin, selected)| selected.then_some(plugin))
            .collect()
    }))
}

fn show_choices(term: &mut Term, checked: &[bool], focus: usize) -> io::Result<()> {
    let choices: Vec<_> = RELEASE
        .iter()
        .enumerate()
        .map(|(index, plugin)| {
            let pointer = if index == focus { '>' } else { ' ' };
            let mark = if checked[index] { 'x' } else { ' ' };
            format!("{pointer}[{mark}] {}", plugin.name)
        })
        .collect();
    let row = choices.join("  ");
    let width = usize::from(term.size().1);
    let visible = if row.len() < width {
        &row
    } else {
        &choices[focus]
    };
    if visible.len() >= width {
        return Err(io::Error::other(
            "terminal is too narrow for the plugin picker",
        ));
    }
    term.clear_line()?;
    write!(term, "{visible}")
}

pub(super) fn release_dir() -> Result<PathBuf, (PathBuf, io::Error)> {
    let executable =
        env::current_exe().map_err(|err| (PathBuf::from("current executable"), err))?;
    let dir = fs::canonicalize(&executable)
        .map_err(|err| (executable, err))?
        .with_file_name("plugins");
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
        let package = source.join(self.name);
        for file in self.required_files {
            let path = package.join(file);
            if !fs::symlink_metadata(&path)
                .map_err(|err| (path.clone(), err))?
                .is_file()
            {
                return Err(invalid_entry(&path, "expected a regular package file"));
            }
        }
        copy_package_files(&package, &Path::new("plugins").join(self.name), ardent_dir)
    }
}

fn copy_package_files(
    source: &Path,
    target: &Path,
    ardent_dir: &Path,
) -> Result<(), (PathBuf, io::Error)> {
    let entries = fs::read_dir(source).map_err(|err| (source.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| (source.to_path_buf(), err))?;
        let path = entry.path();
        let target = target.join(entry.file_name());
        let kind = entry.file_type().map_err(|err| (path.clone(), err))?;
        if kind.is_dir() {
            copy_package_files(&path, &target, ardent_dir)?;
        } else if kind.is_file() {
            let bytes = fs::read(&path).map_err(|err| (path, err))?;
            layout::write_atomic(ardent_dir, &target, &bytes)
                .map_err(|err| (ardent_dir.join(target), err))?;
        } else {
            return Err(invalid_entry(&path, "expected a regular file or directory"));
        }
    }
    Ok(())
}

fn invalid_entry(path: &Path, message: &'static str) -> (PathBuf, io::Error) {
    (
        path.to_path_buf(),
        io::Error::new(io::ErrorKind::InvalidData, message),
    )
}
