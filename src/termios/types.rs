use super::consts::{
    B38400, CREAD, CS8, ECHO, ECHOCTL, ECHOE, ECHOK, ECHOKE, ICANON, ICRNL, IEXTEN, ISIG, IXON,
    NUM_CONTROL_CHARS, ONLCR, OPOST, VDISCARD, VEOF, VEOL, VEOL2, VERASE, VINTR, VKILL, VLNEXT,
    VMIN, VQUIT, VREPRINT, VSTART, VSTOP, VSUSP, VWERASE,
};

/// Linux `struct ktermios`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[allow(missing_docs)]
pub struct Termios {
    pub input_flags: u32,
    pub output_flags: u32,
    pub control_flags: u32,
    pub local_flags: u32,
    pub line_discipline: u8,
    pub control_characters: [u8; NUM_CONTROL_CHARS],
    pub input_speed: u32,
    pub output_speed: u32,
}

/// Linux `struct winsize`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[allow(missing_docs)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
}

#[allow(missing_docs, clippy::indexing_slicing)]
impl Termios {
    #[must_use]
    pub const fn i_enabled(&self, flag: u32) -> bool {
        self.input_flags & flag == flag
    }

    #[must_use]
    pub const fn o_enabled(&self, flag: u32) -> bool {
        self.output_flags & flag == flag
    }

    #[must_use]
    pub const fn l_enabled(&self, flag: u32) -> bool {
        self.local_flags & flag == flag
    }

    #[must_use]
    pub const fn is_eof(&self, c: u8) -> bool {
        let eof = self.control_characters[VEOF as usize];
        eof != 0 && c == eof
    }

    #[must_use]
    pub fn is_terminating(&self, c_bytes: &[u8]) -> bool {
        if c_bytes.len() != 1 {
            return false;
        }
        let c = c_bytes[0];
        if self.is_eof(c) {
            return true;
        }
        if c == 0 {
            return false;
        }
        if c == b'\n' || c == self.control_characters[VEOL as usize] {
            return true;
        }
        c == self.control_characters[VEOL2 as usize] && self.l_enabled(IEXTEN)
    }
}

/// Default matches Linux's default replica PTY termios.
impl Default for Termios {
    fn default() -> Self {
        Self {
            input_flags: ICRNL | IXON,
            output_flags: OPOST | ONLCR,
            control_flags: B38400 | CS8 | CREAD,
            local_flags: ISIG | ICANON | ECHO | ECHOE | ECHOK | ECHOCTL | ECHOKE | IEXTEN,
            line_discipline: 0,
            control_characters: default_control_characters(),
            input_speed: 38400,
            output_speed: 38400,
        }
    }
}

#[allow(clippy::arithmetic_side_effects)]
const fn ctrl(c: u8) -> u8 {
    c - b'A' + 1
}

#[allow(clippy::indexing_slicing)]
const fn default_control_characters() -> [u8; NUM_CONTROL_CHARS] {
    let mut cc = [0u8; NUM_CONTROL_CHARS];
    cc[VINTR as usize] = ctrl(b'C');
    cc[VQUIT as usize] = ctrl(b'\\');
    cc[VERASE as usize] = 0x7F;
    cc[VKILL as usize] = ctrl(b'U');
    cc[VEOF as usize] = ctrl(b'D');
    cc[VMIN as usize] = 1;
    cc[VSTART as usize] = ctrl(b'Q');
    cc[VSTOP as usize] = ctrl(b'S');
    cc[VSUSP as usize] = ctrl(b'Z');
    cc[VREPRINT as usize] = ctrl(b'R');
    cc[VDISCARD as usize] = ctrl(b'O');
    cc[VWERASE as usize] = ctrl(b'W');
    cc[VLNEXT as usize] = ctrl(b'V');
    cc
}
