# line-discipline

[![crates.io](https://img.shields.io/crates/v/line-discipline.svg)](https://crates.io/crates/line-discipline)
[![docs.rs](https://docs.rs/line-discipline/badge.svg)](https://docs.rs/line-discipline)
[![MIT](https://img.shields.io/crates/l/line-discipline.svg)](https://github.com/eliosai/line-discipline/blob/main/LICENSE)

line-discipline is the Linux `n_tty` line discipline as a library. One `State` turns the bytes a
terminal types into the lines a program reads, and the bytes a program writes into what the
terminal shows, with the rules the kernel applies on a pty: canonical editing, echo, signals, flow
control and output post-processing. It makes no system calls and holds no file descriptor, so it
runs a pty in user space, inside a shell, or under a test, and every rule is checked byte for byte
against a real pty.

```toml
[dependencies]
line-discipline = "2"
```

## A line from keystrokes

```rust
use line_discipline::State;

let mut state = State::default();

let typed = state.input(b"ls -la\x7f\x7fl\n");
assert_eq!(typed.to_replica, b"ls -l\n");
assert_eq!(typed.to_master, b"ls -la\x08 \x08\x08 \x08l\r\n");

let shown = state.output(b"file.txt\n");
assert_eq!(shown.to_master, b"file.txt\r\n");
```

`State::default()` carries the termios a fresh pty replica reports: canonical mode, echo with
`ECHOE` and `ECHOCTL`, `ICRNL` on input and `ONLCR` on output. `input` takes what the master wrote
and returns the echo for the master, the completed input for the replica and the signal for the
foreground process group. `output` takes what the replica wrote and returns it post-processed for
the master.

## Signals and end of file

```rust
use line_discipline::{Event, Signal, State};

let mut state = State::default();

let interrupted = state.input(b"ab\x03cd\n");
assert_eq!(interrupted.to_replica, b"cd\n");
assert_eq!(interrupted.to_master, b"^Ccd\r\n");
assert_eq!(
    interrupted.events,
    [Event::Signal { at: 0, signal: Signal::Interrupt }]
);

let ended = state.input(b"\x04echo\n");
assert_eq!(ended.to_replica, b"echo\n");
assert_eq!(ended.events, [Event::Eof { at: 0 }]);
```

`events` carries what the program sees between bytes. Each one names the offset in `to_replica`
it falls at, so the caller writes the bytes before it, delivers the signal or lets the reader see
end of file, and writes the rest. Unless `NOFLSH` is set a signal character also drops the line
under edit, the echo waiting to go out, and the lines the same call had already completed, the
way the kernel's `isig` flushes both queues.

## Flow control

```rust
use line_discipline::State;

let mut state = State::default();

assert_eq!(state.input(b"\x13ab").to_master, b"");
let held = state.output(b"hello\n");
assert_eq!(held.consumed, 0);

assert_eq!(state.input(b"\x11").to_master, b"ab");
let shown = state.output(b"hello\n");
assert_eq!(shown.to_master, b"hello\r\n");
```

Under `IXON` the `VSTOP` character holds output and echo, and `VSTART`, any byte under `IXANY`,
or a signal character releases them. `stop_output` and `start_output` are the `TCOOFF` and
`TCOON` halves of `tcflow`. While output is held `is_output_stopped` is true and `output`
consumes nothing, and the caller retries once it is false.

## Termios

`Termios` is `struct ktermios` as an open record, with the flag bits and the `c_cc` indices as
associated constants, so a value from `tcgetattr` maps field for field.

```rust
use line_discipline::{State, Termios};

let mut raw = Termios::default();
raw.local_flags &= !(Termios::ICANON | Termios::ECHO | Termios::ISIG);
raw.input_flags &= !Termios::ICRNL;
raw.output_flags &= !Termios::OPOST;

let mut state = State::default();
assert_eq!(state.input(b"ab").to_master, b"ab");
assert_eq!(state.set_termios(raw).to_replica, b"ab");
assert_eq!(state.input(b"\x03\r").to_replica, b"\x03\r");
```

`set_termios` returns the line a switch out of canonical mode releases to the program and the
echo a dropped `IXON` releases to the terminal, and `flush_input` is the `TCIFLUSH` half of
`tcflush`. The discipline reads the input, output and
local flags and the control characters; the control flags, the speeds, `VMIN` and `VTIME` travel
with the record for the caller, who owns the file descriptors, the process groups and the clock.

## Checkpointing

The `rkyv` feature derives `rkyv::Archive`, `Serialize` and `Deserialize` for `State` and
`Termios`, so a discipline in the middle of a line serializes and resumes.

## Layout

- `src/state` holds `State` and the `n_tty` rules, one file per path: `receive.rs` takes a byte
  from the master, `canon.rs` edits the line, `echo.rs` writes the echo, `output.rs`
  post-processes what the replica wrote
- `src/termios` holds `Termios` and its constants, and `src/ctype.rs` the kernel's character
  classes
- `benches/discipline.rs` holds the benches the `bench` workflow measures on every pull request
- `tests/kernel/cases.txt` is captured from a real pty by `scripts/capture-cases.py` and replayed
  byte for byte
- `docs/discipline.md` states each rule and what the caller owns, `docs/api.md` lists every
  public item, `docs/releasing.md` the pipeline, `docs/todo.md` the open work

## Building and testing

```text
just check          # comment, doc and layout scans, fmt, check, clippy -D warnings
just test           # the suite under cargo nextest
just test-doc       # every example in this README and the docs
just doc-check      # the docs.rs build with warnings denied
just semver-check   # the public API against the last release
just bench          # the benches, the way the bench workflow runs them
just ci             # all of it, the way the gate runs
```

## License

MIT
