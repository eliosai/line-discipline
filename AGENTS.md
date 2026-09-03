# line-discipline

line-discipline is the Linux `n_tty` line discipline as a library: one `State` turns the bytes a
terminal types into the lines a program reads, and the bytes a program writes into what the
terminal shows. The crate is the repository root, and nothing else is published.

## Layout

- `src/state` holds `State` and the `n_tty` rules, one file per path: `receive.rs` takes a byte
  from the master, `canon.rs` edits the line, `echo.rs` writes the echo and `output.rs`
  post-processes what the replica wrote; a rule is a free `pub fn` that takes the `State`,
  because a `pub` method on `State` would be public API
- `src/termios` holds `Termios`, the open record of `struct ktermios`, and its constants, and
  `src/ctype.rs` the kernel's character classes
- `tests/` holds the integration tests, which reach only the public API: `tests/kernel` replays
  `cases.txt`, which `scripts/capture-cases.py` records from a real pty, and `tests/api` proves
  the contract the README states
- `docs/` explains what the code cannot: `api.md` the public surface, `discipline.md` the rules
  and what the caller owns, `releasing.md` the pipeline, `todo.md` the open work
- `scripts/` holds the checks `just` runs, and `.github/workflows` runs the same recipes

## Commands

Every task has a `just` recipe, and the gate runs nothing a recipe does not run. `just check`
scans comments, docs and layout, format-checks, then type-checks and lints both feature sets with
warnings denied. `just test` runs nextest, `just test-doc` the doc examples, `just doc-check` the
docs.rs build, `just package-check` the crates.io package, `just semver-check` the public API
against the last release, `just msrv` the 1.85 build, `just audit` cargo-deny, `just capture` the
pty capture (Linux only), `just fuzz` a differential run against a real pty and `just ci` all of
it. `just hooks` installs the prek hooks, so a commit runs `just check` and a push runs `just
test` and `just fuzz-ci`.

## The Kernel Is The Reference

Every rule comes from `drivers/tty/n_tty.c`, and a change to a rule starts in the kernel source,
not in a guess. A new rule gets a case in `scripts/capture-cases.py`, a fresh `just capture` on a
Linux box, and then the code that makes the replay pass. A case that depends on the reader's
timing (a full buffer behind an unread line, or more echo than the kernel's 4096 byte echo buffer
holds) does not belong in the capture, because the crate hands lines back as they complete and
the caller owns the queues.

`scripts/fuzz-cases.py` records random sessions the same way and replays them, so a rule nobody
wrote a case for still has to hold. It runs each session three times and keeps only the ones the
kernel repeats, and it drops a step that raises a signal after more than one 256 byte echo block,
which `docs/discipline.md` lists as the one place the kernel races itself.

## Comments And Prose

ONE LINE. Every comment and every doc comment is exactly one line. No second line, no blank `///`,
no paragraph, no `# Errors` or `# Panics` or `# Safety` section, no example block. This is absolute
and applies to modules, types, fields, functions, macros, tests, and inline comments alike.

The line states what the item is. It does not explain why the item exists, who calls it, what it
returns on failure, or anything the signature already says. Do not end it with a period.

If one line cannot carry the meaning, the name is wrong or the item does too much. Fix the code, do
not add a second line.

The crate doc is the README, pulled in with `include_str!`, so every README example is a doc test.
Project prose uses active voice, concrete terms, and short paragraphs. Read `stop-slop` and
`josh-voice` before writing it. Do not add speculative architecture or duplicate an existing document.

## Visibility

Never write `pub(crate)`, `pub(super)`, `pub(in path)`, or any other restricted visibility. An item
is a plain private `fn` unless another module needs it, and then it is `pub`. Keep the private form
whenever it still compiles.

The public surface is the crate root. Every module is private and the root re-exports what a caller
may name. Every public enum is `#[non_exhaustive]`, every public struct is `#[non_exhaustive]`
with every field public or none, and `docs/api.md` lists every public item.

## Code Quality

- keep functions and methods at 25 lines or less and files under 400 lines; split by behavior first
- never pair `x.rs` with an `x/` directory, and never use `#[path]`; a module with children is `x/mod.rs`
- production code never panics; `unwrap`, `expect` and `panic` live only in tests
- the byte loop allocates nothing per byte beyond the growth of the result vectors
- read `rust-best-practices`, `coding-guidelines` and `stop-slop` before every Rust change, and
  `codebase-design` before changing a module boundary or a public signature

## Tests

- integration tests live in `tests` and exercise the public API alone; a unit test lives in
  `#[cfg(test)] mod tests` inside the file that owns the logic it exercises
- run tests with `cargo nextest`, never `cargo test`; doc examples run through `just test-doc`
- every test proves one exact behavior, and a kernel test compares against a real pty's bytes
- read `tdd` and `rust-testing` before changing behavior or tests

## Releases

Every merge to `main` may release. The release workflow reads the commits since the last tag:
a breaking API change reported by cargo-semver-checks bumps the major, a `feat` the minor, a
`fix`, `perf` or `refactor` the patch, and anything else ships nothing. A breaking pull request
carries the `semver-major` label or the semver gate fails it. `docs/releasing.md` has the rest.

## Skills

Read the matching skill in `.agents/skills` before touching its domain. `.claude/skills` points to
the same directory.

- `stop-slop` and `josh-voice` for prose, `tdd` and `rust-testing` for tests
- `rust-best-practices`, `coding-guidelines` and `codebase-design` for Rust
- `code-review` and `thermo-nuclear-code-quality-review` for every review
- `gh-stack` for pull requests, `grilling` when asked to stress-test a decision, `prek` for hooks

Do not edit a copied skill as part of product work. Update skills in their own change.

## Commits

Use conventional commits. Write the subject line and stop. Keep the subject under 60 characters,
in the imperative, with no trailing period. Never add a trailer. No `Co-Authored-By`, no
`Generated-with`, no attribution of any kind. One commit does one thing, and it compiles and passes
its tests on its own.

A pull request adds at most 1000 lines, or 3000 with the `mechanical` label for verbatim moves,
renames and deletes. Only a reviewer adds `size-exempt`.

## Enforcement

`just check` runs `scripts/comment-scan.sh`, which fails on any comment longer than one line,
`scripts/doc-scan.sh`, which fails on any Rust example the doc tests skip, and
`scripts/layout-scan.sh`, which fails on scoped visibility or a module file paired with a directory.
Fix a violation by deleting lines or renaming, never by rewording around the rule.
