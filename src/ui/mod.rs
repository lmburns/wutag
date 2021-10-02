#![allow(unused)]
pub(crate) mod event;
pub(crate) mod style;
pub(crate) mod table;
pub(crate) mod ui_app;

pub(crate) use event::{Event, EventConfig, Events};
pub(crate) use ui_app::AppMode;

use crate::{config::Config, registry::TagRegistry, subcommand::App};
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

/// Errors used within the UI module
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Failure to start UI
    #[error("failed to start UI: {0}")]
    UiStartFailure(#[source] anyhow::Error),
    /// Failure to receive next item from channel
    #[error("failed receive from the crossbeam_channel: {0}")]
    Recv(#[source] crossbeam_channel::RecvError),
    /// Failure updating UI
    #[error("failure updating UI: {0}")]
    Updating(#[source] anyhow::Error),
    /// Failure from input of UI
    #[error("failure handling UI input: {0}")]
    InputHandling(#[source] anyhow::Error),
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
    panic::set_hook(Box::new(|info| {
        destruct_terminal();
        better_panic::Settings::auto().create_panic_handler()(info);
    }));

    let mut app = ui_app::UiApp::new(config, registry).map_err(Error::UiStartFailure)?;
    let mut terminal = setup_terminal();
    app.render(cli_app, &mut terminal).unwrap();

    let events = Events::with_config(EventConfig {
        tick_rate: Duration::from_millis(app.config.ui.tick_rate),
    });

    loop {
        app.render(cli_app, &mut terminal).unwrap();
        match events.next().map_err(Error::Recv)? {
            Event::Input(input) => {
                if input == app.config.keys.edit && app.mode == AppMode::WutagList {
                    events.leave_tui_mode(&mut terminal);
                }

                let res = app.handle_input(input);

                if input == app.config.keys.edit && app.mode == AppMode::WutagList
                    || app.mode == AppMode::WutagError
                {
                    events.enter_tui_mode(&mut terminal);
                }

                if let Err(e) = res {
                    destruct_terminal();
                    return Err(Error::InputHandling(e));
                }
            },
            Event::Tick =>
                if let Err(e) = app.update(false) {
                    destruct_terminal();
                    return Err(Error::Updating(e));
                },
        }
        if app.should_quit {
            destruct_terminal();
            break;
        }
    }

    Ok(())
}
