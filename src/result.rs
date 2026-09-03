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
    /// Bytes the call took, fewer than offered when an end of file or a signal ended it
    pub consumed: usize,
    /// Echo for the terminal
    pub to_master: Vec<u8>,
    /// Completed input for the program
    pub to_replica: Vec<u8>,
    /// Whether the program reads end of file once `to_replica` is drained
    pub eof: bool,
    /// The signal for the foreground process group that ended the call
    pub signal: Option<Signal>,
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
    /// Bytes the call took, zero while the `VSTOP` character holds output
    pub consumed: usize,
    /// Post-processed bytes for the terminal
    pub to_master: Vec<u8>,
}
