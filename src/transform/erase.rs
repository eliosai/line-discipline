use super::opost::opost;
use crate::state::State;
use crate::termios::{ECHO, ECHOCTL, IUTF8, Termios, VERASE, VWERASE};

/// Erase characters from the input queue (VERASE or VWERASE).
#[allow(clippy::arithmetic_side_effects)]
pub fn erase(state: &mut State, kill_type: u32, echo_out: &mut Vec<u8>) {
    // KERNEL: VKILL (^U) erase entire line not implemented
    let mut seen_alnum = false;

    while !state.in_queue.read_buf.is_empty() {
        let (cnt, byte) = measure_back(&state.in_queue.read_buf, state.termios.i_enabled(IUTF8));
        if cnt == 0 {
            break;
        }

        if kill_type == VWERASE && !advance_word_boundary(byte, &mut seen_alnum) {
            break;
        }

        let new_len = state.in_queue.read_buf.len() - cnt;
        state.in_queue.read_buf.truncate(new_len);

        if state.termios.l_enabled(ECHO) {
            let raw = erase_echo_raw(&state.termios, byte);
            let (_, processed) = opost(state, &raw);
            echo_out.extend_from_slice(&processed);
        }

        if kill_type == VERASE {
            break;
        }
    }
}

#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
fn measure_back(buf: &[u8], iutf8: bool) -> (usize, u8) {
    if buf.is_empty() {
        return (0, 0);
    }
    let mut cnt = 0;
    let mut is_cont = true;
    let mut byte = 0u8;

    while cnt < buf.len() && is_cont {
        byte = buf[buf.len() - cnt - 1];
        cnt += 1;
        is_cont = iutf8 && (byte & 0xC0) == 0x80;
    }

    if is_cont { (0, 0) } else { (cnt, byte) }
}

fn advance_word_boundary(byte: u8, seen_alnum: &mut bool) -> bool {
    let is_word = (byte as char).is_alphabetic() || byte.is_ascii_digit() || byte == b'_';
    if is_word {
        *seen_alnum = true;
    } else if *seen_alnum {
        return false;
    }
    true
}

#[allow(clippy::arithmetic_side_effects)]
fn erase_echo_raw(termios: &Termios, byte: u8) -> Vec<u8> {
    // KERNEL: ECHOPRT not implemented
    // KERNEL: VERASE without ECHOE not implemented
    // KERNEL: Tab erasure not implemented
    let is_ctrl = byte < 0x20 || byte == 0x7F;
    let n = if is_ctrl {
        if termios.l_enabled(ECHOCTL) { 2 } else { 0 }
    } else {
        1
    };

    let mut out = Vec::with_capacity(n * 3);
    for _ in 0..n {
        out.extend_from_slice(b"\x08 \x08");
    }
    out
}
