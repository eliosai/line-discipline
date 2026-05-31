use crate::termios::{IUTF8, Termios};

pub fn char_len(termios: &Termios, b: &[u8]) -> usize {
    if termios.i_enabled(IUTF8) {
        utf8_len(b)
    } else {
        1
    }
}

fn utf8_len(b: &[u8]) -> usize {
    let Some(&first) = b.first() else {
        return 0;
    };
    let expected = match first {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 1,
    };
    expected.min(b.len())
}
