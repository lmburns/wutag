#![allow(clippy::use_self)]
#![allow(unused)]

//! Handles all input events. More keys are created to offer the user more
//! options in their configuration file. E.g., Alt + <key> or Ctrl + <key>

// TODO: Add mouse buttons and scrolling

use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, MouseEvent},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    fmt,
    io::{self, Write},
    string::ToString,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel as channel;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tui::{backend::CrosstermBackend, Terminal};

/// Representation of a combination of keypresses
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) enum Key {
    Char(char),
    Alt(char),
    #[serde(alias = "Control")]
    Ctrl(char),
    Backspace,
    #[serde(alias = "Ctrl-Backspace", alias = "Control-Backspace")]
    CtrlBackspace,
    #[serde(alias = "Alt-Backspace")]
    AltBackspace,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    #[serde(alias = "Page-Up")]
    PageUp,
    #[serde(alias = "Page-Down")]
    PageDown,
    Tab,
    #[serde(alias = "Back-Tab")]
    BackTab,
    Insert,
    Delete,
    #[serde(alias = "Ctrl-Delete", alias = "Control-Delete")]
    CtrlDelete,
    #[serde(alias = "Alt-Delete")]
    AltDelete,
    #[serde(alias = "None")]
    Null,
    #[serde(alias = "Escape")]
    Esc,
    #[serde(alias = "Func", alias = "Function")]
    F(u8),
}

impl Key {
    /// Return the [`Key`] as a human-readable string
    pub(crate) fn name(self) -> String {
        match self {
            Self::Char(key) => format!("{}", key),
            Self::Alt(key) => format!("M-{}", key),
            Self::Ctrl(key) => format!("C-{}", key),
            Self::Backspace => String::from("Backspace"),
            Self::CtrlBackspace => String::from("C-Backspace"),
            Self::AltBackspace => String::from("M-Backspace"),
            Self::Left => String::from("Left"),
            Self::Right => String::from("Right"),
            Self::Up => String::from("Up"),
            Self::Down => String::from("Down"),
            Self::Home => String::from("Home"),
            Self::End => String::from("End"),
            Self::PageUp => String::from("PageUp"),
            Self::PageDown => String::from("PageDown"),
            Self::Tab => String::from("Tab"),
            Self::BackTab => String::from("BackTab"),
            Self::Insert => String::from("Insert"),
            Self::Delete => String::from("Delete"),
            Self::CtrlDelete => String::from("C-Delete"),
            Self::AltDelete => String::from("M-Delete"),
            Self::Null => String::from("Null"),
            Self::Esc => String::from("Escape"),
            Self::F(u) => format!("F{}", u),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EventConfig {
    pub(crate) tick_rate: Duration,
}

impl EventConfig {
    pub(crate) const fn new(tick: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick),
        }
    }
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_micros(16555),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Event<I> {
    Input(I),
    Tick,
}

/// Event types are handled through this
#[derive(Debug)]
pub(crate) struct EventHandler {
    /// Sender
    #[allow(unused_variables)]
    pub(crate) tx:                 Sender<Event<Key>>,
    /// Receiver
    pub(crate) rx:                 Receiver<Event<Key>>,
    /// Event handler
    #[allow(unused_variables)]
    pub(crate) handle:             thread::JoinHandle<()>,
    /// Atomic state of key input
    pub(crate) key_input_disabled: Arc<AtomicBool>,
}

impl EventHandler {
    /// Spawn a loop reading inputs from the user to control the TUI
    pub(crate) fn new(config: EventConfig) -> Self {
        use crossterm::event::{KeyCode as Code, KeyModifiers as Modifier};
        let tick_rate = config.tick_rate;
        let key_input_disabled = Arc::new(AtomicBool::new(false));

        let (tx, rx) = channel::unbounded::<Event<Key>>();

        let handle = {
            let tx = tx.clone();
            let key_input_disabled = Arc::clone(&key_input_disabled);

            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(tick_rate);

                    // crossbeam_channel::select! {}

                    if key_input_disabled.load(Ordering::Relaxed) {
                        std::thread::sleep(timeout);
                        continue;
                    } else if event::poll(timeout).expect("no events available in thread") {
                        match event::read() {
                            Ok(event::Event::Key(key)) => {
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
                                tx.send(Event::Input(key)).expect("failed to send key event");
                                thread::sleep(Duration::from_millis(1));
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
            })
        };
        Self {
            tx,
            rx,
            handle,
            key_input_disabled,
        }
    }

    /// Get next item from the thread
    pub(crate) fn next(&self) -> Result<Event<Key>, channel::RecvError> {
        self.rx.recv()
    }
}
