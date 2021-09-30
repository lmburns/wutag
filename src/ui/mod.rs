pub(crate) mod event;
pub(crate) mod table;
pub(crate) mod ui_app;

pub(crate) use event::{EventConfig, Events};
pub(crate) use ui_app::AppMode;

use crate::{config::Config, registry::TagRegistry};
use anyhow::Result;
use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};
use thiserror::Error;
use tui::{backend::CrosstermBackend, Terminal};

/// Errors used within the UI module
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Failure to start UI
    #[error("failed to start UI: {0}")]
    UiStartFailure(#[source] anyhow::Error),
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

pub(crate) fn start_ui(config: Config, registry: TagRegistry) -> Result<(), Error> {
    let uiapp = ui_app::UiApp::new(config, registry);

    if let Err(e) = uiapp {
        destruct_terminal();
        return Err(Error::UiStartFailure(e));
    }

    let mut app = uiapp.unwrap();
    let mut terminal = setup_terminal();
    app.render(&mut terminal).unwrap();

    let events = Events::with_config(EventConfig {
        tick_rate: Duration::from_millis(app.config.ui.tick_rate),
    });

    loop {
        app.render(&mut terminal).unwrap();
        match events.next()? {
            Event::Input(input) => {
                if input == app.config.keys.edit && AppMode::WutagList {
                    events.leave_tui_mode(&mut terminal);
                }

                let res = app.handle_input(input);
            },
            Event::Tick => {
                let res = app.update(false);
                if res.is_err() {
                    destruct_terminal();
                    return res;
                }
            },
        }
    }

    Ok(())
}
