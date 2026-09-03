#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the capture file is trusted test data"
)]

use line_discipline::{State, Termios};
use pretty_assertions::assert_eq;

/// Every case `scripts/capture-cases.py` recorded from a real pty
const CASES: &str = include_str!("cases.txt");

/// One pty session: the termios it opened with and the steps that followed
struct Case {
    name: String,
    termios: Termios,
    steps: Vec<Step>,
}

/// One action and what the master and the replica read after it
struct Step {
    action: Action,
    master: Vec<u8>,
    replica: Vec<Vec<u8>>,
}

/// What the capture did to the pty
enum Action {
    WriteMaster(Vec<u8>),
    WriteReplica(Vec<u8>, Option<usize>),
    SetTermios(Termios),
    Flush,
    StopOutput,
    StartOutput,
}

#[test]
fn default_termios_matches_a_fresh_pty_replica() {
    let (default, _) = parse();
    assert_eq!(Termios::default(), default);
}

#[test]
fn every_captured_case_replays_byte_for_byte() {
    let (_, cases) = parse();
    assert_eq!(cases.len(), CASES.matches("\ncase ").count());
    for case in &cases {
        replay(case);
    }
}

/// Runs one case against a fresh `State`, comparing after every step
fn replay(case: &Case) {
    let mut state = State::new(case.termios);
    for (index, step) in case.steps.iter().enumerate() {
        let at = format!("{} step {index}", case.name);
        let (master, replica) = perform(&mut state, step, &at);
        assert_eq!(show(&master), show(&step.master), "{at} master");
        let got: Vec<_> = replica.iter().map(|segment| show(segment)).collect();
        let want: Vec<_> = step.replica.iter().map(|segment| show(segment)).collect();
        assert_eq!(got, want, "{at} replica");
    }
}

/// Applies one step and returns what the master and the replica would read
fn perform(state: &mut State, step: &Step, at: &str) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut master = Vec::new();
    let mut replica = vec![Vec::new()];
    match &step.action {
        Action::WriteMaster(bytes) => feed(state, bytes, &mut master, &mut replica),
        Action::WriteReplica(bytes, written) => {
            let result = state.output(bytes);
            master.extend(result.to_master);
            assert_eq!(
                result.consumed,
                written.unwrap_or(bytes.len()),
                "{at} consumed"
            );
        }
        Action::SetTermios(termios) => {
            let released = state.set_termios(*termios);
            master.extend(released.to_master);
            replica
                .first_mut()
                .expect("one segment")
                .extend(released.to_replica);
        }
        Action::Flush => state.flush_input(),
        Action::StopOutput => state.stop_output(),
        Action::StartOutput => state.start_output(),
    }
    (master, replica)
}

/// Feeds bytes the way a driver does, resuming after each end of file
fn feed(state: &mut State, bytes: &[u8], master: &mut Vec<u8>, replica: &mut Vec<Vec<u8>>) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let result = state.input(rest);
        master.extend(result.to_master);
        replica
            .last_mut()
            .expect("one segment")
            .extend(result.to_replica);
        if result.eof {
            replica.push(Vec::new());
        }
        assert!(result.consumed > 0, "input made no progress");
        rest = rest
            .get(result.consumed..)
            .expect("consumed within the input");
    }
}

/// Bytes as the escaped text the assertion diff shows
fn show(bytes: &[u8]) -> String {
    bytes.escape_ascii().to_string()
}

/// The default termios and every case in the capture file
fn parse() -> (Termios, Vec<Case>) {
    let mut default = Termios::default();
    let mut cases: Vec<Case> = Vec::new();
    for line in CASES.lines() {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "default" => default = termios(rest),
            "case" => cases.push(Case {
                name: rest.to_owned(),
                termios: Termios::default(),
                steps: Vec::new(),
            }),
            _ => parse_line(cases.last_mut().expect("a case"), word, rest),
        }
    }
    (default, cases)
}

/// One line inside a case
fn parse_line(case: &mut Case, word: &str, rest: &str) {
    match word {
        "termios" => set_termios(case, termios(rest)),
        "write" => push_write(case, rest),
        "written" => set_written(case, rest.parse().expect("a count")),
        "flush" => push_step(case, Action::Flush),
        "tcoff" => push_step(case, Action::StopOutput),
        "tcon" => push_step(case, Action::StartOutput),
        "master" => last_step(case).master = hex(rest),
        "replica" => last_step(case).replica.push(hex(rest)),
        "eof" => {}
        other => panic!("unknown line {other}"),
    }
}

/// The first termios opens the case and any later one is a `tcsetattr`
fn set_termios(case: &mut Case, termios: Termios) {
    if case.steps.is_empty() && case.termios == Termios::default() {
        case.termios = termios;
    } else {
        push_step(case, Action::SetTermios(termios));
    }
}

/// `write master HEX` or `write replica HEX`
fn push_write(case: &mut Case, rest: &str) {
    let (side, data) = rest.split_once(' ').unwrap_or((rest, ""));
    let action = match side {
        "master" => Action::WriteMaster(hex(data)),
        "replica" => Action::WriteReplica(hex(data), None),
        other => panic!("unknown side {other}"),
    };
    push_step(case, action);
}

/// Records how much of the last replica write the kernel accepted
fn set_written(case: &mut Case, count: usize) {
    match &mut last_step(case).action {
        Action::WriteReplica(_, written) => *written = Some(count),
        _ => panic!("written after a step that is not a replica write"),
    }
}

fn push_step(case: &mut Case, action: Action) {
    case.steps.push(Step {
        action,
        master: Vec::new(),
        replica: Vec::new(),
    });
}

fn last_step(case: &mut Case) -> &mut Step {
    case.steps.last_mut().expect("a step")
}

/// `IFLAG OFLAG CFLAG LFLAG CC...` with the flags in octal and the characters in hex
fn termios(text: &str) -> Termios {
    let mut fields = text.split(' ');
    let mut flag =
        || u32::from_str_radix(fields.next().expect("a flag"), 8).expect("an octal flag");
    let mut termios = Termios::default();
    termios.input_flags = flag();
    termios.output_flags = flag();
    termios.control_flags = flag();
    termios.local_flags = flag();
    for (slot, byte) in termios.control_characters.iter_mut().zip(fields) {
        *slot = u8::from_str_radix(byte, 16).expect("a hex byte");
    }
    termios
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(
                text.get(index..index.saturating_add(2)).expect("a pair"),
                16,
            )
            .expect("a hex byte")
        })
        .collect()
}
