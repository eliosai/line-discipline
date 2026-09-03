mod canon;
mod echo;
mod output;
mod receive;

use crate::result::{InputResult, OutputResult, TermiosResult};
use crate::termios::Termios;

/// Bytes one canonical line may hold, the kernel's `N_TTY_BUF_SIZE`
const LINE_MAX: usize = 4096;

/// The screen column and the column the current line started at
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
struct Cursor {
    column: usize,
    canon_column: usize,
}

/// Whether output runs, or which flow control holds it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
enum Flow {
    /// Output and echo go out as they are produced
    #[default]
    Running,
    /// The `VSTOP` character holds output until `VSTART`, `IXANY`, a signal, a dropped `IXON` or `TCOON`
    Xoff,
    /// `TCOOFF` holds output until `TCOON`
    Tcooff,
}

/// One pty's line discipline: the termios, the line under edit, the held echo and the cursor
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct State {
    termios: Termios,
    line: Vec<u8>,
    echo: Vec<u8>,
    cursor: Cursor,
    erasing: bool,
    lnext: bool,
    flow: Flow,
}

impl State {
    /// Starts with an empty line and the cursor at column zero
    #[must_use]
    pub const fn new(termios: Termios) -> Self {
        Self {
            termios,
            line: Vec::new(),
            echo: Vec::new(),
            cursor: Cursor {
                column: 0,
                canon_column: 0,
            },
            erasing: false,
            lnext: false,
            flow: Flow::Running,
        }
    }

    /// The termios in force
    #[must_use]
    pub const fn termios(&self) -> &Termios {
        &self.termios
    }

    /// Installs a termios and returns the line and the echo the change releases
    #[must_use]
    pub fn set_termios(&mut self, termios: Termios) -> TermiosResult {
        let mut out = TermiosResult {
            to_master: Vec::new(),
            to_replica: Vec::new(),
        };
        let mode = Termios::ICANON | Termios::EXTPROC;
        let mode_changed = (self.termios.local_flags ^ termios.local_flags) & mode != 0;
        let ixon_dropped = self.iflag(Termios::IXON) && termios.input_flags & Termios::IXON == 0;
        self.termios = termios;
        if ixon_dropped {
            self.start();
            echo::commit(self, &mut out.to_master);
        }
        if mode_changed {
            self.erasing = false;
            self.lnext = false;
            out.to_replica = std::mem::take(&mut self.line);
        }
        out
    }

    /// Drops the line under edit, the `TCIFLUSH` half of `tcflush`
    pub fn flush_input(&mut self) {
        self.line.clear();
        self.erasing = false;
    }

    /// Holds output and echo until `start_output`, the `TCOOFF` half of `tcflow`
    pub const fn stop_output(&mut self) {
        self.flow = Flow::Tcooff;
    }

    /// Releases what `stop_output` held, the `TCOON` half of `tcflow`
    pub const fn start_output(&mut self) {
        if matches!(self.flow, Flow::Tcooff) {
            self.flow = Flow::Running;
        }
    }

    /// Whether `VSTOP` or `stop_output` holds output, so `output` consumes nothing
    #[must_use]
    pub const fn is_output_stopped(&self) -> bool {
        !matches!(self.flow, Flow::Running)
    }

    /// `stop_tty`: `VSTOP` holds output unless something already does
    const fn stop(&mut self) {
        if matches!(self.flow, Flow::Running) {
            self.flow = Flow::Xoff;
        }
    }

    /// `start_tty`: releases output that `VSTOP` holds, never what `TCOOFF` holds
    const fn start(&mut self) {
        if matches!(self.flow, Flow::Xoff) {
            self.flow = Flow::Running;
        }
    }

    /// Whether `VSTOP` holds output and a restart may release it
    const fn is_xoff(&self) -> bool {
        matches!(self.flow, Flow::Xoff)
    }

    /// Takes bytes the terminal typed and returns the echo, the completed input and the signal
    #[must_use]
    pub fn input(&mut self, bytes: &[u8]) -> InputResult {
        let mut out = InputResult {
            consumed: bytes.len(),
            to_master: Vec::new(),
            to_replica: Vec::new(),
            eof: false,
            signal: None,
        };
        for (index, &byte) in bytes.iter().enumerate() {
            if receive::byte(self, byte, &mut out) {
                out.consumed = index.saturating_add(1);
                break;
            }
        }
        echo::commit(self, &mut out.to_master);
        out
    }

    /// Takes bytes the program wrote and returns them post-processed for the terminal
    #[must_use]
    pub fn output(&mut self, bytes: &[u8]) -> OutputResult {
        output::write(self, bytes)
    }

    const fn iflag(&self, flag: u32) -> bool {
        self.termios.input_flags & flag != 0
    }

    const fn oflag(&self, flag: u32) -> bool {
        self.termios.output_flags & flag != 0
    }

    const fn lflag(&self, flag: u32) -> bool {
        self.termios.local_flags & flag != 0
    }

    /// Adds a byte to the canonical line, dropping the newest byte first when the line is full
    fn push_line(&mut self, c: u8) {
        if self.line.len() >= LINE_MAX {
            self.line.pop();
        }
        self.line.push(c);
    }

    fn cc(&self, index: usize) -> u8 {
        self.termios
            .control_characters
            .get(index)
            .copied()
            .unwrap_or(0)
    }

    const fn is_continuation(&self, c: u8) -> bool {
        is_continuation(&self.termios, c)
    }
}

/// A UTF-8 continuation byte under `IUTF8`, which takes no column of its own
const fn is_continuation(termios: &Termios, c: u8) -> bool {
    termios.input_flags & Termios::IUTF8 != 0 && c & 0xC0 == 0x80
}
