#![allow(unused)]
pub(crate) mod banner;
pub(crate) mod command;
pub(crate) mod completion;
pub(crate) mod event;
pub(crate) mod history;
pub(crate) mod keybindings;
pub(crate) mod list;
pub(crate) mod mtui;
pub(crate) mod table;
pub(crate) mod ui_app;

pub(crate) use event::{Event, EventConfig, EventHandler};
pub(crate) use ui_app::AppMode;

use crate::{config::Config, oregistry::TagRegistry, subcommand::App};
use anyhow::Result;
use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, panic, time::Duration};
use thiserror::Error;
use tui::{backend::CrosstermBackend, Terminal};

#[cfg(all(target_os = "linux", not(target_env = "musl")))]
use notify_rust::Hint;
#[cfg(not(target_env = "musl"))]
use notify_rust::Notification;

/// Errors used within the UI module
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Failure to start UI
    #[error("failed to start UI: {0}")]
    UiStartFailure(#[source] anyhow::Error),
    /// Failure to stop UI
    #[error("failed to stop UI: {0}")]
    UiStopFailure(#[source] anyhow::Error),
    /// Failure to render/draw UI
    #[error("failed to render UI: {0}")]
    UiRender(#[source] anyhow::Error),
    /// Failure to pause UI
    #[error("failed to pause UI: {0}")]
    UiPause(#[source] anyhow::Error),
    /// Failure to receive next item from channel
    #[error("failed receive from the crossbeam_channel: {0}")]
    Recv(#[source] crossbeam_channel::RecvError),
    /// Failure updating UI
    #[error("failure updating UI: {0}")]
    Updating(#[source] anyhow::Error),
    /// Failure from input of UI
    #[error("failure handling UI input: {0}")]
    InputHandling(#[source] anyhow::Error),
    /// Failure to setup terminal
    #[error("failure setting up terminal: {0}")]
    TerminalSetup(#[source] io::Error),
    /// Custom string as error
    #[error("{0}")]
    Custom(String),
}

/// Setup the `[Terminal]` `AlternateScreen` interface for the UI
pub(crate) fn setup_terminal() -> Terminal<CrosstermBackend<io::Stdout>> {
    terminal::enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    execute!(stdout, Clear(ClearType::All)).unwrap();
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).unwrap()
}

/// Return to the terminal screen prior to entering the UI
pub(crate) fn destruct_terminal() {
    terminal::disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}

/// Used with print statements for debugging purposes
pub(crate) fn dump_and_exit<F: FnOnce() + Send>(f: F) {
    destruct_terminal();
    f();
    std::process::exit(1);
}

/// Start the UI interface
pub(crate) fn start_ui(cli_app: &App, config: Config, registry: TagRegistry) -> Result<(), Error> {
    panic::set_hook(Box::new(|panic_info| {
        destruct_terminal();
        better_panic::Settings::auto().create_panic_handler()(panic_info);
    }));

    let mut app = ui_app::UiApp::new(config, registry).map_err(Error::UiStartFailure)?;
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend).map_err(Error::TerminalSetup)?;

    let events = EventHandler::new(EventConfig::new(app.config.ui.tick_rate));
    let mut tui = mtui::Tui::new(terminal, events);
    tui.enter_tui_mode().map_err(Error::UiStartFailure)?;

    let mut toggle_pause = false;
    loop {
        tui.render(cli_app, &mut app).map_err(Error::UiRender)?;
        match tui.events.next().map_err(Error::Recv)? {
            Event::Input(input) => {
                if input == app.config.keys.view && app.mode == AppMode::List {
                    // tui.leave_tui_mode().map_err(Error::UiStopFailure)?;
                    tui.toggle_pause().map_err(Error::UiPause)?;
                    toggle_pause = true;
                }

                let res = app.handle_input(input);

                if (input == app.config.keys.view && app.mode == AppMode::List) {
                    // tui.enter_tui_mode().map_err(Error::UiStartFailure)?;
                    tui.toggle_pause().map_err(Error::UiPause)?;
                    toggle_pause = false;
                }

                if toggle_pause {
                    tui.toggle_pause().map_err(Error::UiPause)?;
                }

                if let Err(e) = res {
                    tui.leave_tui_mode();
                    return Err(Error::InputHandling(e));
                }
            },
            Event::Tick =>
                if let Err(e) = app.update(false) {
                    tui.leave_tui_mode().map_err(Error::UiStopFailure)?;
                    return Err(Error::Updating(e));
                },
        }

        if app.should_quit {
            tui.leave_tui_mode().map_err(Error::UiStopFailure)?;
            break;
        }
    }

    Ok(())
}

// XXX: Breaks .as_ref() in opts
/// Show notification to let me know that what I was trying to do worked
pub(crate) fn notify<P: AsRef<str>>(sum: P, body: Option<P>) -> Result<()> {
    // Will segfault otherwise
    #[cfg(not(target_env = "musl"))]
    {
        let mut n = Notification::new();
        n.appname("wutag")
            .summary(sum.as_ref())
            .auto_icon()
            .icon("terminal")
            .timeout(3000);

        if let Some(b) = body {
            n.body(b.as_ref());
        }

        #[cfg(target_os = "linux")]
        n.urgency(notify_rust::Urgency::Low)
            .hint(Hint::Category("presence.offline".into()));

        n.show()?;

        Ok(())
    }
}
