#[cfg(any(windows, test))]
use std::io::BufRead;
use std::io::{self, IsTerminal, Write};

#[cfg(windows)]
use winapi_util::file;

use console::{Key, Term};

use super::plugins::{RELEASE, ReleasePlugin};

#[cfg(unix)]
mod unix_keys;

pub(super) fn select(names: Option<&str>) -> Result<Vec<&'static ReleasePlugin>, String> {
    if let Some(names) = names {
        return parse_names(names);
    }
    let term = Term::stderr();
    if !io::stdin().is_terminal() || !term.is_term() {
        return Err(
            "pass --plugins (for example, --plugins markdown,rust) outside a terminal".into(),
        );
    }
    #[cfg(windows)]
    if file::typ(io::stdin())
        .map_err(|err| err.to_string())?
        .is_pipe()
    {
        return pick_by_name(io::stdin().lock(), io::stderr().lock());
    }
    #[cfg(unix)]
    let result = unix_keys::with_raw_input(|read_key| pick(term, read_key));
    #[cfg(not(unix))]
    let result = pick(term.clone(), || term.read_key_raw());
    result
        .map_err(|err| {
            format!("cannot read plugin selection: {err}. Use --plugins to select by name")
        })?
        .ok_or_else(|| "plugin selection cancelled".into())
}

fn parse_names(names: &str) -> Result<Vec<&'static ReleasePlugin>, String> {
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
                    available_names()
                )
            })?;
        if !selected.iter().any(|picked| picked.name == name) {
            selected.push(plugin);
        }
    }
    Ok(selected)
}

fn available_names() -> String {
    RELEASE
        .iter()
        .map(|plugin| plugin.name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(any(windows, test))]
fn pick_by_name(
    mut input: impl BufRead,
    mut output: impl Write,
) -> Result<Vec<&'static ReleasePlugin>, String> {
    writeln!(output, "Available format plugins: {}", available_names())
        .map_err(|err| err.to_string())?;
    write!(
        output,
        "Enter comma-separated names, leave blank for none, or enter q to cancel: "
    )
    .and_then(|()| output.flush())
    .map_err(|err| err.to_string())?;
    let mut line = String::new();
    let read = input.read_line(&mut line).map_err(|err| err.to_string())?;
    if read == 0 || matches!(line.trim(), "q" | "\u{3}") {
        return Err("plugin selection cancelled".into());
    }
    parse_names(line.trim())
}

fn pick(
    mut term: Term,
    mut read_key: impl FnMut() -> io::Result<Key>,
) -> io::Result<Option<Vec<&'static ReleasePlugin>>> {
    term.write_line(
        "Format plugins (arrows to move, Space to select, Enter to confirm, q to cancel)",
    )?;
    let mut checked = [false; RELEASE.len()];
    let mut focus = 0;
    let confirmed = loop {
        show_choices(&mut term, &checked, focus)?;
        let key = read_key()?;
        match key {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_prompt_handles_windows_line_endings_and_cancellation() {
        for (input, expected) in [
            ("rust,markdown\r\n", Some(vec!["rust", "markdown"])),
            ("\r\n", Some(vec![])),
            ("q\r\n", None),
            ("", None),
            ("html\r\n", None),
        ] {
            let mut prompt = Vec::new();
            let selection = pick_by_name(input.as_bytes(), &mut prompt);
            assert_eq!(
                selection
                    .ok()
                    .map(|plugins| plugins.iter().map(|p| p.name).collect::<Vec<_>>()),
                expected
            );
            assert!(String::from_utf8_lossy(&prompt).contains("markdown, rust"));
        }
    }
}
