use super::utf8;
use crate::state::State;
use crate::termios::{
    NON_CANON_MAX_BYTES, OCRNL, ONLCR, ONLRET, ONOCR, OPOST, SPACES_PER_TAB, TABDLY, XTABS,
};
use crate::types::OutputResult;

/// Process bytes written by the replica (shell output).
pub fn output(state: &mut State, buf: &[u8]) -> OutputResult {
    let (consumed, output) = opost(state, buf);
    OutputResult { consumed, output }
}

/// OPOST processing. Shared by [`output`] and the echo path in [`super::input`].
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
pub fn opost(state: &mut State, buf: &[u8]) -> (usize, Vec<u8>) {
    if !state.termios.o_enabled(OPOST) {
        let n = buf.len().min(NON_CANON_MAX_BYTES);
        return (n, buf[..n].to_vec());
    }

    let mut out = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        let budget = NON_CANON_MAX_BYTES.saturating_sub(out.len());
        if budget == 0 {
            break;
        }
        let size = utf8::char_len(&state.termios, &buf[pos..]);
        if size > budget {
            break;
        }

        if !emit_char(state, buf[pos], budget, &mut out) {
            break;
        }

        if size > 1 && !matches!(buf[pos], b'\n' | b'\r' | b'\t' | b'\x08') {
            out.extend_from_slice(&buf[pos + 1..pos + size]);
        }
        pos += size;
    }

    (pos, out)
}

#[allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn emit_char(state: &mut State, c: u8, budget: usize, out: &mut Vec<u8>) -> bool {
    match c {
        b'\n' => {
            if state.termios.o_enabled(ONLRET) {
                state.column = 0;
            }
            if state.termios.o_enabled(ONLCR) {
                if budget < 2 {
                    return false;
                }
                out.extend_from_slice(b"\r\n");
            } else {
                out.push(b'\n');
            }
        }
        b'\r' => {
            if state.termios.o_enabled(ONOCR) && state.column == 0 {
                return true;
            }
            if state.termios.o_enabled(OCRNL) {
                out.push(b'\n');
                if state.termios.o_enabled(ONLRET) {
                    state.column = 0;
                }
            } else {
                state.column = 0;
                out.push(b'\r');
            }
        }
        b'\t' => {
            let spaces = SPACES_PER_TAB - (state.column as usize % SPACES_PER_TAB);
            if state.termios.output_flags & TABDLY == XTABS {
                if budget < spaces {
                    return false;
                }
                state.column += spaces as i32;
                out.resize(out.len() + spaces, b' ');
            } else {
                state.column += spaces as i32;
                out.push(b'\t');
            }
        }
        b'\x08' => {
            if state.column > 0 {
                state.column -= 1;
            }
            out.push(b'\x08');
        }
        _ => {
            state.column += 1;
            out.push(c);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{State, Termios, WindowSize};

    fn state() -> State {
        State::new(Termios::default(), WindowSize { rows: 24, cols: 80 })
    }

    #[test]
    fn nl_to_crnl() {
        let mut s = state();
        let r = output(&mut s, b"hello\nworld\n");
        assert_eq!(r.output, b"hello\r\nworld\r\n");
    }

    #[test]
    fn no_opost_passthrough() {
        let mut t = Termios::default();
        t.output_flags = 0;
        let mut s = State::new(t, WindowSize::default());
        let r = output(&mut s, b"hello\nworld");
        assert_eq!(r.output, b"hello\nworld");
    }

    #[test]
    fn tab_expansion() {
        let mut t = Termios::default();
        t.output_flags = OPOST | XTABS;
        let mut s = State::new(t, WindowSize::default());
        let r = output(&mut s, b"\t");
        assert_eq!(r.output, b"        ");
    }

    #[test]
    fn column_tracking() {
        let mut s = state();
        output(&mut s, b"abc");
        assert_eq!(s.column(), 3);
        output(&mut s, b"\r");
        assert_eq!(s.column(), 0);
    }
}
