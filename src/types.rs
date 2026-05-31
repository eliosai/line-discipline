/// Signal delivered to the foreground process group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[non_exhaustive]
pub enum Signal {
    /// ^C
    Interrupt,
    /// ^Z
    Suspend,
    /// ^\
    Quit,
}

/// Returned by [`crate::input`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputResult {
    /// Bytes consumed from the input buffer.
    pub consumed: usize,
    /// Echo bytes for the master (terminal display).
    pub output: Vec<u8>,
    /// Completed input for the replica (shell stdin).
    pub to_replica: Vec<u8>,
    /// Signals generated.
    pub signals: Vec<Signal>,
}

/// Returned by [`crate::output`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputResult {
    /// Bytes consumed from the output buffer.
    pub consumed: usize,
    /// Post-processed bytes for the master (terminal display).
    pub output: Vec<u8>,
}

/// Delivers signals to the foreground process group.
pub trait Sys {
    /// Called when a signal character is received and ISIG is set.
    fn signal_foreground(&self, sig: Signal);
}

/// No-op [`Sys`]. Signals are still reported in [`InputResult::signals`].
pub struct NoopSys;

impl Sys for NoopSys {
    fn signal_foreground(&self, _: Signal) {}
}
