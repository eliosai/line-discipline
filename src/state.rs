use crate::queue::Queue;
use crate::termios::{ICANON, Termios, WAIT_BUF_MAX_BYTES, WindowSize};

/// Which PTY endpoint issued an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Endpoint {
    /// Terminal emulator side.
    Master,
    /// Shell/process side.
    Replica,
}

/// Target for [`State::flush`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FlushTarget {
    /// Data heading toward the reader.
    Input,
    /// Data heading toward the writer.
    Output,
    /// Both directions.
    Both,
}

/// Poll readiness flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Readiness {
    /// Data available for reading.
    pub readable: bool,
    /// Space available for writing.
    pub writable: bool,
    /// All replicas closed.
    pub hup: bool,
}

/// Line discipline state. Serialize with rkyv for checkpointing.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct State {
    pub(crate) termios: Termios,
    pub(crate) size: WindowSize,
    pub(crate) column: i32,
    pub(crate) in_queue: Queue,
    pub(crate) out_queue: Queue,
    num_replicas: u32,
    packet: bool,
    packet_status: u8,
}

impl Default for State {
    fn default() -> Self {
        Self::new(Termios::default(), WindowSize::default())
    }
}

#[allow(missing_docs)]
impl State {
    #[must_use]
    pub fn new(termios: Termios, size: WindowSize) -> Self {
        Self {
            termios,
            size,
            column: 0,
            in_queue: Queue::default(),
            out_queue: Queue::default(),
            num_replicas: 0,
            packet: false,
            packet_status: 0,
        }
    }

    #[must_use]
    pub const fn termios(&self) -> &Termios {
        &self.termios
    }

    /// Flushes pending canonical input when switching from ICANON on to off.
    pub fn set_termios(&mut self, t: Termios) {
        let old_canon = self.termios.l_enabled(ICANON);
        self.termios = t;
        if old_canon && !self.termios.l_enabled(ICANON) {
            self.in_queue.push_wait_buf_raw();
            self.in_queue.readable = !self.in_queue.read_buf.is_empty();
        }
    }

    #[must_use]
    pub const fn window_size(&self) -> WindowSize {
        self.size
    }

    pub const fn set_window_size(&mut self, ws: WindowSize) {
        self.size = ws;
    }

    /// TCFLSH ioctl. Queues are relative to the endpoint that issued it.
    pub fn flush(&mut self, endpoint: Endpoint, target: FlushTarget) {
        let (inp, out) = match endpoint {
            Endpoint::Master => (&mut self.out_queue, &mut self.in_queue),
            Endpoint::Replica => (&mut self.in_queue, &mut self.out_queue),
        };
        match target {
            FlushTarget::Input => inp.flush(),
            FlushTarget::Output => out.flush(),
            FlushTarget::Both => {
                inp.flush();
                out.flush();
            }
        }
    }

    #[must_use]
    pub fn master_readiness(&self) -> Readiness {
        Readiness {
            readable: (self.out_queue.readable && !self.out_queue.read_buf.is_empty())
                || (self.packet && self.packet_status != 0),
            writable: self.in_queue.wait_buf_len < WAIT_BUF_MAX_BYTES,
            hup: self.num_replicas == 0,
        }
    }

    #[must_use]
    pub fn replica_readiness(&self) -> Readiness {
        Readiness {
            readable: self.in_queue.readable && !self.in_queue.read_buf.is_empty(),
            writable: self.out_queue.wait_buf_len < WAIT_BUF_MAX_BYTES,
            hup: false,
        }
    }

    pub const fn replica_open(&mut self) {
        self.num_replicas = self.num_replicas.saturating_add(1);
    }

    /// Returns true when the last replica closes (HUP).
    #[must_use]
    pub const fn replica_close(&mut self) -> bool {
        self.num_replicas = self.num_replicas.saturating_sub(1);
        self.num_replicas == 0
    }

    /// TIOCPKT ioctl.
    pub const fn set_packet_mode(&mut self, on: bool) {
        if !on {
            self.packet = false;
            return;
        }
        if !self.packet {
            self.packet_status = 0;
        }
        self.packet = true;
    }

    #[must_use]
    pub const fn packet_mode(&self) -> bool {
        self.packet
    }

    #[must_use]
    pub const fn column(&self) -> i32 {
        self.column
    }
}
