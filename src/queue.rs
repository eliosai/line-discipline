#[derive(Debug, Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct Queue {
    pub(crate) read_buf: Vec<u8>,
    pub(crate) wait_buf: Vec<Vec<u8>>,
    pub(crate) wait_buf_len: u64,
    pub(crate) readable: bool,
}

impl Queue {
    pub(crate) fn flush(&mut self) {
        self.read_buf.clear();
        self.wait_buf.clear();
        self.wait_buf_len = 0;
        self.readable = false;
    }

    pub(crate) fn push_wait_buf_raw(&mut self) {
        for chunk in self.wait_buf.drain(..) {
            self.read_buf.extend_from_slice(&chunk);
        }
        self.wait_buf_len = 0;
    }
}
