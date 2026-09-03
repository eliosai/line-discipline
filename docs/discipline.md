# The discipline

`State::input` runs the kernel's `n_tty_receive_buf_common` one byte at a time, and
`State::output` runs `n_tty_write`. Every rule below comes from `drivers/tty/n_tty.c`, and
`tests/kernel/cases.txt` holds the bytes a real pty produced for each one.

## What the caller owns

The crate holds the termios, the line under edit, the echo `VSTOP` holds back, and the cursor
column. The caller holds what the kernel keeps outside the line discipline:

- the master and replica file descriptors and the queues between them: a completed line comes
  back in `to_replica` the moment it completes, so the crate never stalls a writer on a full
  buffer, and the caller's queue is the buffer
- the foreground process group: `signal` names the one to deliver, `TOSTOP` and job control are
  the caller's, and a signal the kernel would not send (no session on the tty) is still named
- `VMIN`, `VTIME` and blocking: the crate hands over what is complete and the caller decides when
  a read returns
- `tcflow`: `TCIOFF` and `TCION` write `VSTOP` and `VSTART` to the master, while `TCOOFF` and
  `TCOON` are `stop_output` and `start_output`, since the kernel's `tco_stopped` decides whether
  `VSTART`, `IXANY` or a dropped `IXON` may restart output
- the window size, packet mode (`TIOCPKT`), hangup on last close, `IXOFF` (which `n_tty` does not
  implement on a pty) and `IMAXBEL` (which `n_tty` does not implement)

## Input

Per byte, in order:

1. In canonical mode a line holds at most 4096 bytes: at 4096 the newest byte is dropped before
   the next one is taken and before each byte stored, so the line keeps its first 4095 bytes and
   the newest, and a newline replaces the newest.
2. `ISTRIP` clears the high bit, then `IUCLC` with `IEXTEN` lowercases with the kernel's Latin-1
   table.
3. After `VLNEXT` the byte is ordinary input: echoed as `^X` when it is a control character, then
   queued.
4. Under `EXTPROC` the byte is queued for the program untouched, with no echo and no editing;
   with `ICANON` still set, a `VEOF` byte the program would read on its own becomes an `Eof`
   event instead, the way `copy_from_read_buf` turns it into a zero-length read.
5. A byte is special when it is `\r` under `IGNCR` or `ICRNL`, `\n` under `INLCR`, a canonical
   editing or line end character, `VSTART` or `VSTOP` under `IXON`, or a signal character under
   `ISIG`. NUL is never special, so a control character set to 0 is disabled.
6. Any other byte is ordinary: under `IXON` and `IXANY` it releases held output; under `ECHO` it
   closes an `ECHOPRT` run, records the column the line starts at, and is echoed; under `PARMRK`
   a 0xff byte is doubled; then it is queued.

A special byte, in order:

- `VSTART` releases held output and echo unless `stop_output` holds them, `VSTOP` holds them, and
  neither is echoed or queued.
- `VINTR`, `VQUIT` and `VSUSP` under `ISIG` add a `Signal` event at the offset they fall at in
  `to_replica`, so the caller delivers the signal between the bytes before it and the bytes
  after it. Unless `NOFLSH` is set, the line under edit, the echo waiting to go out, and the
  lines this call already completed are dropped, the way `isig` flushes both queues; a signal
  the same call already named stays, since the kernel sends it before it flushes. Under `IXON`
  output restarts. Under `ECHO` the character is echoed as `^X`.
- Under `IXON` and `IXANY` any other special byte releases held output.
- `\r` is dropped under `IGNCR` and becomes `\n` under `ICRNL`; `\n` becomes `\r` under `INLCR`.
- In canonical mode `VERASE`, `VKILL` and `VWERASE` (with `IEXTEN`) edit the line; `VLNEXT` (with
  `IEXTEN`) takes the next byte literally and echoes `^` with a backspace; `VREPRINT` (with
  `IEXTEN` and `ECHO`) echoes `^R`, a newline and the line; `\n` echoes a newline under `ECHO` or
  `ECHONL` and completes the line; `VEOF` completes the line without itself, or on an empty line
  adds an `Eof` event, the zero-length read the kernel gives the program; `VEOL` and `VEOL2`
  (with `IEXTEN`) are echoed, doubled under `PARMRK`, and complete the line.
- Otherwise the byte is ordinary, except that `\n` echoes as a bare newline instead of `^J`.

## Erasing

- An erase on an empty line does nothing.
- `VERASE` removes the last character, `VWERASE` the trailing separators and then one run of word
  characters (ASCII and Latin-1 letters and digits, and `_`), and `VKILL` the line.
- Under `IUTF8` a character is a lead byte with its continuation bytes, and a lone continuation
  byte at the line start stays.
- `VKILL` erases character by character only under `ECHO`, `ECHOK`, `ECHOKE` and `ECHOE`
  together; otherwise it drops the line and, under `ECHO`, echoes the kill character, with a
  newline under `ECHOK`.
- The echo for one erased character: under `ECHOPRT` a backslash opens a run, the character is
  printed, and a slash closes the run before the next echo, when the line empties, or on reprint;
  under `VERASE` without `ECHOE` the erase character is echoed the way any byte is, so `^?` for
  the default `0x7f` under `ECHOCTL`; a tab backs up to the
  column it left, counted from the previous tab or from the column the line started at; anything
  else is backspace, space, backspace, twice for a control character shown as `^X`, and nothing
  for a control character without `ECHOCTL`.

## Echo

Echo is recorded in the kernel's own buffer format, not as finished bytes. A control character
under `ECHOCTL` is tagged rather than expanded, an erased tab and the two column moves are
operations, and `0xff` escapes itself. The buffer is written out on the kernel's schedule:

- `commit_echoes` runs after each byte and writes nothing until 256 bytes wait, then once per
  further block of 256, so short bursts stay in the buffer until the call ends.
- `flush_echoes` at the end of an `input` call writes out the rest, unless neither `ECHO` nor
  `ECHONL` is set.
- `process_echoes` writes out what is marked when `VSTART`, `IXANY` or a `set_termios` that drops
  `IXON` restarts output, and at the head of every `output` call.
- Nothing is written out while `VSTOP` or `stop_output` holds output, and once the buffer holds
  3808 bytes its oldest operations are dropped, the kernel's `ECHO_DISCARD_WATERMARK`.

Writing out is where the rules apply: a tagged control character becomes `^` and the byte xor
`0x40` and takes two columns, an escaped `0xff` takes one column and skips `OPOST`, and every
other byte goes through the output rules under `OPOST` or out verbatim without it. The kernel's
control table stops at `0x7f`, so `0x80` to `0x9f` are printing bytes. Because the column moves
with the bytes that are written out, echo a signal discards never moves it.

## Output

`output` post-processes under `OPOST`, passes bytes through otherwise, and consumes nothing while
`VSTOP` or `stop_output` holds output, which `is_output_stopped` reports. Per byte:

- `\n`: `ONLRET` resets the column; `ONLCR` writes `\r\n` and resets the column and line start
- `\r`: dropped at column zero under `ONOCR`; becomes `\n` under `OCRNL`, resetting the column
  under `ONLRET`; otherwise resets the column
- `\t`: moves to the next stop of 8, written as spaces under `XTABS`
- backspace: moves back one column
- a control character: written as is, with no column change
- anything else: `OLCUC` uppercases with the kernel's Latin-1 table, and the column advances
  unless the byte continues a UTF-8 sequence under `IUTF8`

## Termios changes

`set_termios` mirrors `n_tty_set_termios`: when `ICANON` or `EXTPROC` changes, the line under
edit is released to the program as raw bytes, the way the kernel marks it readable, and the
`ECHOPRT` run and the literal-next state are reset; when `IXON` is dropped, held output restarts
and the held echo is released to the terminal.
`flush_input` mirrors `TCIFLUSH`: the line under edit and the `ECHOPRT` run are dropped.
`stop_output` and `start_output` mirror `TCOOFF` and `TCOON`: the first holds output and echo the
way `VSTOP` does and blocks every restart but `start_output`, and `start_output` releases the
output at once and the echo with the next `input` or `output` call, the way the kernel's
`start_tty` wakes the writer without processing echo.

## Known differences

- A signal drops the echo this call has not returned yet, which is what a pty does for a write
  short enough that the master has read nothing. Past one 256 byte echo block the kernel has
  already handed some echo to the master's buffer, and whether the master reads it before the
  flush is a race the crate cannot model, so the crate always drops it. `just fuzz` leaves those
  steps out of its comparison for the same reason.
- The kernel's flush on a signal also drops the replica's unread output from earlier calls; the
  crate cannot retract what it already returned.
- Under `PARMRK` the kernel reserves three bytes of room per byte received, so its line fills
  sooner; the crate applies the same 4096 byte limit to every byte it stores, doubled or not, and
  outside canonical mode the caller's queue is the only limit.
- `VREPRINT` echoes the whole line under edit, doubled for control characters under `ECHOCTL`, so
  one byte can produce more than 8000 bytes of echo and a large write can produce far more. The
  kernel drops what the master's buffer cannot hold; the crate returns all of it, so a driver
  hands `input` a bounded chunk rather than a whole read.
