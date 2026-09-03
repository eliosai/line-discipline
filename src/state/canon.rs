use super::{State, echo};
use crate::ctype;
use crate::result::InputResult;
use crate::termios::Termios;

/// What the canonical rules did with a byte
pub enum Outcome {
    /// The byte edited or ended the line
    Handled,
    /// The byte was an end of file on an empty line
    Eof,
    /// The byte is ordinary input
    Plain,
}

/// How much one kill character removes
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kill {
    Char,
    Word,
    Line,
}

/// `n_tty_receive_char_canon`: editing, literal next and reprint, then the line ends
pub fn receive(state: &mut State, c: u8, out: &mut InputResult) -> Outcome {
    let iexten = state.lflag(Termios::IEXTEN);
    if is_erase(state, c) {
        eraser(state, c);
    } else if iexten && c == state.cc(Termios::VLNEXT) {
        literal_next(state);
    } else if iexten && state.lflag(Termios::ECHO) && c == state.cc(Termios::VREPRINT) {
        reprint(state, c);
    } else {
        return line_end(state, c, out);
    }
    Outcome::Handled
}

/// `VERASE`, `VKILL`, and `VWERASE` under `IEXTEN`
fn is_erase(state: &State, c: u8) -> bool {
    c == state.cc(Termios::VERASE)
        || c == state.cc(Termios::VKILL)
        || (state.lflag(Termios::IEXTEN) && c == state.cc(Termios::VWERASE))
}

/// A newline, an end of file, or a `VEOL` or `VEOL2` byte ends the line
fn line_end(state: &mut State, c: u8, out: &mut InputResult) -> Outcome {
    let iexten = state.lflag(Termios::IEXTEN);
    if c == b'\n' {
        if state.lflag(Termios::ECHO) || state.lflag(Termios::ECHONL) {
            echo::raw(state, b'\n');
        }
        complete(state, Some(b'\n'), out);
    } else if c == state.cc(Termios::VEOF) {
        if state.line.is_empty() {
            return Outcome::Eof;
        }
        complete(state, None, out);
    } else if c == state.cc(Termios::VEOL) || (iexten && c == state.cc(Termios::VEOL2)) {
        end_of_line(state, c, out);
    } else {
        return Outcome::Plain;
    }
    Outcome::Handled
}

/// A `VEOL` or `VEOL2` byte is echoed, doubled under `PARMRK` and ends the line
fn end_of_line(state: &mut State, c: u8, out: &mut InputResult) {
    if state.lflag(Termios::ECHO) {
        echo::set_canon_column(state);
        echo::visible(state, c);
    }
    if c == 0xff && state.iflag(Termios::PARMRK) {
        state.push_line(c);
    }
    complete(state, Some(c), out);
}

/// Hands the line to the program, with its terminator unless an end of file ended it
fn complete(state: &mut State, terminator: Option<u8>, out: &mut InputResult) {
    out.to_replica.append(&mut state.line);
    out.to_replica.extend(terminator);
}

/// `VLNEXT` takes the next byte literally and shows `^` with a backspace over it
fn literal_next(state: &mut State) {
    state.lnext = true;
    if !state.lflag(Termios::ECHO) {
        return;
    }
    echo::finish_erasing(state);
    if state.lflag(Termios::ECHOCTL) {
        echo::raw(state, b'^');
        echo::raw(state, b'\x08');
    }
}

/// `VREPRINT` echoes itself, a newline and the line under edit
fn reprint(state: &mut State, c: u8) {
    echo::finish_erasing(state);
    echo::visible(state, c);
    echo::raw(state, b'\n');
    let mut index = 0;
    while let Some(byte) = state.line.get(index).copied() {
        echo::visible(state, byte);
        index = index.saturating_add(1);
    }
}

/// `eraser`: remove one character, one word or the line, echoing the way the flags ask
fn eraser(state: &mut State, c: u8) {
    if state.line.is_empty() {
        return;
    }
    let kind = if c == state.cc(Termios::VERASE) {
        Kill::Char
    } else if c == state.cc(Termios::VWERASE) {
        Kill::Word
    } else {
        Kill::Line
    };
    if kind == Kill::Line && !erases_line_visibly(state) {
        kill_line(state);
        return;
    }
    erase_run(state, kind);
    if state.line.is_empty() && state.lflag(Termios::ECHO) {
        echo::finish_erasing(state);
    }
}

/// Erases from the end of the line as far as the kind allows, one character at a time
fn erase_run(state: &mut State, kind: Kill) {
    let mut seen_alnum = false;
    while let Some(head) = last_char_start(state) {
        let c = state.line.get(head).copied().unwrap_or(0);
        if kind == Kill::Word && !within_word(c, &mut seen_alnum) {
            break;
        }
        erase_char(state, head, c, kind);
        if kind == Kill::Char {
            break;
        }
    }
}

/// `ECHOK`, `ECHOKE` and `ECHOE` together erase a killed line from the display
const fn erases_line_visibly(state: &State) -> bool {
    state.lflag(Termios::ECHO)
        && state.lflag(Termios::ECHOK)
        && state.lflag(Termios::ECHOKE)
        && state.lflag(Termios::ECHOE)
}

/// A kill the display cannot undo: drop the line and echo the kill character, with a newline under `ECHOK`
fn kill_line(state: &mut State) {
    state.line.clear();
    if !state.lflag(Termios::ECHO) {
        return;
    }
    echo::finish_erasing(state);
    let kill = state.cc(Termios::VKILL);
    echo::visible(state, kill);
    if state.lflag(Termios::ECHOK) {
        echo::raw(state, b'\n');
    }
}

/// Where the last character starts, none when the line is empty or holds only continuation bytes
fn last_char_start(state: &State) -> Option<usize> {
    state
        .line
        .iter()
        .rposition(|&byte| !state.is_continuation(byte))
}

/// `VWERASE` erases through the trailing separators and then one run of word characters
const fn within_word(c: u8, seen_alnum: &mut bool) -> bool {
    if ctype::is_alnum(c) || c == b'_' {
        *seen_alnum = true;
        true
    } else {
        !*seen_alnum
    }
}

/// Echoes one erased character and drops it from the line
fn erase_char(state: &mut State, head: usize, c: u8, kind: Kill) {
    if state.lflag(Termios::ECHO) {
        if state.lflag(Termios::ECHOPRT) {
            echo_printed(state, head, c);
        } else if kind == Kill::Char && !state.lflag(Termios::ECHOE) {
            let erase = state.cc(Termios::VERASE);
            echo::visible(state, erase);
        } else if c == b'\t' {
            erase_tab(state, head);
        } else {
            erase_visible(state, c);
        }
    }
    state.line.truncate(head);
}

/// `ECHOPRT`: print the erased character between a backslash and the slash that ends the run
fn echo_printed(state: &mut State, head: usize, c: u8) {
    if !state.erasing {
        echo::raw(state, b'\\');
        state.erasing = true;
    }
    echo::visible(state, c);
    let mut index = head.saturating_add(1);
    while let Some(byte) = state.line.get(index).copied() {
        echo::raw(state, byte);
        echo::move_back_column(state);
        index = index.saturating_add(1);
    }
}

/// Backs up over a tab by the columns the line used since the previous tab or its start
fn erase_tab(state: &mut State, head: usize) {
    let mut columns = 0_usize;
    let mut after_tab = false;
    for &byte in state.line.iter().take(head).rev() {
        if byte == b'\t' {
            after_tab = true;
            break;
        }
        if ctype::is_cntrl(byte) {
            if state.lflag(Termios::ECHOCTL) {
                columns = columns.saturating_add(2);
            }
        } else if !state.is_continuation(byte) {
            columns = columns.saturating_add(1);
        }
    }
    echo::erase_tab(state, columns, after_tab);
}

/// Rubs out one display cell, or two for a control character shown as `^X`
fn erase_visible(state: &mut State, c: u8) {
    let echoctl = state.lflag(Termios::ECHOCTL);
    if ctype::is_cntrl(c) && echoctl {
        rubout(state);
    }
    if !ctype::is_cntrl(c) || echoctl {
        rubout(state);
    }
}

/// Backspace, space, backspace
fn rubout(state: &mut State) {
    echo::raw(state, b'\x08');
    echo::raw(state, b' ');
    echo::raw(state, b'\x08');
}
