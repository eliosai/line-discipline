# The public API

Every public item lives at the crate root. cargo-semver-checks compares each release against the
one before it, and a breaking change bumps the major.

## `State`

One pty's line discipline. `State::new(termios)` starts one, and `State::default()` starts one
with `Termios::default()`.

- `termios()` returns the termios in force
- `set_termios(termios)` installs a termios, the replica's `tcsetattr`, and returns the bytes a
  switch out of canonical mode (or into `EXTPROC`) releases to the program; it also restarts
  output that `VSTOP` held when the new termios drops `IXON`
- `flush_input()` drops the line under edit, the `TCIFLUSH` half of `tcflush`
- `input(bytes)` takes bytes the master wrote and returns an `InputResult`
- `output(bytes)` takes bytes the replica wrote and returns an `OutputResult`

`State` is `Debug`, `Clone`, `PartialEq`, `Eq` and `Default`, and with the `rkyv` feature
`rkyv::Archive`, `Serialize` and `Deserialize`.

## `Termios`

`struct ktermios` as an open record: `input_flags`, `output_flags`, `control_flags` and
`local_flags` (`u32`), `line_discipline` (`u8`), `control_characters` (`[u8; Termios::NCCS]`),
`input_speed` and `output_speed` (`u32`). The struct is `#[non_exhaustive]`, so a value starts
from `Termios::default()`, what `tcgetattr` reports on a fresh pty replica, and sets fields.

The associated constants are the flag bits and `c_cc` indices of `asm-generic/termbits.h`: the
`c_iflag` bits `IGNBRK` through `IUTF8`, the `c_oflag` bits `OPOST` through `FFDLY` with the delay
values, the `c_cflag` bits `CBAUD` through `CRTSCTS` with every `B*` rate and `IBSHIFT`, the
`c_lflag` bits `ISIG` through `EXTPROC`, the indices `VINTR` through `VEOL2`, and `NCCS`.

`Termios` is `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Default`, and with the
`rkyv` feature `rkyv::Archive`, `Serialize` and `Deserialize`.

## `InputResult`

`consumed` (bytes the call took), `to_master` (the echo), `to_replica` (completed input), `eof`
(the program reads end of file once `to_replica` is drained) and `signal` (`Option<Signal>`, the
signal to deliver once `to_replica` is drained). The call takes every byte offered unless an end
of file on an empty line or a signal character ends it, and then `consumed` stops after that
byte. The struct is `#[non_exhaustive]` and `Debug`, `Clone`, `PartialEq`, `Eq` and `Default`.

## `OutputResult`

`consumed` (bytes the call took, all of them or none while `VSTOP` holds output) and `to_master`
(the post-processed bytes, behind any echo `VSTOP` held). The struct is `#[non_exhaustive]` and
`Debug`, `Clone`, `PartialEq`, `Eq` and `Default`.

## `Signal`

`Interrupt` (`SIGINT`, from `VINTR`), `Quit` (`SIGQUIT`, from `VQUIT`) and `Suspend` (`SIGTSTP`,
from `VSUSP`). The enum is `#[non_exhaustive]` and `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
and `Hash`.

## Features

`rkyv` (off by default) adds the rkyv derives to `State` and `Termios`.
