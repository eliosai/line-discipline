use super::State;
use crate::ctype;
use crate::termios::Termios;

/// `ECHO_OP_START`, which opens an operation in the echo buffer and escapes itself
const START: u8 = 0xff;

/// `ECHO_OP_ERASE_TAB`
const ERASE_TAB: u8 = 0x80;

/// `ECHO_OP_SET_CANON_COL`
const SET_CANON_COL: u8 = 0x81;

/// `ECHO_OP_MOVE_BACK_COL`
const MOVE_BACK_COL: u8 = 0x82;

/// Bytes the kernel's echo buffer holds, its `N_TTY_BUF_SIZE`
const BUF_SIZE: usize = 4096;

/// `ECHO_COMMIT_WATERMARK`, the bytes that must wait before a commit writes them out
const COMMIT_WATERMARK: usize = 256;

/// `ECHO_BLOCK`, the granularity of commits past the watermark
const BLOCK: usize = 256;

/// `ECHO_DISCARD_WATERMARK`, past which held echo loses its oldest bytes
const DISCARD_WATERMARK: usize = BUF_SIZE - (BLOCK + 32);

/// `echo_char`: a control character under `ECHOCTL` is tagged to show as `^X` when written out
pub fn visible(state: &mut State, c: u8) {
    if c != START && state.lflag(Termios::ECHOCTL) && ctype::is_cntrl(c) && c != b'\t' {
        state.echo.push(START);
    }
    raw(state, c);
}

/// `echo_char_raw`: one byte as is, with the operation byte escaped
pub fn raw(state: &mut State, c: u8) {
    if c == START {
        state.echo.push(START);
    }
    state.echo.push(c);
}

/// Ends an `ECHOPRT` run with the closing slash
pub fn finish_erasing(state: &mut State) {
    if state.erasing {
        raw(state, b'/');
        state.erasing = false;
    }
}

/// `echo_set_canon_col` before a line's first byte, recorded when the echo is written out
pub fn set_canon_column(state: &mut State) {
    if state.line.is_empty() {
        state.echo.extend_from_slice(&[START, SET_CANON_COL]);
    }
}

/// `echo_move_back_col`
pub fn move_back_column(state: &mut State) {
    state.echo.extend_from_slice(&[START, MOVE_BACK_COL]);
}

/// `echo_erase_tab`: the columns used since the line start or the previous tab, modulo 8
pub fn erase_tab(state: &mut State, columns: usize, after_tab: bool) {
    let mut count = u8::try_from(columns & 7).unwrap_or(0);
    if after_tab {
        count |= 0x80;
    }
    state.echo.extend_from_slice(&[START, ERASE_TAB, count]);
}

/// `commit_echoes`: marks the echo so far and writes it out once another block of 256 bytes waits
pub fn commit(state: &mut State, to_master: &mut Vec<u8>) {
    let head = state.echo.len();
    state.echo_mark = head;
    let old = state.echo_commit;
    if head < COMMIT_WATERMARK || within_block(head) > within_block(old) {
        return;
    }
    state.echo_commit = head;
    process(state, to_master);
}

/// How far into its 256 byte block a length reaches, the kernel's `% ECHO_BLOCK`
const fn within_block(length: usize) -> usize {
    length & (BLOCK - 1)
}

/// `process_echoes`: writes out the echo marked so far, which a restart or a program write releases
pub fn release(state: &mut State, to_master: &mut Vec<u8>) {
    if state.echo_mark == 0 {
        return;
    }
    state.echo_commit = state.echo_mark;
    process(state, to_master);
}

/// `flush_echoes`: writes out every echo byte at the end of an input call
pub fn flush(state: &mut State, to_master: &mut Vec<u8>) {
    let head = state.echo.len();
    let silent = !state.lflag(Termios::ECHO) && !state.lflag(Termios::ECHONL);
    if silent || state.echo_commit == head {
        return;
    }
    state.echo_commit = head;
    process(state, to_master);
}

/// The echo buffer reset `isig` performs
pub fn clear(state: &mut State) {
    state.echo.clear();
    state.echo_commit = 0;
    state.echo_mark = 0;
}

/// `__process_echoes`: writes out the committed echo unless output is held, then discards past the watermark
fn process(state: &mut State, to_master: &mut Vec<u8>) {
    let mut tail = 0;
    if !state.is_output_stopped() {
        while tail < state.echo_commit {
            tail = render(state, tail, to_master);
        }
    }
    while state.echo_commit.saturating_sub(tail) >= DISCARD_WATERMARK {
        tail = skip(state, tail);
    }
    state.echo.drain(..tail);
    state.echo_commit = state.echo_commit.saturating_sub(tail);
    state.echo_mark = state.echo_mark.saturating_sub(tail);
}

/// Writes out the byte or operation at `tail` and returns where the next one starts
fn render(state: &mut State, tail: usize, to_master: &mut Vec<u8>) -> usize {
    let c = state.echo.get(tail).copied().unwrap_or(0);
    if c != START {
        if state.oflag(Termios::OPOST) {
            state.cursor.write(&state.termios, c, to_master);
        } else {
            to_master.push(c);
        }
        return tail.saturating_add(1);
    }
    let op = state.echo.get(tail.saturating_add(1)).copied().unwrap_or(0);
    match op {
        ERASE_TAB => {
            erase_tab_columns(state, tail, to_master);
            tail.saturating_add(3)
        }
        SET_CANON_COL => {
            state.cursor.canon_column = state.cursor.column;
            tail.saturating_add(2)
        }
        MOVE_BACK_COL => {
            state.cursor.column = state.cursor.column.saturating_sub(1);
            tail.saturating_add(2)
        }
        START => {
            to_master.push(START);
            state.cursor.column = state.cursor.column.saturating_add(1);
            tail.saturating_add(2)
        }
        _ => {
            to_master.extend_from_slice(&[b'^', op ^ 0x40]);
            state.cursor.column = state.cursor.column.saturating_add(2);
            tail.saturating_add(2)
        }
    }
}

/// `ECHO_OP_ERASE_TAB`: backspaces to the tab stop the erased tab left
fn erase_tab_columns(state: &mut State, tail: usize, to_master: &mut Vec<u8>) {
    let count = state.echo.get(tail.saturating_add(2)).copied().unwrap_or(0);
    let mut used = usize::from(count & 0x7f);
    if count & 0x80 == 0 {
        used = used.wrapping_add(state.cursor.canon_column);
    }
    for _ in 0..8_usize.saturating_sub(used & 7) {
        to_master.push(b'\x08');
        state.cursor.column = state.cursor.column.saturating_sub(1);
    }
}

/// Steps over the byte or operation at `tail` that the discard drops
fn skip(state: &State, tail: usize) -> usize {
    if state.echo.get(tail) != Some(&START) {
        return tail.saturating_add(1);
    }
    if state.echo.get(tail.saturating_add(1)) == Some(&ERASE_TAB) {
        tail.saturating_add(3)
    } else {
        tail.saturating_add(2)
    }
}
