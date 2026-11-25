//! Own the terminal modes for a browser session and dispatch input to its navigation state.

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use crate::render::Frame;
use crate::{Browser, Document, FileList, Screen};

pub(super) fn run(
    browser: Browser,
    load: impl Fn(&str) -> io::Result<Document>,
    refresh: impl FnMut() -> io::Result<Option<FileList>>,
) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "ardent browse requires an interactive terminal",
        ));
    }
    #[cfg(unix)]
    let mut signals = signal_hook::iterator::Signals::new({
        use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
        [SIGINT, SIGTERM, SIGHUP, SIGQUIT]
    })?;
    let mut output = io::stdout().lock();
    terminal::enable_raw_mode()?;
    let result: io::Result<Option<i32>> = (|| {
        execute!(output, EnterAlternateScreen, Hide)?;
        event_loop(&mut output, browser, load, refresh, || {
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
    output: &mut impl Write,
    mut browser: Browser,
    load: impl Fn(&str) -> io::Result<Document>,
    mut refresh: impl FnMut() -> io::Result<Option<FileList>>,
    mut interrupted: impl FnMut() -> Option<i32>,
) -> io::Result<Option<i32>> {
    let mut frame = paint(output, &browser)?;
    loop {
        if let Some(signal) = interrupted() {
            return Ok(Some(signal));
        }
        if let Some(files) = refresh()? {
            browser.refresh(files, &load);
            frame = paint(output, &browser)?;
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
                browser.key(key.code, frame.preview_height, frame.more, &load);
                frame = paint(output, &browser)?;
            }
            Event::Resize(_, _) => {
                if let Screen::Outline(page)
                | Screen::Unavailable {
                    previous: Some(page),
                    ..
                } = &mut browser.screen
                {
                    page.scroll = 0;
                }
                frame = paint(output, &browser)?;
            }
            _ => {}
        }
    }
}

fn paint(output: &mut impl Write, browser: &Browser) -> io::Result<Frame> {
    let (width, height) = terminal::size()?;
    let frame = Frame::draw(browser, width.into(), height.into());
    queue!(output, Clear(ClearType::All))?;
    for (i, row) in frame
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| !row.is_empty())
    {
        queue!(output, MoveTo(0, i as u16), Print(row))?;
    }
    output.flush()?;
    Ok(frame)
}
