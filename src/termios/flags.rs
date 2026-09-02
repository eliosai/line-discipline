use super::Termios;

/// The `c_iflag`, `c_oflag`, `c_lflag` bits and the `c_cc` indices from `asm-generic/termbits.h`
impl Termios {
    /// Input flag: ignore a break condition
    pub const IGNBRK: u32 = 0o000_001;
    /// Input flag: a break condition flushes the queues and raises `SIGINT`
    pub const BRKINT: u32 = 0o000_002;
    /// Input flag: ignore framing and parity errors
    pub const IGNPAR: u32 = 0o000_004;
    /// Input flag: mark parity errors in the input, and double a 0xff byte
    pub const PARMRK: u32 = 0o000_010;
    /// Input flag: enable input parity checking
    pub const INPCK: u32 = 0o000_020;
    /// Input flag: strip the eighth bit
    pub const ISTRIP: u32 = 0o000_040;
    /// Input flag: translate newline to carriage return
    pub const INLCR: u32 = 0o000_100;
    /// Input flag: ignore carriage return
    pub const IGNCR: u32 = 0o000_200;
    /// Input flag: translate carriage return to newline
    pub const ICRNL: u32 = 0o000_400;
    /// Input flag: map uppercase to lowercase, with `IEXTEN`
    pub const IUCLC: u32 = 0o001_000;
    /// Input flag: `VSTOP` and `VSTART` hold and release output
    pub const IXON: u32 = 0o002_000;
    /// Input flag: any character releases held output
    pub const IXANY: u32 = 0o004_000;
    /// Input flag: send `VSTOP` and `VSTART` to the terminal, which `n_tty` ignores on a pty
    pub const IXOFF: u32 = 0o010_000;
    /// Input flag: ring the bell when the line is full, which `n_tty` ignores
    pub const IMAXBEL: u32 = 0o020_000;
    /// Input flag: the input is UTF-8, so erasing removes whole sequences
    pub const IUTF8: u32 = 0o040_000;

    /// Output flag: post-process output
    pub const OPOST: u32 = 0o000_001;
    /// Output flag: map lowercase to uppercase
    pub const OLCUC: u32 = 0o000_002;
    /// Output flag: write carriage return before newline
    pub const ONLCR: u32 = 0o000_004;
    /// Output flag: map carriage return to newline
    pub const OCRNL: u32 = 0o000_010;
    /// Output flag: drop a carriage return at column zero
    pub const ONOCR: u32 = 0o000_020;
    /// Output flag: newline returns the column to zero
    pub const ONLRET: u32 = 0o000_040;
    /// Output flag: send fill characters for a delay, which `n_tty` ignores
    pub const OFILL: u32 = 0o000_100;
    /// Output flag: the fill character is DEL, which `n_tty` ignores
    pub const OFDEL: u32 = 0o000_200;
    /// Output flag mask: newline delay, which `n_tty` ignores
    pub const NLDLY: u32 = 0o000_400;
    /// Output flag: no newline delay
    pub const NL0: u32 = 0o000_000;
    /// Output flag: newline delay one
    pub const NL1: u32 = 0o000_400;
    /// Output flag mask: carriage return delay, which `n_tty` ignores
    pub const CRDLY: u32 = 0o003_000;
    /// Output flag: no carriage return delay
    pub const CR0: u32 = 0o000_000;
    /// Output flag: carriage return delay one
    pub const CR1: u32 = 0o001_000;
    /// Output flag: carriage return delay two
    pub const CR2: u32 = 0o002_000;
    /// Output flag: carriage return delay three
    pub const CR3: u32 = 0o003_000;
    /// Output flag mask: tab delay, whose `XTABS` value expands tabs
    pub const TABDLY: u32 = 0o014_000;
    /// Output flag: no tab delay
    pub const TAB0: u32 = 0o000_000;
    /// Output flag: tab delay one
    pub const TAB1: u32 = 0o004_000;
    /// Output flag: tab delay two
    pub const TAB2: u32 = 0o010_000;
    /// Output flag: tab delay three, the same bits as `XTABS`
    pub const TAB3: u32 = 0o014_000;
    /// Output flag: expand tabs to spaces
    pub const XTABS: u32 = 0o014_000;
    /// Output flag mask: backspace delay, which `n_tty` ignores
    pub const BSDLY: u32 = 0o020_000;
    /// Output flag: no backspace delay
    pub const BS0: u32 = 0o000_000;
    /// Output flag: backspace delay one
    pub const BS1: u32 = 0o020_000;
    /// Output flag mask: vertical tab delay, which `n_tty` ignores
    pub const VTDLY: u32 = 0o040_000;
    /// Output flag: no vertical tab delay
    pub const VT0: u32 = 0o000_000;
    /// Output flag: vertical tab delay one
    pub const VT1: u32 = 0o040_000;
    /// Output flag mask: form feed delay, which `n_tty` ignores
    pub const FFDLY: u32 = 0o100_000;
    /// Output flag: no form feed delay
    pub const FF0: u32 = 0o000_000;
    /// Output flag: form feed delay one
    pub const FF1: u32 = 0o100_000;

    /// Local flag: `VINTR`, `VQUIT` and `VSUSP` raise signals
    pub const ISIG: u32 = 0o000_001;
    /// Local flag: canonical mode, input is edited and delivered by line
    pub const ICANON: u32 = 0o000_002;
    /// Local flag: uppercase terminal, which `n_tty` ignores
    pub const XCASE: u32 = 0o000_004;
    /// Local flag: echo input
    pub const ECHO: u32 = 0o000_010;
    /// Local flag: `VERASE` erases the character from the display
    pub const ECHOE: u32 = 0o000_020;
    /// Local flag: `VKILL` echoes a newline
    pub const ECHOK: u32 = 0o000_040;
    /// Local flag: echo newline even without `ECHO`
    pub const ECHONL: u32 = 0o000_100;
    /// Local flag: a signal character leaves the queues alone
    pub const NOFLSH: u32 = 0o000_200;
    /// Local flag: a background write raises `SIGTTOU`, which the caller owns
    pub const TOSTOP: u32 = 0o000_400;
    /// Local flag: echo a control character as `^X`
    pub const ECHOCTL: u32 = 0o001_000;
    /// Local flag: echo erased characters between backslash and slash
    pub const ECHOPRT: u32 = 0o002_000;
    /// Local flag: `VKILL` erases the line from the display
    pub const ECHOKE: u32 = 0o004_000;
    /// Local flag: output is being discarded, which `n_tty` ignores
    pub const FLUSHO: u32 = 0o010_000;
    /// Local flag: reprint pending input, which `n_tty` ignores
    pub const PENDIN: u32 = 0o040_000;
    /// Local flag: `VWERASE`, `VLNEXT`, `VREPRINT`, `VEOL2` and `IUCLC` take effect
    pub const IEXTEN: u32 = 0o100_000;
    /// Local flag: external processing, input passes through untouched
    pub const EXTPROC: u32 = 0o200_000;

    /// `c_cc` index of the interrupt character
    pub const VINTR: usize = 0;
    /// `c_cc` index of the quit character
    pub const VQUIT: usize = 1;
    /// `c_cc` index of the erase character
    pub const VERASE: usize = 2;
    /// `c_cc` index of the kill character
    pub const VKILL: usize = 3;
    /// `c_cc` index of the end of file character
    pub const VEOF: usize = 4;
    /// `c_cc` index of the read timeout, which the caller owns
    pub const VTIME: usize = 5;
    /// `c_cc` index of the minimum read count, which the caller owns
    pub const VMIN: usize = 6;
    /// `c_cc` index of the switch character, which `n_tty` ignores
    pub const VSWTC: usize = 7;
    /// `c_cc` index of the start character
    pub const VSTART: usize = 8;
    /// `c_cc` index of the stop character
    pub const VSTOP: usize = 9;
    /// `c_cc` index of the suspend character
    pub const VSUSP: usize = 10;
    /// `c_cc` index of the end of line character
    pub const VEOL: usize = 11;
    /// `c_cc` index of the reprint character
    pub const VREPRINT: usize = 12;
    /// `c_cc` index of the discard character, which `n_tty` ignores
    pub const VDISCARD: usize = 13;
    /// `c_cc` index of the word erase character
    pub const VWERASE: usize = 14;
    /// `c_cc` index of the literal next character
    pub const VLNEXT: usize = 15;
    /// `c_cc` index of the second end of line character
    pub const VEOL2: usize = 16;
}
