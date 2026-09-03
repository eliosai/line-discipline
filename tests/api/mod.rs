use line_discipline::{Event, Signal, State, Termios};
use pretty_assertions::assert_eq;

/// The default termios without the local flags named
fn without(local: u32) -> Termios {
    let mut termios = Termios::default();
    termios.local_flags &= !local;
    termios
}

#[test]
fn a_canonical_line_reaches_the_program_with_its_echo() {
    let mut state = State::default();
    let result = state.input(b"ls\n");
    assert_eq!(result.to_replica, b"ls\n");
    assert_eq!(result.to_master, b"ls\r\n");
    assert_eq!(result.events, []);
}

#[test]
fn each_signal_character_names_its_signal_in_order() {
    let mut state = State::default();
    let result = state.input(b"\x03\x1c\x1a");
    assert_eq!(
        result.events,
        [
            Event::Signal {
                at: 0,
                signal: Signal::Interrupt
            },
            Event::Signal {
                at: 0,
                signal: Signal::Quit
            },
            Event::Signal {
                at: 0,
                signal: Signal::Suspend
            },
        ]
    );
    assert_eq!(result.to_master, b"^Z");
}

#[test]
fn a_signal_flushes_the_line_and_the_echo_unless_noflsh() {
    let mut state = State::default();
    let flushed = state.input(b"ab\x03c\n");
    assert_eq!(
        (flushed.to_replica.as_slice(), flushed.to_master.as_slice()),
        (&b"c\n"[..], &b"^Cc\r\n"[..])
    );

    let mut termios = Termios::default();
    termios.local_flags |= Termios::NOFLSH;
    let mut state = State::new(termios);
    let kept = state.input(b"ab\x03c\n");
    assert_eq!(
        (kept.to_replica.as_slice(), kept.to_master.as_slice()),
        (&b"abc\n"[..], &b"ab^Cc\r\n"[..])
    );
}

#[test]
fn a_signal_marks_where_it_falls_among_the_completed_lines() {
    let mut termios = Termios::default();
    termios.local_flags |= Termios::NOFLSH;
    let mut state = State::new(termios);
    let result = state.input(b"a\n\x03b\n");
    assert_eq!(result.to_replica, b"a\nb\n");
    assert_eq!(
        result.events,
        [Event::Signal {
            at: 2,
            signal: Signal::Interrupt
        }]
    );
}

#[test]
fn an_end_of_file_on_an_empty_line_marks_where_the_reader_stops() {
    let mut state = State::default();
    let result = state.input(b"a\n\x04b\n");
    assert_eq!(result.to_replica, b"a\nb\n");
    assert_eq!(result.events, [Event::Eof { at: 2 }]);
}

#[test]
fn an_end_of_file_after_input_delivers_the_line_without_it() {
    let mut state = State::default();
    let result = state.input(b"hi\x04");
    assert_eq!(result.to_replica, b"hi");
    assert_eq!(result.events, []);
}

#[test]
fn a_lone_end_of_file_under_extproc_is_a_zero_length_read() {
    let mut termios = Termios::default();
    termios.local_flags |= Termios::EXTPROC;
    let mut state = State::new(termios);
    let alone = state.input(b"\x04");
    assert_eq!(alone.to_replica, b"");
    assert_eq!(alone.events, [Event::Eof { at: 0 }]);
    let within = state.input(b"ab\x04c");
    assert_eq!(within.to_replica, b"ab\x04c");
    assert_eq!(within.events, []);
}

#[test]
fn leaving_canonical_mode_releases_the_pending_line() {
    let mut state = State::default();
    assert_eq!(state.input(b"ab").to_replica, b"");
    assert_eq!(
        state.set_termios(without(Termios::ICANON)).to_replica,
        b"ab"
    );
    assert_eq!(state.input(b"c").to_replica, b"c");
}

#[test]
fn a_termios_change_within_canonical_mode_keeps_the_pending_line() {
    let mut state = State::default();
    assert_eq!(state.input(b"ab").to_replica, b"");
    assert_eq!(state.set_termios(without(Termios::ECHO)).to_replica, b"");
    assert_eq!(state.input(b"\n").to_replica, b"ab\n");
}

#[test]
fn flush_input_drops_the_pending_line() {
    let mut state = State::default();
    assert_eq!(state.input(b"ab").to_replica, b"");
    state.flush_input();
    assert_eq!(state.input(b"\n").to_replica, b"\n");
}

#[test]
fn stop_holds_output_and_echo_until_start() {
    let mut state = State::default();
    assert_eq!(state.input(b"\x13ab").to_master, b"");
    let held = state.output(b"x");
    assert_eq!((held.consumed, held.to_master.as_slice()), (0, &b""[..]));
    assert_eq!(state.input(b"\x11").to_master, b"ab");
    assert!(!state.is_output_stopped());
    let shown = state.output(b"x");
    assert_eq!((shown.consumed, shown.to_master.as_slice()), (1, &b"x"[..]));
}

#[test]
fn dropping_ixon_releases_the_held_echo_and_output() {
    let mut state = State::default();
    assert_eq!(state.input(b"\x13ab").to_master, b"");
    let mut termios = Termios::default();
    termios.input_flags &= !Termios::IXON;
    let released = state.set_termios(termios);
    assert_eq!(
        (
            released.to_master.as_slice(),
            released.to_replica.as_slice()
        ),
        (&b"ab"[..], &b""[..])
    );
    assert_eq!(state.output(b"x").consumed, 1);
}

#[test]
fn stop_output_holds_output_until_start_output() {
    let mut state = State::default();
    state.stop_output();
    assert!(state.is_output_stopped());
    assert_eq!(state.output(b"x").consumed, 0);
    assert_eq!(state.input(b"\x11").to_master, b"");
    assert!(state.is_output_stopped());
    state.start_output();
    assert!(!state.is_output_stopped());
    assert_eq!(state.output(b"x").consumed, 1);
}

#[test]
fn held_echo_never_outgrows_the_kernel_buffer() {
    let mut state = State::default();
    state.stop_output();
    for _ in 0_u32..1000 {
        assert_eq!(state.input(&[b'a'; 1000]).to_master, b"");
    }
    state.start_output();
    let released = state.output(b"").to_master;
    assert!(released.len() <= 4096, "{} bytes released", released.len());
}

#[test]
fn a_parmrk_flood_keeps_the_line_within_the_kernel_buffer() {
    let mut termios = Termios::default();
    termios.input_flags |= Termios::PARMRK;
    let mut state = State::new(termios);
    assert_eq!(state.input(&[0xff; 5000]).to_replica, b"");
    let line = state.input(b"\n").to_replica;
    assert_eq!(line.len(), 4096);
    assert_eq!((line.first(), line.last()), (Some(&0xff), Some(&b'\n')));
}

#[test]
fn output_without_opost_passes_through() {
    let mut termios = Termios::default();
    termios.output_flags = 0;
    let mut state = State::new(termios);
    assert_eq!(state.output(b"a\n\tb").to_master, b"a\n\tb");
}

#[test]
fn the_termios_accessor_returns_what_was_set() {
    let mut state = State::default();
    let raw = without(Termios::ICANON | Termios::ECHO | Termios::ISIG);
    assert_eq!(
        state.set_termios(raw),
        line_discipline::TermiosResult::default()
    );
    assert_eq!(state.termios(), &raw);
}

#[test]
fn the_default_termios_is_the_pty_replica_default() {
    let termios = Termios::default();
    assert_eq!(termios.input_flags, Termios::ICRNL | Termios::IXON);
    assert_eq!(termios.output_flags, Termios::OPOST | Termios::ONLCR);
    assert_eq!(termios.control_characters.get(Termios::VEOF), Some(&4));
    assert_eq!(termios.control_characters.len(), Termios::NCCS);
    assert_eq!(
        (
            termios.input_speed,
            termios.output_speed,
            termios.line_discipline
        ),
        (38400, 38400, 0)
    );
}

#[cfg(feature = "rkyv")]
#[test]
#[expect(
    clippy::expect_used,
    reason = "the round trip is the behavior under test"
)]
fn a_state_mid_line_survives_an_rkyv_round_trip() {
    let mut state = State::default();
    assert_eq!(state.input(b"ab\x13c").to_master, b"");
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&state).expect("serialize");
    let mut restored = rkyv::from_bytes::<State, rkyv::rancor::Error>(&bytes).expect("deserialize");
    assert_eq!(restored, state);
    assert_eq!(restored.input(b"\x11\n").to_master, b"abc\r\n");
    assert_eq!(restored.input(b"").to_replica, b"");
    let mut original = state;
    assert_eq!(original.input(b"\x11\n").to_replica, b"abc\n");
}
