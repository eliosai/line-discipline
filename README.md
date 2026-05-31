# line-discipline

PTY line discipline. Two functions transform bytes between a terminal and a shell.

State serializes with rkyv. No async, no allocator, no OS calls.

```rust
use line_discipline::{input, output, State, NoopSys};

let mut state = State::default();

let r = input(&mut state, &NoopSys, b"ls\n");
assert_eq!(r.to_replica, b"ls\n");
assert_eq!(r.output, b"ls\r\n");

let r = output(&mut state, b"file.txt\n");
assert_eq!(r.output, b"file.txt\r\n");
```

## What it does

`input` processes bytes from the master (terminal emulator) toward the replica (shell):

- Canonical mode line buffering
- VERASE, VWERASE character/word erasure
- Signal generation (^C, ^Z, ^\)
- CR/NL translation (ICRNL, INLCR, IGNCR)
- Echo through OPOST
- EOF delivery
- UTF-8 aware editing (IUTF8)

`output` processes bytes from the replica toward the master:

- ONLCR, OCRNL, ONOCR, ONLRET
- Tab expansion (XTABS)
- Column tracking
- Backspace column adjustment

## State control

```rust,ignore
state.set_termios(termios);
state.set_window_size(ws);
state.flush(Endpoint::Replica, FlushTarget::Both);
state.replica_open();
state.replica_close();
state.set_packet_mode(true);
let r = state.master_readiness();
```

## Sys trait

`input` takes a `&dyn Sys` to deliver signals. Implement one method:

```rust
use line_discipline::{Sys, Signal};

struct MySys;
impl Sys for MySys {
    fn signal_foreground(&self, sig: Signal) {
        // deliver to foreground process group
    }
}
```

Use `NoopSys` in tests or when you handle signals from the returned `InputResult::signals` vec.
