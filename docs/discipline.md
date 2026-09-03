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
- the foreground process group: `signals` is a list to deliver, `TOSTOP` and job control are the
  caller's, and a signal the kernel would not send (no session on the tty) is still listed
- `VMIN`, `VTIME` and blocking: the crate hands over what is complete and the caller decides when
  a read returns
- `tcflow`: `TCOOFF` and `TCOON` stop and start the caller's output path, and `TCIOFF` and
  `TCION` write `VSTOP` and `VSTART` to the master
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
4. Under `EXTPROC` the byte is queued for the program untouched, with no echo and no editing.
5. A byte is special when it is `\r` under `IGNCR` or `ICRNL`, `\n` under `INLCR`, a canonical
   editing or line end character, `VSTART` or `VSTOP` under `IXON`, or a signal character under
   `ISIG`. NUL is never special, so a control character set to 0 is disabled.
6. Any other byte is ordinary: under `IXON` and `IXANY` it releases held output; under `ECHO` it
   closes an `ECHOPRT` run, records the column the line starts at, and is echoed; under `PARMRK`
   a 0xff byte is doubled; then it is queued.

A special byte, in order:

- `VSTART` releases held output and echo, `VSTOP` holds them, and neither is echoed or queued.
- `VINTR`, `VQUIT` and `VSUSP` under `ISIG` list their signal. Unless `NOFLSH` is set, the line
  under edit, the held echo, and the echo and lines this call already produced are dropped, the
  way `isig` flushes both queues. Under `IXON` output restarts. Under `ECHO` the character is
  echoed as `^X`.
- Under `IXON` and `IXANY` any other special byte releases held output.
- `\r` is dropped under `IGNCR` and becomes `\n` under `ICRNL`; `\n` becomes `\r` under `INLCR`.
- In canonical mode `VERASE`, `VKILL` and `VWERASE` (with `IEXTEN`) edit the line; `VLNEXT` (with
  `IEXTEN`) takes the next byte literally and echoes `^` with a backspace; `VREPRINT` (with
  `IEXTEN` and `ECHO`) echoes `^R`, a newline and the line; `\n` echoes a newline under `ECHO` or
  `ECHONL` and completes the line; `VEOF` completes the line without itself, or on an empty line
  ends the call with `eof` set; `VEOL` and `VEOL2` (with `IEXTEN`) are echoed, doubled under
  `PARMRK`, and complete the line.
- Otherwise the byte is ordinary, except that `\n` echoes as a bare newline instead of `^J`.

## Erasing

- An erase on an empty line does nothing.
- `VERASE` removes the last character, `VWERASE` the trailing separators and then one run of word
  characters (ASCII and Latin-1 letters and digits, and `_`), and `VKILL` the line.
- Under `IUTF8` a character is a lead byte with its continuation bytes, and a lone continuation
  byte at the line start stays.
- `VKILL` erases character by character only under `ECHO`, `ECHOK`, `ECHOKE` and `ECHOE`
  together; otherwise it drops the line and echoes `^U`, with a newline under `ECHOK`.
- The echo for one erased character: under `ECHOPRT` a backslash opens a run, the character is
  printed, and a slash closes the run before the next echo, when the line empties, or on reprint;
  under `VERASE` without `ECHOE` the erase character is echoed as `^?`; a tab backs up to the
  column it left, counted from the previous tab or from the column the line started at; anything
  else is backspace, space, backspace, twice for a control character shown as `^X`, and nothing
  for a control character without `ECHOCTL`.

## Echo

- Under `ECHOCTL` a control character (`0x00` to `0x1f` and `0x7f`, never a tab) echoes as `^`
  and the character xor `0x40`; the kernel's table stops at `0x7f`, so `0x80` to `0x9f` are
  printing bytes.
- An echoed byte goes through the output rules when `OPOST` is set and out verbatim otherwise;
  `0xff` always takes one column and skips `OPOST`, as the kernel's escaped echo byte does.
- The column a line starts at is recorded before its first echoed byte and used to erase a tab.
- Under `VSTOP` the echo waits in `State` and `VSTART`, `IXANY`, a signal character, or the next
  `output` call after `IXON` is dropped releases it; once more than 4096 bytes wait, the oldest
  are dropped until 3808 remain, the kernel's discard watermark.

## Output

`output` post-processes under `OPOST`, passes bytes through otherwise, and consumes nothing while
`VSTOP` holds output. Per byte:

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
`ECHOPRT` run and the literal-next state are reset; when `IXON` is dropped, held output restarts.
`flush_input` mirrors `TCIFLUSH`: the line under edit and the `ECHOPRT` run are dropped.

## Known differences

- Echo is post-processed when it is produced, not when `VSTART` releases it, so an `OPOST` change
  while output is held applies to later echo only.
- The kernel counts held echo in its own encoding, where a control character shown as `^X` takes
  two bytes and an erased tab three, and discards each time it processes echo; the crate counts
  the rendered bytes and discards when more than 4096 wait, so the boundary differs by a few bytes.
- The kernel's flush on a signal also drops the replica's unread output and the master's unread
  echo from earlier calls; the crate cannot retract what it already returned.
- Under `PARMRK` the kernel reserves three bytes of room per byte received, so its line fills
  sooner; the crate applies the same 4096 byte limit to every byte it stores, doubled or not, and
  outside canonical mode the caller's queue is the only limit.
