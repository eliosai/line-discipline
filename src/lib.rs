#![doc = include_str!("../README.md")]

mod ctype;
mod result;
mod state;
mod termios;

pub use result::{Event, InputResult, OutputResult, Signal, TermiosResult};
pub use state::State;
pub use termios::Termios;
