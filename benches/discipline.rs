//! The byte loops the bench workflow measures, one bench per path a pty driver runs hot

use divan::{Bencher, black_box};
use line_discipline::{State, Termios};

fn main() {
    divan::main();
}

/// The default termios with the local flags named turned off
fn without(local: u32) -> Termios {
    let mut termios = Termios::default();
    termios.local_flags &= !local;
    termios
}

/// A byte pattern repeated up to a size
fn repeated(pattern: &[u8], size: usize) -> Vec<u8> {
    pattern.iter().copied().cycle().take(size).collect()
}

/// A command line typed with erasures, echoed and delivered in canonical mode
#[divan::bench]
fn typed_line(bencher: Bencher) {
    let typed = b"git commit --amend --no-edit -m fixup\x7f\x7f\x7f\x7f\x7fsquash\n";
    bencher
        .with_inputs(State::default)
        .bench_values(|mut state| state.input(black_box(typed)));
}

/// 4 KiB pasted into a program in raw mode, with no echo and no editing
#[divan::bench]
fn raw_paste(bencher: Bencher) {
    let raw = without(Termios::ICANON | Termios::ECHO | Termios::ISIG | Termios::IEXTEN);
    let paste = repeated(b"fn main() { println!(\"hello\"); }\n", 4096);
    bencher
        .with_inputs(|| State::new(raw))
        .bench_values(|mut state| state.input(black_box(&paste)));
}

/// 64 KiB of program output through `ONLCR`
#[divan::bench]
fn onlcr_output(bencher: Bencher) {
    let text = repeated(b"drwxr-xr-x  2 josh josh 4096 Sep  2 22:35 target\n", 65536);
    bencher
        .with_inputs(State::default)
        .bench_values(|mut state| state.output(black_box(&text)));
}

/// Tabs expanded under `XTABS` and a UTF-8 word erased under `IUTF8`
#[divan::bench]
fn tab_and_utf8_erase(bencher: Bencher) {
    let mut termios = Termios::default();
    termios.input_flags |= Termios::IUTF8;
    termios.output_flags |= Termios::XTABS;
    let typed = "\tétude\tnaïve\u{7f}\u{7f}\u{7f}\u{7f}\u{7f}\u{7f}\u{17}\n".as_bytes();
    bencher
        .with_inputs(|| State::new(termios))
        .bench_values(|mut state| state.input(black_box(typed)));
}
