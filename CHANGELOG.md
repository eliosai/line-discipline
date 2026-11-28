# Changelog

## 2.0.0 (2026-11-28)


### Features
- End an input call at a signal character
- state: Own the TCOOFF and TCOON halves of tcflow
- Port the kernel echo buffer and place events in the input


### Fixes
- ci: Drop the bench workflow this repository has no benchmarks for
- ctype: Uppercase Latin-1 letters by the kernel's offset
- state: Bound every byte stored in the canonical line
- state: Drop the oldest held echo past the kernel watermark


### Refactoring
- echo: Match the kernel op codes and split render
- echo: Drop clauses a full render never needs


### Documentation
- Describe the echo buffer, the events and the fuzzer
- Cleaned up the main rust docs


### Housekeeping
- Rebuild the pipeline on canonical workflows and one setup action
- Gate pull requests and release from a release PR
- Run the byte loop benches on CodSpeed
- Add walltime benches on CodSpeed macro runners
- Run all benches on CodSpeed macro runners
- Build benches with the instrument each job runs
- Run the benches through the pinned setup action
- Replay fresh fuzzed cases in the gate
- kernel: Capture echo commits, holds and discards
- Count a mutant that hangs the suite as caught
- Check this repository's own links in the checkout

## 1.0.0 (2026-09-02)


### Features
- Put the n_tty rules behind one State


### Documentation
- State the rules, the API and the open work


### Housekeeping
- Adopt the ecmd tooling and layout
- Gate, size, semver and release workflows
- Set the version to 1.0.0


### Bump
- For release

