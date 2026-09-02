use super::State;
use crate::ctype;
use crate::termios::Termios;

/// The byte the kernel escapes in its echo buffer, which always takes one column and skips `OPOST`
const ESCAPE: u8 = 0xff;

/// Columns between tab stops
const TAB: usize = 8;

/// `echo_char`: a control character shows as `^X` under `ECHOCTL`, a tab and everything else as is
pub fn visible(state: &mut State, c: u8) {
    if c != ESCAPE && state.lflag(Termios::ECHOCTL) && ctype::is_cntrl(c) && c != b'\t' {
        state.echo.push(b'^');
        state.echo.push(c ^ 0x40);
        state.cursor.column = state.cursor.column.saturating_add(2);
    } else {
        raw(state, c);
    }
}

/// `echo_char_raw`: one byte through output post-processing, or verbatim without `OPOST`
pub fn raw(state: &mut State, c: u8) {
    if c == ESCAPE {
        state.echo.push(ESCAPE);
        state.cursor.column = state.cursor.column.saturating_add(1);
    } else if state.oflag(Termios::OPOST) {
        state.cursor.write(&state.termios, c, &mut state.echo);
    } else {
        state.echo.push(c);
    }
}

/// Ends an `ECHOPRT` run with the closing slash
pub fn finish_erasing(state: &mut State) {
    if state.erasing {
        raw(state, b'/');
        state.erasing = false;
    }
}

/// Records the column a line starts at, before its first echoed byte
pub fn set_canon_column(state: &mut State) {
    if state.line.is_empty() {
        state.cursor.canon_column = state.cursor.column;
    }
}

/// `ECHO_OP_MOVE_BACK_COL`
pub const fn move_back_column(state: &mut State) {
    state.cursor.column = state.cursor.column.saturating_sub(1);
}

/// `ECHO_OP_ERASE_TAB`: backspace to the tab stop the erased tab left, counting from the line start unless a tab preceded it
pub fn erase_tab(state: &mut State, columns: usize, after_tab: bool) {
    let used = if after_tab {
        columns & 7
    } else {
        (columns & 7).saturating_add(state.cursor.canon_column)
    };
    for _ in 0..TAB.saturating_sub(used & 7) {
        state.echo.push(b'\x08');
        move_back_column(state);
    }
}

/// Hands the echo to the terminal unless `VSTOP` holds output
pub fn commit(state: &mut State, to_master: &mut Vec<u8>) {
    if !state.stopped {
        to_master.append(&mut state.echo);
    }
}
