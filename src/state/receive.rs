use super::{LINE_MAX, State, canon, echo};
use crate::ctype;
use crate::result::{InputResult, Signal};
use crate::termios::Termios;

/// Takes one byte the terminal typed, true when it is an end of file on an empty line
pub fn byte(state: &mut State, byte: u8, out: &mut InputResult) -> bool {
    if state.lflag(Termios::ICANON) && state.line.len() >= LINE_MAX {
        state.line.pop();
    }
    let c = preprocess(state, byte);
    if state.lnext {
        state.lnext = false;
        plain(state, c, out);
    } else if state.lflag(Termios::EXTPROC) {
        queue(state, c, out);
    } else if is_special(state, c) {
        return special(state, c, out);
    } else {
        plain(state, c, out);
    }
    false
}

/// `ISTRIP` and `IUCLC`, applied before any other rule
const fn preprocess(state: &State, byte: u8) -> u8 {
    let c = if state.iflag(Termios::ISTRIP) {
        byte & 0x7f
    } else {
        byte
    };
    if state.iflag(Termios::IUCLC) && state.lflag(Termios::IEXTEN) {
        ctype::to_lower(c)
    } else {
        c
    }
}

/// The kernel's `char_map`: the bytes that get more than echo and queue, never NUL
fn is_special(state: &State, c: u8) -> bool {
    if c == 0 {
        return false;
    }
    (c == b'\r' && (state.iflag(Termios::IGNCR) || state.iflag(Termios::ICRNL)))
        || (c == b'\n' && state.iflag(Termios::INLCR))
        || (state.lflag(Termios::ICANON) && is_canon_special(state, c))
        || (state.iflag(Termios::IXON)
            && (c == state.cc(Termios::VSTART) || c == state.cc(Termios::VSTOP)))
        || (state.lflag(Termios::ISIG) && signal_for(state, c).is_some())
}

/// The editing and line end characters canonical mode watches
fn is_canon_special(state: &State, c: u8) -> bool {
    let iexten = state.lflag(Termios::IEXTEN);
    c == b'\n'
        || c == state.cc(Termios::VERASE)
        || c == state.cc(Termios::VKILL)
        || c == state.cc(Termios::VEOF)
        || c == state.cc(Termios::VEOL)
        || (iexten && c == state.cc(Termios::VWERASE))
        || (iexten && c == state.cc(Termios::VLNEXT))
        || (iexten && c == state.cc(Termios::VEOL2))
        || (iexten && state.lflag(Termios::ECHO) && c == state.cc(Termios::VREPRINT))
}

/// The signal a byte raises under `ISIG`
fn signal_for(state: &State, c: u8) -> Option<Signal> {
    if c == state.cc(Termios::VINTR) {
        Some(Signal::Interrupt)
    } else if c == state.cc(Termios::VQUIT) {
        Some(Signal::Quit)
    } else if c == state.cc(Termios::VSUSP) {
        Some(Signal::Suspend)
    } else {
        None
    }
}

/// `n_tty_receive_char_special`: flow control, signals, translation, then the canonical rules
fn special(state: &mut State, c: u8, out: &mut InputResult) -> bool {
    if state.iflag(Termios::IXON) && flow_control(state, c, out) {
        return false;
    }
    if let Some(signal) = signal_for(state, c).filter(|_| state.lflag(Termios::ISIG)) {
        raise(state, signal, c, out);
        return false;
    }
    restart_on_any(state, out);
    let c = match c {
        b'\r' if state.iflag(Termios::IGNCR) => return false,
        b'\r' if state.iflag(Termios::ICRNL) => b'\n',
        b'\n' if state.iflag(Termios::INLCR) => b'\r',
        c => c,
    };
    if state.lflag(Termios::ICANON) {
        match canon::receive(state, c, out) {
            canon::Outcome::Handled => return false,
            canon::Outcome::Eof => return true,
            canon::Outcome::Plain => {}
        }
    }
    accept(state, c, true, out);
    false
}

/// `VSTART` releases held output and `VSTOP` holds it, and neither is echoed or queued
fn flow_control(state: &mut State, c: u8, out: &mut InputResult) -> bool {
    if c == state.cc(Termios::VSTART) {
        start(state, out);
    } else if c == state.cc(Termios::VSTOP) {
        state.stopped = true;
    } else {
        return false;
    }
    true
}

/// Releases output and the echo it held back
fn start(state: &mut State, out: &mut InputResult) {
    state.stopped = false;
    echo::commit(state, &mut out.to_master);
}

/// Under `IXANY` any byte releases held output
fn restart_on_any(state: &mut State, out: &mut InputResult) {
    if state.stopped && state.iflag(Termios::IXON) && state.iflag(Termios::IXANY) {
        start(state, out);
    }
}

/// `isig`: the signal, the flush unless `NOFLSH`, the release under `IXON`, then the echo
fn raise(state: &mut State, signal: Signal, c: u8, out: &mut InputResult) {
    out.signals.push(signal);
    if !state.lflag(Termios::NOFLSH) {
        state.line.clear();
        state.echo.clear();
        state.erasing = false;
        out.to_master.clear();
        out.to_replica.clear();
    }
    if state.iflag(Termios::IXON) {
        state.stopped = false;
    }
    if state.lflag(Termios::ECHO) {
        echo::visible(state, c);
    }
}

/// `n_tty_receive_char`: echo and queue an ordinary byte
fn plain(state: &mut State, c: u8, out: &mut InputResult) {
    restart_on_any(state, out);
    accept(state, c, false, out);
}

/// Echo, with a bare newline when the special path asks for one, then queue with `PARMRK` doubling
fn accept(state: &mut State, c: u8, raw_newline: bool, out: &mut InputResult) {
    if state.lflag(Termios::ECHO) {
        echo::finish_erasing(state);
        if raw_newline && c == b'\n' {
            echo::raw(state, b'\n');
        } else {
            echo::set_canon_column(state);
            echo::visible(state, c);
        }
    }
    if c == 0xff && state.iflag(Termios::PARMRK) {
        queue(state, c, out);
    }
    queue(state, c, out);
}

/// Adds a byte to the line under edit, or hands it to the program outside canonical mode
fn queue(state: &mut State, c: u8, out: &mut InputResult) {
    if state.lflag(Termios::ICANON) && !state.lflag(Termios::EXTPROC) {
        state.push_line(c);
    } else {
        out.to_replica.push(c);
    }
}
