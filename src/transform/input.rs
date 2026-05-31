use super::erase::erase;
use super::opost::opost;
use super::utf8;
use crate::state::State;
use crate::termios::{
    CANON_MAX_BYTES, ECHO, ECHONL, ICANON, ICRNL, IEXTEN, IGNCR, INLCR, ISIG, NON_CANON_MAX_BYTES,
    VERASE, VINTR, VQUIT, VSUSP, VWERASE,
};
use crate::types::{InputResult, Signal, Sys};

/// Process bytes written by the master (user typing).
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
pub fn input(state: &mut State, sys: &dyn Sys, buf: &[u8]) -> InputResult {
    let mut r = InputResult::default();

    if state.termios.l_enabled(ICANON) && state.in_queue.readable {
        return r;
    }

    let max = if state.termios.l_enabled(ICANON) {
        CANON_MAX_BYTES
    } else {
        NON_CANON_MAX_BYTES
    };
    let mut pos = 0;

    while pos < buf.len() && state.in_queue.read_buf.len() < CANON_MAX_BYTES {
        let size = utf8::char_len(&state.termios, &buf[pos..]);
        let c = buf[pos];

        if let Some(sig) = check_signal(&state.termios, c) {
            sys.signal_foreground(sig);
            r.signals.push(sig);
            pos += size;
            r.consumed += size;
            continue;
        }

        if try_erase(state, c, &mut r.output) {
            pos += 1;
            r.consumed += 1;
            continue;
        }

        let c_bytes = translate(&state.termios, &buf[pos..pos + size]);

        if should_discard(&state.termios, &state.in_queue.read_buf, &c_bytes) {
            pos += size;
            r.consumed += size;
            continue;
        }

        if state.in_queue.read_buf.len() + size > max {
            break;
        }

        pos += size;
        r.consumed += size;

        if state.termios.l_enabled(ICANON) && state.termios.is_eof(c_bytes[0]) {
            state.in_queue.readable = true;
            break;
        }

        state.in_queue.read_buf.extend_from_slice(&c_bytes);
        echo(state, &c_bytes, &mut r.output);

        if state.termios.l_enabled(ICANON) && state.termios.is_terminating(&c_bytes) {
            state.in_queue.readable = true;
            break;
        }
    }

    if !state.termios.l_enabled(ICANON) && !state.in_queue.read_buf.is_empty() {
        state.in_queue.readable = true;
    }

    if state.in_queue.readable {
        r.to_replica = std::mem::take(&mut state.in_queue.read_buf);
        state.in_queue.readable = false;
    }

    r
}

#[allow(clippy::indexing_slicing)]
const fn check_signal(termios: &crate::termios::Termios, c: u8) -> Option<Signal> {
    if !termios.l_enabled(ISIG) || c == b'\r' || c == b'\n' {
        return None;
    }
    let cc = &termios.control_characters;
    match c {
        _ if c == cc[VINTR as usize] => Some(Signal::Interrupt),
        _ if c == cc[VSUSP as usize] => Some(Signal::Suspend),
        _ if c == cc[VQUIT as usize] => Some(Signal::Quit),
        _ => None,
    }
}

#[allow(clippy::indexing_slicing)]
fn try_erase(state: &mut State, c: u8, echo_out: &mut Vec<u8>) -> bool {
    if !state.termios.l_enabled(ICANON) {
        return false;
    }
    let cc = state.termios.control_characters;
    match c {
        _ if c == cc[VWERASE as usize] && state.termios.l_enabled(IEXTEN) => {
            erase(state, VWERASE, echo_out);
            true
        }
        _ if c == cc[VERASE as usize] => {
            erase(state, VERASE, echo_out);
            true
        }
        _ => false,
    }
}

#[allow(clippy::indexing_slicing)]
fn translate(termios: &crate::termios::Termios, raw: &[u8]) -> Vec<u8> {
    let mut out = raw.to_vec();
    match out[0] {
        b'\r' if termios.i_enabled(IGNCR) => return vec![],
        b'\r' if termios.i_enabled(ICRNL) => out[0] = b'\n',
        b'\n' if termios.i_enabled(INLCR) => out[0] = b'\r',
        _ => {}
    }
    out
}

#[allow(clippy::arithmetic_side_effects)]
fn should_discard(termios: &crate::termios::Termios, read_buf: &[u8], c_bytes: &[u8]) -> bool {
    c_bytes.is_empty()
        || (termios.l_enabled(ICANON)
            && read_buf.len() + c_bytes.len() >= CANON_MAX_BYTES
            && !termios.is_terminating(c_bytes))
}

fn echo(state: &mut State, c_bytes: &[u8], out: &mut Vec<u8>) {
    let should = state.termios.l_enabled(ECHO)
        || (state.termios.l_enabled(ECHONL) && c_bytes.first() == Some(&b'\n'));
    if should {
        let (_, processed) = opost(state, c_bytes);
        out.extend_from_slice(&processed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoopSys, State, Termios, WindowSize};

    fn state() -> State {
        State::new(Termios::default(), WindowSize { rows: 24, cols: 80 })
    }

    #[test]
    fn basic_canonical_line() {
        let mut s = state();
        let r = input(&mut s, &NoopSys, b"ls\n");
        assert_eq!(r.consumed, 3);
        assert_eq!(r.to_replica, b"ls\n");
        assert_eq!(r.output, b"ls\r\n");
    }

    #[test]
    fn canonical_buffers_until_newline() {
        let mut s = state();
        let r = input(&mut s, &NoopSys, b"ls");
        assert_eq!(r.consumed, 2);
        assert!(r.to_replica.is_empty());
        assert_eq!(r.output, b"ls");

        let r = input(&mut s, &NoopSys, b"\n");
        assert_eq!(r.to_replica, b"ls\n");
        assert_eq!(r.output, b"\r\n");
    }

    #[test]
    fn signal_generation() {
        let mut s = state();
        let r = input(&mut s, &NoopSys, b"\x03");
        assert_eq!(r.signals, vec![Signal::Interrupt]);
        assert!(r.to_replica.is_empty());
    }

    #[test]
    fn cr_to_nl_translation() {
        let mut s = state();
        let r = input(&mut s, &NoopSys, b"a\r");
        assert_eq!(r.to_replica, b"a\n");
    }

    #[test]
    fn backspace_erases() {
        let mut s = state();
        input(&mut s, &NoopSys, b"ab");
        let r = input(&mut s, &NoopSys, b"\x7f\n");
        assert_eq!(r.to_replica, b"a\n");
    }

    #[test]
    fn eof_on_empty_delivers_empty() {
        let mut s = state();
        let r = input(&mut s, &NoopSys, b"\x04");
        assert!(r.to_replica.is_empty());
    }

    #[test]
    fn eof_with_data_delivers_without_eof_char() {
        let mut s = state();
        input(&mut s, &NoopSys, b"hi");
        let r = input(&mut s, &NoopSys, b"\x04");
        assert_eq!(r.to_replica, b"hi");
    }
}
