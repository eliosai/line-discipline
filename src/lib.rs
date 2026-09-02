#![doc = include_str!("../README.md")]

mod ctype;
mod result;
mod state;
mod termios;

pub use result::{InputResult, OutputResult, Signal};
pub use state::State;
pub use termios::Termios;
