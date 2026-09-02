mod control;
mod flags;

/// Entries in `c_cc`
const NCCS: usize = 19;

/// The Linux `struct ktermios` as an open record
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct Termios {
    /// `c_iflag`
    pub input_flags: u32,
    /// `c_oflag`
    pub output_flags: u32,
    /// `c_cflag`
    pub control_flags: u32,
    /// `c_lflag`
    pub local_flags: u32,
    /// `c_line`
    pub line_discipline: u8,
    /// `c_cc`, indexed by the `V*` constants
    pub control_characters: [u8; NCCS],
    /// `c_ispeed`
    pub input_speed: u32,
    /// `c_ospeed`
    pub output_speed: u32,
}

impl Termios {
    /// Length of `control_characters`
    pub const NCCS: usize = NCCS;
}

/// What `tcgetattr` reports on a fresh pty replica
impl Default for Termios {
    fn default() -> Self {
        Self {
            input_flags: Self::ICRNL | Self::IXON,
            output_flags: Self::OPOST | Self::ONLCR,
            control_flags: Self::B38400 | Self::CS8 | Self::CREAD | (Self::B38400 << Self::IBSHIFT),
            local_flags: Self::ISIG
                | Self::ICANON
                | Self::ECHO
                | Self::ECHOE
                | Self::ECHOK
                | Self::ECHOCTL
                | Self::ECHOKE
                | Self::IEXTEN,
            line_discipline: 0,
            control_characters: default_control_characters(),
            input_speed: 38400,
            output_speed: 38400,
        }
    }
}

/// The kernel's `INIT_C_CC`
const fn default_control_characters() -> [u8; NCCS] {
    let mut cc = [0; NCCS];
    cc[Termios::VINTR] = ctrl(b'C');
    cc[Termios::VQUIT] = ctrl(b'\\');
    cc[Termios::VERASE] = 0x7f;
    cc[Termios::VKILL] = ctrl(b'U');
    cc[Termios::VEOF] = ctrl(b'D');
    cc[Termios::VMIN] = 1;
    cc[Termios::VSTART] = ctrl(b'Q');
    cc[Termios::VSTOP] = ctrl(b'S');
    cc[Termios::VSUSP] = ctrl(b'Z');
    cc[Termios::VREPRINT] = ctrl(b'R');
    cc[Termios::VDISCARD] = ctrl(b'O');
    cc[Termios::VWERASE] = ctrl(b'W');
    cc[Termios::VLNEXT] = ctrl(b'V');
    cc
}

/// The control character a letter names, `^C` for `C`
const fn ctrl(letter: u8) -> u8 {
    letter & 0x1f
}
