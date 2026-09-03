use std::iter;

use super::{Cursor, State, echo, is_continuation};
use crate::ctype;
use crate::result::OutputResult;
use crate::termios::Termios;

/// Columns between tab stops
const TAB: usize = 8;

/// `n_tty_write`: pending echo first, then the bytes through `OPOST`, none while `VSTOP` holds output
pub fn write(state: &mut State, bytes: &[u8]) -> OutputResult {
    let mut out = OutputResult {
        consumed: 0,
        to_master: Vec::new(),
    };
    echo::release(state, &mut out.to_master);
    if state.is_output_stopped() {
        return out;
    }
    if state.oflag(Termios::OPOST) {
        for &c in bytes {
            state.cursor.write(&state.termios, c, &mut out.to_master);
        }
    } else {
        out.to_master.extend_from_slice(bytes);
    }
    out.consumed = bytes.len();
    out
}

impl Cursor {
    /// `do_output_char`: one byte through the output flags, tracking the column it leaves
    pub fn write(&mut self, termios: &Termios, c: u8, out: &mut Vec<u8>) {
        match c {
            b'\n' => self.newline(termios, out),
            b'\r' => self.carriage_return(termios, out),
            b'\t' => self.tab(termios, out),
            b'\x08' => {
                self.column = self.column.saturating_sub(1);
                out.push(c);
            }
            _ => self.plain(termios, c, out),
        }
    }

    /// A newline, with a carriage return first under `ONLCR`, and the column reset under `ONLRET`
    fn newline(&mut self, termios: &Termios, out: &mut Vec<u8>) {
        if termios.output_flags & Termios::ONLRET != 0 {
            self.column = 0;
        }
        if termios.output_flags & Termios::ONLCR != 0 {
            self.column = 0;
            self.canon_column = 0;
            out.extend_from_slice(b"\r\n");
            return;
        }
        self.canon_column = self.column;
        out.push(b'\n');
    }

    /// A carriage return, dropped at column zero under `ONOCR` and turned into a newline under `OCRNL`
    fn carriage_return(&mut self, termios: &Termios, out: &mut Vec<u8>) {
        let oflag = |flag: u32| termios.output_flags & flag != 0;
        if oflag(Termios::ONOCR) && self.column == 0 {
            return;
        }
        if oflag(Termios::OCRNL) {
            if oflag(Termios::ONLRET) {
                self.column = 0;
                self.canon_column = 0;
            }
            out.push(b'\n');
            return;
        }
        self.column = 0;
        self.canon_column = 0;
        out.push(b'\r');
    }

    /// A tab moves to the next stop, as spaces under `XTABS`
    fn tab(&mut self, termios: &Termios, out: &mut Vec<u8>) {
        let spaces = TAB.saturating_sub(self.column & 7);
        self.column = self.column.saturating_add(spaces);
        if termios.output_flags & Termios::TABDLY == Termios::XTABS {
            out.extend(iter::repeat_n(b' ', spaces));
        } else {
            out.push(b'\t');
        }
    }

    /// A printing byte advances the column unless it continues a UTF-8 sequence, and `OLCUC` uppercases it
    fn plain(&mut self, termios: &Termios, c: u8, out: &mut Vec<u8>) {
        let mut c = c;
        if !ctype::is_cntrl(c) {
            if termios.output_flags & Termios::OLCUC != 0 {
                c = ctype::to_upper(c);
            }
            if !is_continuation(termios, c) {
                self.column = self.column.saturating_add(1);
            }
        }
        out.push(c);
    }
}
