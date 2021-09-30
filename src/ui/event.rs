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

use serde::{Deserialize, Serialize};
use tui::{backend::CrosstermBackend, Terminal};

use crossbeam_channel as channel;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_utils::thread;

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
        use crossterm::event::{KeyCode::*, KeyModifiers as Modifier};
        let tick_rate = config.tick_rate;
        let key_input_disabled = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel::unbounded::<Event<Key>>();

        {
            thread::scope(move |s| {
                let tx = tx.clone();
                let key_input_disabled = key_input_disabled.clone();

                // s.spawn(move |_| {});

                s.spawn(move |_| {
                    let mut last_tick = Instant::now();
                    loop {
                        let timeout = tick_rate
                            .checked_sub(last_tick.elapsed())
                            .unwrap_or(tick_rate);
                        if key_input_disabled.load(Ordering::Relaxed) {
                            std::thread::sleep(timeout);
                            continue;
                        } else if event::poll(timeout).expect("no events available in thread") {
                            match event::read() {
                                Ok(event) => match event {
                                    event::Event::Key(key) => {
                                        let key = match key.code {
                                            Char(c) => match key.modifiers {
                                                Modifier::NONE | Modifier::SHIFT => Key::Char(c),
                                                Modifier::ALT => Key::Alt(c),
                                                Modifier::CONTROL => Key::Ctrl(c),
                                                _ => Key::Null,
                                            },
                                            Backspace => match key.modifiers {
                                                Modifier::ALT => Key::AltBackspace,
                                                Modifier::CONTROL => Key::CtrlBackspace,
                                                _ => Key::Backspace,
                                            },
                                            Delete => match key.modifiers {
                                                Modifier::ALT => Key::AltDelete,
                                                Modifier::CONTROL => Key::CtrlDelete,
                                                _ => Key::Delete,
                                            },
                                            Tab => Key::Tab,
                                            BackTab => Key::BackTab,
                                            Left => Key::Left,
                                            Right => Key::Right,
                                            Up => Key::Up,
                                            Down => Key::Down,
                                            Home => Key::Home,
                                            End => Key::End,
                                            PageUp => Key::PageUp,
                                            PageDown => Key::PageDown,
                                            Insert => Key::Insert,
                                            Esc => Key::Esc,
                                            F(k) => Key::F(k),
                                            Null => Key::Null,
                                            Enter => Key::Char('\n'),
                                        };
                                        tx.send(Event::Input(key)).ok();
                                        std::thread::sleep(Duration::from_millis(1));
                                        if last_tick.elapsed() >= tick_rate {
                                            tx.send(Event::Tick)
                                                .expect("failed to send tick event");
                                            last_tick = Instant::now();
                                        }
                                    },
                                    event::Event::Mouse(mouse) => {
                                        // tx.send(Event::Mouse(mouse)).ok();
                                    },
                                    event::Event::Resize(w, h) => {},
                                },
                                _ => {
                                    tx.send(Event::Tick).ok();
                                },
                            }
                        }
                    }
                });
            });
        }
        Events { rx }
    }

    pub(crate) fn next(&self) -> Result<Event<Key>, channel::RecvError> {
        self.rx.recv()
    }

    pub(crate) fn leave_tui_mode(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        disable_raw_mode().unwrap();
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        terminal.show_cursor().unwrap();
    }

    pub(crate) fn enter_tui_mode(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();
        terminal.resize(terminal.size().unwrap()).unwrap();
    }
}
