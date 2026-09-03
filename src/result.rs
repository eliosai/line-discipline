/// A signal the discipline raises for the foreground process group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Signal {
    /// `SIGINT`, from the `VINTR` character
    Interrupt,
    /// `SIGQUIT`, from the `VQUIT` character
    Quit,
    /// `SIGTSTP`, from the `VSUSP` character
    Suspend,
}

/// A point in `to_replica` where the program sees more than bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Event {
    /// A read returns end of file once the bytes before `at` are drained
    Eof {
        /// Bytes of `to_replica` the program reads first
        at: usize,
    },
    /// The signal goes to the foreground process group once the bytes before `at` are queued
    Signal {
        /// Bytes of `to_replica` the program may read first
        at: usize,
        /// The signal to send
        signal: Signal,
    },
}

/// What one [`State::input`](crate::State::input) call produced
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct InputResult {
    /// Echo for the terminal
    pub to_master: Vec<u8>,
    /// Completed input for the program
    pub to_replica: Vec<u8>,
    /// Ends of file and signals, in order, each placed in `to_replica`
    pub events: Vec<Event>,
}

/// What one [`State::set_termios`](crate::State::set_termios) call released
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct TermiosResult {
    /// Echo the terminal gets once the new termios drops `IXON` while `VSTOP` held it
    pub to_master: Vec<u8>,
    /// The line under edit, released to the program when `ICANON` or `EXTPROC` changed
    pub to_replica: Vec<u8>,
}

/// What one [`State::output`](crate::State::output) call produced
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OutputResult {
    /// Bytes the call took, zero while `VSTOP` or `stop_output` holds output
    pub consumed: usize,
    /// Post-processed bytes for the terminal, behind any echo the call released
    pub to_master: Vec<u8>,
}
