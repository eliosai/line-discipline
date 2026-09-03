# The public API

Every public item lives at the crate root. cargo-semver-checks compares each release against the
one before it, and a breaking change bumps the major.

## `State`

One pty's line discipline. `State::new(termios)` starts one, and `State::default()` starts one
with `Termios::default()`.

- `termios()` returns the termios in force
- `set_termios(termios)` installs a termios, the replica's `tcsetattr`, and returns a
  `TermiosResult`: the line an `ICANON` or `EXTPROC` change releases to the program, and the echo
  a dropped `IXON` releases to the terminal along with the output `VSTOP` held
- `flush_input()` drops the line under edit, the `TCIFLUSH` half of `tcflush`
- `stop_output()` and `start_output()` are the `TCOOFF` and `TCOON` halves of `tcflow`: the
  first holds output and echo, and only the second releases them, whatever `VSTART`, `IXANY` or
  a dropped `IXON` says in between
- `is_output_stopped()` reports whether `VSTOP` or `stop_output` holds output, so a driver knows
  an `output` call would consume nothing; `start_output` hands the echo it held to the next
  `input` or `output` call, the way `start_tty` wakes the writer without processing echo
- `input(bytes)` takes bytes the master wrote and returns an `InputResult`
- `output(bytes)` takes bytes the replica wrote and returns an `OutputResult`

`State` is `#[non_exhaustive]` with every field private, `Debug`, `Clone`, `PartialEq`, `Eq` and
`Default`, and with the `rkyv` feature `rkyv::Archive`, `Serialize` and `Deserialize`.

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

`to_master` (the echo), `to_replica` (completed input) and `events` (`Vec<Event>`, in order). The
call always takes every byte offered. The struct is `#[non_exhaustive]` and `Debug`, `Clone`,
`PartialEq`, `Eq` and `Default`.

## `Event`

What the program sees between the bytes of `to_replica`, each carrying the offset `at` it falls
at. `Eof` is a read that returns nothing, from a `VEOF` character on an empty canonical line or,
under `EXTPROC`, from a `VEOF` byte the program would read alone. `Signal` carries the `Signal`
to deliver to the foreground process group. The enum is `#[non_exhaustive]` and `Debug`, `Clone`,
`Copy`, `PartialEq`, `Eq` and `Hash`.

## `TermiosResult`

`to_master` (the echo a dropped `IXON` releases) and `to_replica` (the line an `ICANON` or
`EXTPROC` change releases). The struct is `#[non_exhaustive]` and `Debug`, `Clone`, `PartialEq`,
`Eq` and `Default`.

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
