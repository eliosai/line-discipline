use super::Termios;

/// The `c_cflag` bits from `asm-generic/termbits.h`, which the discipline carries and never reads
impl Termios {
    /// Control flag mask: the output baud rate
    pub const CBAUD: u32 = 0o010_017;
    /// Baud rate: hang up
    pub const B0: u32 = 0o000_000;
    /// Baud rate: 50
    pub const B50: u32 = 0o000_001;
    /// Baud rate: 75
    pub const B75: u32 = 0o000_002;
    /// Baud rate: 110
    pub const B110: u32 = 0o000_003;
    /// Baud rate: 134.5
    pub const B134: u32 = 0o000_004;
    /// Baud rate: 150
    pub const B150: u32 = 0o000_005;
    /// Baud rate: 200
    pub const B200: u32 = 0o000_006;
    /// Baud rate: 300
    pub const B300: u32 = 0o000_007;
    /// Baud rate: 600
    pub const B600: u32 = 0o000_010;
    /// Baud rate: 1200
    pub const B1200: u32 = 0o000_011;
    /// Baud rate: 1800
    pub const B1800: u32 = 0o000_012;
    /// Baud rate: 2400
    pub const B2400: u32 = 0o000_013;
    /// Baud rate: 4800
    pub const B4800: u32 = 0o000_014;
    /// Baud rate: 9600
    pub const B9600: u32 = 0o000_015;
    /// Baud rate: 19200
    pub const B19200: u32 = 0o000_016;
    /// Baud rate: 38400
    pub const B38400: u32 = 0o000_017;
    /// Baud rate: 19200, the historical external A rate
    pub const EXTA: u32 = Self::B19200;
    /// Baud rate: 38400, the historical external B rate
    pub const EXTB: u32 = Self::B38400;
    /// Control flag mask: character size
    pub const CSIZE: u32 = 0o000_060;
    /// Control flag: five data bits
    pub const CS5: u32 = 0o000_000;
    /// Control flag: six data bits
    pub const CS6: u32 = 0o000_020;
    /// Control flag: seven data bits
    pub const CS7: u32 = 0o000_040;
    /// Control flag: eight data bits
    pub const CS8: u32 = 0o000_060;
    /// Control flag: two stop bits
    pub const CSTOPB: u32 = 0o000_100;
    /// Control flag: enable the receiver
    pub const CREAD: u32 = 0o000_200;
    /// Control flag: generate parity
    pub const PARENB: u32 = 0o000_400;
    /// Control flag: odd parity
    pub const PARODD: u32 = 0o001_000;
    /// Control flag: hang up on last close
    pub const HUPCL: u32 = 0o002_000;
    /// Control flag: ignore modem control lines
    pub const CLOCAL: u32 = 0o004_000;
    /// Control flag: the baud rate is in the extended range
    pub const CBAUDEX: u32 = 0o010_000;
    /// Baud rate: any rate, carried in `c_ospeed`
    pub const BOTHER: u32 = 0o010_000;
    /// Baud rate: 57600
    pub const B57600: u32 = 0o010_001;
    /// Baud rate: 115200
    pub const B115200: u32 = 0o010_002;
    /// Baud rate: 230400
    pub const B230400: u32 = 0o010_003;
    /// Baud rate: 460800
    pub const B460800: u32 = 0o010_004;
    /// Baud rate: 500000
    pub const B500000: u32 = 0o010_005;
    /// Baud rate: 576000
    pub const B576000: u32 = 0o010_006;
    /// Baud rate: 921600
    pub const B921600: u32 = 0o010_007;
    /// Baud rate: 1000000
    pub const B1000000: u32 = 0o010_010;
    /// Baud rate: 1152000
    pub const B1152000: u32 = 0o010_011;
    /// Baud rate: 1500000
    pub const B1500000: u32 = 0o010_012;
    /// Baud rate: 2000000
    pub const B2000000: u32 = 0o010_013;
    /// Baud rate: 2500000
    pub const B2500000: u32 = 0o010_014;
    /// Baud rate: 3000000
    pub const B3000000: u32 = 0o010_015;
    /// Baud rate: 3500000
    pub const B3500000: u32 = 0o010_016;
    /// Baud rate: 4000000
    pub const B4000000: u32 = 0o010_017;
    /// Control flag mask: the input baud rate
    pub const CIBAUD: u32 = 0o2_003_600_000;
    /// Bits the input baud rate sits above the output baud rate
    pub const IBSHIFT: u32 = 16;
    /// Control flag: mark or space parity
    pub const CMSPAR: u32 = 0o10_000_000_000;
    /// Control flag: hardware flow control
    pub const CRTSCTS: u32 = 0o20_000_000_000;
}
