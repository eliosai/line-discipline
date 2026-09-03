#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the capture file is trusted test data"
)]

use line_discipline::{Event, State, Termios};
use pretty_assertions::assert_eq;

/// Every case `scripts/capture-cases.py` recorded from a real pty
const CASES: &str = include_str!("cases.txt");

/// One pty session: the termios it opened with and the steps that followed
struct Case {
    name: String,
    termios: Termios,
    opened: bool,
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
    let (default, _) = parse(CASES);
    assert_eq!(Termios::default(), default);
}

#[test]
fn every_captured_case_replays_byte_for_byte() {
    let (_, cases) = parse(CASES);
    assert_eq!(cases.len(), CASES.matches("\ncase ").count());
    for case in &cases {
        replay(case);
    }
}

/// Replays the capture `LINE_DISCIPLINE_CASES` names, which `just fuzz` records from random sessions
#[test]
#[ignore = "runs through just fuzz"]
fn every_fuzzed_case_replays_byte_for_byte() {
    let path = std::env::var("LINE_DISCIPLINE_CASES").expect("LINE_DISCIPLINE_CASES");
    let text = std::fs::read_to_string(&path).expect("a capture file");
    let (_, cases) = parse(&text);
    let failures: Vec<String> = cases.iter().filter_map(diverge).collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases diverge\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// Runs one case against a fresh `State`, comparing after every step
fn replay(case: &Case) {
    if let Some(failure) = diverge(case) {
        panic!("{failure}");
    }
}

/// Runs one case against a fresh `State` and describes the first step that differs from the pty
fn diverge(case: &Case) -> Option<String> {
    let mut state = State::new(case.termios);
    for (index, step) in case.steps.iter().enumerate() {
        let at = format!("{} step {index}", case.name);
        let (master, replica) = match perform(&mut state, step) {
            Ok(read) => read,
            Err(problem) => return Some(format!("{at}: {problem}")),
        };
        if master != step.master && !retracted_echo(step, &master) {
            return Some(format!(
                "{at} master\n  got  {}\n  want {}",
                show(&master),
                show(&step.master)
            ));
        }
        if replica != step.replica {
            let got: Vec<_> = replica.iter().map(|segment| show(segment)).collect();
            let want: Vec<_> = step.replica.iter().map(|segment| show(segment)).collect();
            return Some(format!("{at} replica\n  got  {got:?}\n  want {want:?}"));
        }
    }
    None
}

/// Applies one step and returns what the master and the replica would read
fn perform(state: &mut State, step: &Step) -> Result<(Vec<u8>, Vec<Vec<u8>>), String> {
    let mut master = Vec::new();
    let mut replica = vec![Vec::new()];
    match &step.action {
        Action::WriteMaster(bytes) => feed(state, bytes, &mut master, &mut replica)?,
        Action::WriteReplica(bytes, written) => {
            let result = state.output(bytes);
            master.extend(result.to_master);
            let want = written.unwrap_or(bytes.len());
            if result.consumed != want {
                return Err(format!("consumed {} of {want}", result.consumed));
            }
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
    Ok((master, replica))
}

/// Feeds one write and splits the replica's bytes at every end of file
fn feed(
    state: &mut State,
    bytes: &[u8],
    master: &mut Vec<u8>,
    replica: &mut Vec<Vec<u8>>,
) -> Result<(), String> {
    let result = state.input(bytes);
    master.extend(result.to_master);
    let mut start = 0;
    for event in &result.events {
        if let Event::Eof { at } = event {
            let segment = result
                .to_replica
                .get(start..*at)
                .ok_or("an event past the input")?;
            replica.last_mut().expect("one segment").extend(segment);
            replica.push(Vec::new());
            start = *at;
        }
    }
    let rest = result
        .to_replica
        .get(start..)
        .ok_or("an event past the input")?;
    replica.last_mut().expect("one segment").extend(rest);
    Ok(())
}

/// Whether a signal retracted echo the pty had already handed to the master, which races its buffer
fn retracted_echo(step: &Step, master: &[u8]) -> bool {
    let Action::WriteMaster(bytes) = &step.action else {
        return false;
    };
    let signal = bytes.iter().any(|byte| SIGNAL_CHARS.contains(byte));
    signal && master.len() < step.master.len() && step.master.ends_with(master)
}

/// The default `VINTR`, `VQUIT` and `VSUSP` characters
const SIGNAL_CHARS: [u8; 3] = [0x03, 0x1c, 0x1a];

/// Bytes as the escaped text the assertion diff shows
fn show(bytes: &[u8]) -> String {
    bytes.escape_ascii().to_string()
}

/// The default termios and every case in a capture file
fn parse(text: &str) -> (Termios, Vec<Case>) {
    let mut default = Termios::default();
    let mut cases: Vec<Case> = Vec::new();
    for line in text.lines() {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "default" => default = termios(rest),
            "case" => cases.push(Case {
                name: rest.to_owned(),
                termios: Termios::default(),
                opened: false,
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
    if case.opened {
        push_step(case, Action::SetTermios(termios));
    } else {
        case.termios = termios;
        case.opened = true;
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
