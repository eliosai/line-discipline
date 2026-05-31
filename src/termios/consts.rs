#![allow(missing_docs, dead_code)]

pub const CANON_MAX_BYTES: usize = 4096;
pub const NON_CANON_MAX_BYTES: usize = CANON_MAX_BYTES - 1;
pub const SPACES_PER_TAB: usize = 8;
pub const WAIT_BUF_MAX_BYTES: u64 = 131_072;

// Input flags
pub const IGNBRK: u32 = 0o000_001;
pub const BRKINT: u32 = 0o000_002;
pub const IGNPAR: u32 = 0o000_004;
pub const PARMRK: u32 = 0o000_010;
pub const INPCK: u32 = 0o000_020;
pub const ISTRIP: u32 = 0o000_040;
pub const INLCR: u32 = 0o000_100;
pub const IGNCR: u32 = 0o000_200;
pub const ICRNL: u32 = 0o000_400;
pub const IUCLC: u32 = 0o001_000;
pub const IXON: u32 = 0o002_000;
pub const IXANY: u32 = 0o004_000;
pub const IXOFF: u32 = 0o010_000;
pub const IMAXBEL: u32 = 0o020_000;
pub const IUTF8: u32 = 0o040_000;

// Output flags
pub const OPOST: u32 = 0o000_001;
pub const OLCUC: u32 = 0o000_002;
pub const ONLCR: u32 = 0o000_004;
pub const OCRNL: u32 = 0o000_010;
pub const ONOCR: u32 = 0o000_020;
pub const ONLRET: u32 = 0o000_040;
pub const OFILL: u32 = 0o000_100;
pub const OFDEL: u32 = 0o000_200;
pub const NLDLY: u32 = 0o000_400;
pub const CRDLY: u32 = 0o003_000;
pub const TABDLY: u32 = 0o014_000;
pub const BSDLY: u32 = 0o020_000;
pub const VTDLY: u32 = 0o040_000;
pub const FFDLY: u32 = 0o100_000;
pub const XTABS: u32 = 0o014_000;

// Local flags
pub const ISIG: u32 = 0o000_001;
pub const ICANON: u32 = 0o000_002;
pub const XCASE: u32 = 0o000_004;
pub const ECHO: u32 = 0o000_010;
pub const ECHOE: u32 = 0o000_020;
pub const ECHOK: u32 = 0o000_040;
pub const ECHONL: u32 = 0o000_100;
pub const NOFLSH: u32 = 0o000_200;
pub const TOSTOP: u32 = 0o000_400;
pub const ECHOCTL: u32 = 0o001_000;
pub const ECHOPRT: u32 = 0o002_000;
pub const ECHOKE: u32 = 0o004_000;
pub const FLUSHO: u32 = 0o010_000;
pub const PENDIN: u32 = 0o040_000;
pub const IEXTEN: u32 = 0o100_000;
pub const EXTPROC: u32 = 0o200_000;

// Control character indices
pub const VINTR: u32 = 0;
pub const VQUIT: u32 = 1;
pub const VERASE: u32 = 2;
pub const VKILL: u32 = 3;
pub const VEOF: u32 = 4;
pub const VTIME: u32 = 5;
pub const VMIN: u32 = 6;
pub const VSWTC: u32 = 7;
pub const VSTART: u32 = 8;
pub const VSTOP: u32 = 9;
pub const VSUSP: u32 = 10;
pub const VEOL: u32 = 11;
pub const VREPRINT: u32 = 12;
pub const VDISCARD: u32 = 13;
pub const VWERASE: u32 = 14;
pub const VLNEXT: u32 = 15;
pub const VEOL2: u32 = 16;

pub const NUM_CONTROL_CHARS: usize = 19;

// Control flags
pub const B38400: u32 = 0o000_017;
pub const CS8: u32 = 0o000_060;
pub const CREAD: u32 = 0o000_200;
