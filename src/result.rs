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

/// What one [`State::input`](crate::State::input) call produced
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct InputResult {
    /// Bytes the call took, fewer than offered when an end of file ended it
    pub consumed: usize,
    /// Echo for the terminal
    pub to_master: Vec<u8>,
    /// Completed input for the program
    pub to_replica: Vec<u8>,
    /// Whether the program reads end of file once `to_replica` is drained
    pub eof: bool,
    /// Signals for the foreground process group, in order
    pub signals: Vec<Signal>,
}

/// What one [`State::output`](crate::State::output) call produced
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OutputResult {
    /// Bytes the call took, zero while the `VSTOP` character holds output
    pub consumed: usize,
    /// Post-processed bytes for the terminal
    pub to_master: Vec<u8>,
}
