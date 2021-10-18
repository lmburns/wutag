#![allow(unused)]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// use termion::{
//     color, event::Key, input::MouseTerminal, raw::IntoRawMode,
// screen::AlternateScreen, style, };

use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

use crossbeam_channel as channel;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_utils::thread;
use serde::{Deserialize, Serialize};
use tui::{backend::CrosstermBackend, Terminal};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) enum Key {
    Char(char),
    Alt(char),
    Ctrl(char),
    Backspace,
    CtrlBackspace,
    AltBackspace,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Insert,
    Delete,
    CtrlDelete,
    AltDelete,
    Null,
    Esc,
    F(u8),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EventConfig {
    pub(crate) tick_rate: Duration,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_micros(16666),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Event<I> {
    Input(I),
    Tick,
}

pub(crate) struct Events {
    pub(crate) rx: Receiver<Event<Key>>,
}

// let stdin = io::stdin();
// for key in stdin.keys() {
//     if let Ok(k) = key {
//         if tx.send(Event::Input(k)).is_err() {
//             return;
//         }
//     }
// }

impl Events {
    pub(crate) fn new() -> Events {
        Events::with_config(EventConfig::default())
    }

    pub(crate) fn with_config(config: EventConfig) -> Self {
        use crossterm::event::{KeyCode as Code, KeyModifiers as Modifier};
        let tick_rate = config.tick_rate;
        let key_input_disabled = Arc::new(AtomicBool::new(false));

        let (tx, rx) = channel::unbounded::<Event<Key>>();

        rayon::spawn(move || {
            let tx = tx.clone();
            let key_input_disabled = key_input_disabled.clone();

            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(tick_rate);
                if key_input_disabled.load(Ordering::Relaxed) {
                    std::thread::sleep(timeout);
                    continue;
                } else if event::poll(timeout).expect("no events available in thread") {
                    #[allow(clippy::collapsible_match)]
                    match event::read() {
                        Ok(event) => match event {
                            event::Event::Key(key) => {
                                let key = match key.code {
                                    Code::Char(c) => match key.modifiers {
                                        Modifier::NONE | Modifier::SHIFT => Key::Char(c),
                                        Modifier::ALT => Key::Alt(c),
                                        Modifier::CONTROL => Key::Ctrl(c),
                                        _ => Key::Null,
                                    },
                                    Code::Backspace => match key.modifiers {
                                        Modifier::ALT => Key::AltBackspace,
                                        Modifier::CONTROL => Key::CtrlBackspace,
                                        _ => Key::Backspace,
                                    },
                                    Code::Delete => match key.modifiers {
                                        Modifier::ALT => Key::AltDelete,
                                        Modifier::CONTROL => Key::CtrlDelete,
                                        _ => Key::Delete,
                                    },
                                    Code::Tab => Key::Tab,
                                    Code::BackTab => Key::BackTab,
                                    Code::Left => Key::Left,
                                    Code::Right => Key::Right,
                                    Code::Up => Key::Up,
                                    Code::Down => Key::Down,
                                    Code::Home => Key::Home,
                                    Code::End => Key::End,
                                    Code::PageUp => Key::PageUp,
                                    Code::PageDown => Key::PageDown,
                                    Code::Insert => Key::Insert,
                                    Code::Esc => Key::Esc,
                                    Code::F(k) => Key::F(k),
                                    Code::Null => Key::Null,
                                    Code::Enter => Key::Char('\n'),
                                };
                                tx.send(Event::Input(key))
                                    .expect("failed to send key event");
                                std::thread::sleep(Duration::from_millis(1));
                            },
                            // event::Event::Mouse(mouse) => {},
                            // event::Event::Resize(w, h) => {},
                            _ => {
                                tx.send(Event::Tick).expect("failed to send tick event");
                            },
                        },
                        _ => {
                            tx.send(Event::Tick).expect("failed to send tick event");
                        },
                    }
                }
                if last_tick.elapsed() >= tick_rate {
                    tx.send(Event::Tick).expect("failed to send tick event");
                    last_tick = Instant::now();
                }
            }
        });
        Events { rx }
    }

    /// Get next item from the thread
    pub(crate) fn next(&self) -> Result<Event<Key>, channel::RecvError> {
        self.rx.recv()
    }

    /// Leave TUI mode
    #[allow(clippy::unused_self)]
    pub(crate) fn leave_tui_mode(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        disable_raw_mode().unwrap();
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        terminal.show_cursor().unwrap();
    }

    /// Enter TUI mode
    #[allow(clippy::unused_self)]
    pub(crate) fn enter_tui_mode(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        enable_raw_mode().unwrap();
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        terminal.resize(terminal.size().unwrap()).unwrap();
    }

    /// Enable mouse capture
    #[allow(clippy::unused_self)]
    pub(crate) fn enable_mouse_capture(&self) {
        execute!(io::stdout(), EnableMouseCapture).expect("unable to enable mouse capturing");
    }

    /// Disables mouse capture
    #[allow(clippy::unused_self)]
    pub(crate) fn disable_mouse_capture(&self) {
        execute!(io::stdout(), DisableMouseCapture).expect("unable to disable mouse capturing");
    }
}
