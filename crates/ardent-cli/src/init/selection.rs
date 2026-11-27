use std::io::{self, IsTerminal};
#[cfg(any(windows, test))]
use std::io::{BufRead, Write};

#[cfg(windows)]
use winapi_util::file;

use console::{Key, Term};

use crate::release::{RELEASE, ReleasePlugin};

#[cfg(unix)]
mod unix_keys;

const OPT_IN: &str = "Install format plugins? [Y/n]";
const LOCATION: &str = "Where should Ardent install them?";
const SCOPES: [&str; 2] = ["For my user account", "In this project"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Scope {
    User,
    Project,
}

pub(super) struct Selection {
    pub(super) scope: Scope,
    pub(super) plugins: Vec<&'static ReleasePlugin>,
}

impl Selection {
    fn empty() -> Self {
        Self {
            scope: Scope::Project,
            plugins: Vec::new(),
        }
    }
}

pub(super) fn select(names: Option<&str>) -> Result<Selection, String> {
    if let Some(names) = names {
        return Ok(Selection {
            scope: Scope::Project,
            plugins: parse_names(names)?,
        });
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
fn pick_by_name(mut input: impl BufRead, mut output: impl Write) -> Result<Selection, String> {
    let mut ask = |question: &str| -> Result<String, String> {
        write!(output, "{question} ")
            .and_then(|()| output.flush())
            .map_err(|err| err.to_string())?;
        let mut line = String::new();
        let read = input.read_line(&mut line).map_err(|err| err.to_string())?;
        if read == 0 || matches!(line.trim(), "q" | "\u{3}" | "\u{1b}") {
            return Err("plugin selection cancelled".into());
        }
        Ok(line.trim().to_owned())
    };
    loop {
        match ask(OPT_IN)?.to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => break,
            "n" | "no" => return Ok(Selection::empty()),
            _ => continue,
        }
    }
    let question = format!(
        "{LOCATION}\n  1. {} (default)\n  2. {}\nEnter 1 or 2:",
        SCOPES[0], SCOPES[1]
    );
    let scope = loop {
        match ask(&question)?.as_str() {
            "" | "1" => break Scope::User,
            "2" => break Scope::Project,
            _ => continue,
        }
    };
    let names = ask(&format!(
        "Select format plugins ({})\nEnter comma-separated names, leave blank for none, or enter q to cancel:",
        available_names()
    ))?;
    Ok(Selection {
        scope,
        plugins: parse_names(&names)?,
    })
}

fn pick(
    mut term: Term,
    mut read_key: impl FnMut() -> io::Result<Key>,
) -> io::Result<Option<Selection>> {
    term.write_line(OPT_IN)?;
    loop {
        match read_key()? {
            Key::Enter | Key::Char('y' | 'Y') => break,
            Key::Char('n' | 'N') => return Ok(Some(Selection::empty())),
            Key::Char('q') | Key::Escape | Key::CtrlC => return Ok(None),
            _ => {}
        }
    }
    term.write_line(LOCATION)?;
    term.write_line("Use arrows to move, Enter to confirm, q to cancel.")?;
    let scopes = if usize::from(term.size().1) > SCOPES[0].len() + 2 {
        SCOPES
    } else {
        ["Per-user", "Per-project"]
    };
    let Some(index) = menu(&mut term, &mut read_key, &scopes, None)? else {
        return Ok(None);
    };
    let scope = if index == 0 {
        Scope::User
    } else {
        Scope::Project
    };
    term.write_line(
        "Select format plugins (arrows to move, Space to toggle, Enter to confirm, q to cancel)",
    )?;
    let names: Vec<_> = RELEASE.iter().map(|plugin| plugin.name).collect();
    let mut checked = [false; RELEASE.len()];
    if menu(&mut term, &mut read_key, &names, Some(&mut checked))?.is_none() {
        return Ok(None);
    }
    let plugins = RELEASE
        .iter()
        .zip(checked)
        .filter_map(|(plugin, checked)| checked.then_some(plugin))
        .collect();
    Ok(Some(Selection { scope, plugins }))
}

fn menu(
    term: &mut Term,
    read_key: &mut impl FnMut() -> io::Result<Key>,
    names: &[&str],
    mut checked: Option<&mut [bool]>,
) -> io::Result<Option<usize>> {
    let mut focus = 0;
    loop {
        let lines = draw_menu(term, names, focus, checked.as_deref())?;
        match read_key()? {
            Key::ArrowUp | Key::ArrowLeft | Key::BackTab | Key::Char('k') => {
                focus = (focus + names.len() - 1) % names.len()
            }
            Key::ArrowDown | Key::ArrowRight | Key::Tab | Key::Char('j') => {
                focus = (focus + 1) % names.len()
            }
            Key::Char(' ') if let Some(items) = &mut checked => items[focus] = !items[focus],
            Key::Char('a') if let Some(items) = &mut checked => {
                let all = items.contains(&false);
                items.fill(all);
            }
            Key::Enter => {
                term.write_line("")?;
                return Ok(Some(focus));
            }
            Key::Char('q') | Key::Escape | Key::CtrlC => {
                term.write_line("")?;
                return Ok(None);
            }
            _ => {}
        }
        term.clear_line()?;
        term.clear_last_lines(lines - 1)?;
    }
}

fn draw_menu(
    term: &Term,
    names: &[&str],
    focus: usize,
    checked: Option<&[bool]>,
) -> io::Result<usize> {
    let (height, width) = term.size();
    let stacked = usize::from(height) >= names.len();
    let mut lines = 0;
    for (index, name) in names.iter().enumerate() {
        if !stacked && index != focus {
            continue;
        }
        let pointer = if index == focus { '>' } else { ' ' };
        let mark = match &checked {
            Some(items) if items[index] => "[x] ",
            Some(_) => "[ ] ",
            None => "",
        };
        let separator = if mark.len() + name.len() + 2 >= usize::from(width) {
            ""
        } else {
            " "
        };
        let row = format!("{pointer}{separator}{mark}{name}");
        if row.len() >= usize::from(width) {
            return Err(io::Error::other(
                "terminal is too narrow for the plugin picker",
            ));
        }
        if lines > 0 {
            term.write_line("")?;
        }
        term.write_str(&row)?;
        lines += 1;
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_prompt_routes_scopes_and_skips_followups_when_declined() {
        for (input, expected, stages) in [
            ("\r\n\r\n\r\n", Some((Scope::User, vec![])), 3),
            (
                "Y\r\n2\r\nrust,markdown\r\n",
                Some((Scope::Project, vec!["rust", "markdown"])),
                3,
            ),
            ("n\r\n", Some((Scope::Project, vec![])), 1),
            ("q\r\n", None, 1),
            ("\r\nq\r\n", None, 2),
            ("\r\n\r\nq\r\n", None, 3),
            ("\r\n\r\nhtml\r\n", None, 3),
        ] {
            let mut prompt = Vec::new();
            let selection = pick_by_name(input.as_bytes(), &mut prompt);
            assert_eq!(
                selection.ok().map(|s| (
                    s.scope,
                    s.plugins.iter().map(|p| p.name).collect::<Vec<_>>()
                )),
                expected
            );
            let output = String::from_utf8_lossy(&prompt);
            assert!(output.contains(OPT_IN));
            assert_eq!(output.contains(LOCATION), stages >= 2);
            assert_eq!(output.contains("markdown, rust"), stages == 3);
        }
    }
}
