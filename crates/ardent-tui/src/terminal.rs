//! Own the terminal modes for a browser session and dispatch input to its navigation state.

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::render::{self, Paging};
use crate::{Browser, Document, FileList};

/// Check that the browser can take input and draw on an interactive terminal.
/// Call this before starting project work for a terminal-only command.
///
/// # Errors
///
/// Returns an error if standard input or standard output is redirected.
pub fn require_terminal() -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "the browser requires an interactive terminal",
        ));
    }
    Ok(())
}

pub(super) fn run(
    browser: Browser,
    load: impl Fn(&str) -> io::Result<Document>,
    refresh: impl FnMut(Option<&str>) -> io::Result<Option<FileList>>,
) -> io::Result<()> {
    require_terminal()?;
    #[cfg(unix)]
    let mut signals = signal_hook::iterator::Signals::new({
        use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
        [SIGINT, SIGTERM, SIGHUP, SIGQUIT]
    })?;
    let mut output = io::stdout().lock();
    terminal::enable_raw_mode()?;
    let result: io::Result<Option<i32>> = (|| {
        execute!(output, EnterAlternateScreen, Clear(ClearType::All), Hide)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(&mut output))?;
        event_loop(&mut terminal, browser, load, refresh, || {
            #[cfg(unix)]
            {
                signals.pending().next()
            }
            #[cfg(not(unix))]
            {
                None
            }
        })
    })();
    // Attempt both restorations even if the screen write fails. Deliver a pending signal after cleanup.
    let screen = execute!(output, Show, LeaveAlternateScreen);
    let input = terminal::disable_raw_mode();
    screen?;
    input?;
    #[cfg(unix)]
    if let Some(signal) = result? {
        return signal_hook::low_level::emulate_default_handler(signal);
    }
    #[cfg(not(unix))]
    result?;
    Ok(())
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<impl Write>>,
    mut browser: Browser,
    load: impl Fn(&str) -> io::Result<Document>,
    mut refresh: impl FnMut(Option<&str>) -> io::Result<Option<FileList>>,
    mut interrupted: impl FnMut() -> Option<i32>,
) -> io::Result<Option<i32>> {
    let mut frame = paint(terminal, &browser)?;
    loop {
        if let Some(signal) = interrupted() {
            return Ok(Some(signal));
        }
        if let Some(files) = refresh(browser.open_path())? {
            browser.refresh(files, &load);
            frame = paint(terminal, &browser)?;
        }
        if !event::poll(Duration::from_millis(75))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return Ok(None::<i32>);
                }
                browser.key(key.code, frame.page_height, frame.more, &load);
                frame = paint(terminal, &browser)?;
            }
            Event::Resize(_, _) => {
                browser.resized();
                frame = paint(terminal, &browser)?;
            }
            _ => {}
        }
    }
}

fn paint(
    terminal: &mut Terminal<CrosstermBackend<impl Write>>,
    browser: &Browser,
) -> io::Result<Paging> {
    let mut paging = Paging::default();
    terminal.draw(|frame| paging = render::draw(frame, browser))?;
    Ok(paging)
}
