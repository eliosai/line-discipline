#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]

mod queue;
mod state;
mod termios;
mod transform;
mod types;

pub use state::{Endpoint, FlushTarget, Readiness, State};
pub use termios::{Termios, WindowSize};
pub use transform::{input, output};
pub use types::{InputResult, NoopSys, OutputResult, Signal, Sys};
