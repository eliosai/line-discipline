# line-discipline justfile
# Usage: just <command> [args]

# List every command
default:
    @just --list

# Run every file level check, which is the same set prek runs on a commit and ci runs on a pull request
lint:
    prek run --all-files

# Format check, then compile every feature pair, then lint every target
# Clippy runs the same front end as `cargo check`, so no plain check pass runs beside it
check:
    cargo fmt --all -- --check
    cargo hack check --feature-powerset --depth 2 --no-dev-deps
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features -- -D warnings

# Compile every feature subset, which the paired sweep in `check` bounds at two
features:
    cargo hack check --feature-powerset --no-dev-deps

# Ask whether the lower bounds the manifests declare actually resolve and build
minimal:
    cargo minimal-versions check --all-features --direct

# Name every dependency no crate in the workspace reaches
unused:
    cargo machete --with-metadata

# Report line coverage over the same run the gate makes, which is a figure to read and never a gate
coverage:
    cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info

# Name every mutant that no test noticed, bounded to what this branch changed
mutants base="origin/main":
    git diff {{base}}... > /tmp/ld-mutants.diff
    cargo mutants --test-tool=nextest --in-diff /tmp/ld-mutants.diff

# Format the crate
fmt:
    cargo fmt --all

# Build every target with every feature
build:
    cargo build --all-targets --all-features

# Run the test suite
test:
    cargo nextest run --all-features
    cargo nextest run --no-default-features

# Run the test suite the way the gate does
test-ci:
    cargo nextest run --profile ci --all-features
    cargo nextest run --profile ci --no-default-features

# Run every doc example, which nextest cannot
test-doc:
    cargo test --all-features --doc

# Build the docs the way docs.rs does, failing on any warning or broken link
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

# Build the docs and open them
docs-open:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --open

# Type-check on the minimum supported Rust version the manifest declares
msrv:
    cargo +1.85 check --all-targets --all-features --locked

# Compare the public API against the last release, or against the given revision
semver-check baseline="":
    cargo semver-checks --all-features {{ if baseline != "" { "--baseline-rev " + baseline } else { "" } }}

# Build the crates.io package and list what ships in it
package-check:
    cargo package --locked --all-features --allow-dirty
    cargo package --locked --all-features --allow-dirty --list

# Check licenses, advisories, duplicate versions and sources
audit:
    cargo deny check

# Run the benches the way CodSpeed does, or plainly with `cargo bench`
bench mode="simulation": (bench-build mode) (bench-run mode)

# Build the benches for one CodSpeed instrument
bench-build mode="simulation":
    cargo codspeed build -m {{mode}}

# Run the benches built for one CodSpeed instrument
bench-run mode="simulation":
    cargo codspeed run -m {{mode}}

# Record the kernel's behavior from a real pty into tests/kernel/cases.txt (Linux only)
capture:
    python3 scripts/capture-cases.py

# Record random pty sessions and replay them against the crate
fuzz seed="0" count="2000" jobs="0":
    python3 scripts/fuzz-cases.py --seed {{seed}} --count {{count}} {{ if jobs == "0" { "" } else { "--jobs " + jobs } }} --out target/fuzz/cases-{{seed}}.txt
    LINE_DISCIPLINE_CASES=target/fuzz/cases-{{seed}}.txt cargo nextest run --all-features --run-ignored ignored-only -E 'test(every_fuzzed_case_replays_byte_for_byte)'

# Fuzz a fresh seed, the way the gate and the push hook do
fuzz-ci count="300":
    just fuzz $(date +%s) {{count}} 4

# Install the git hooks
hooks:
    prek install --prepare-hooks

# Run the hooks against every file
hooks-run:
    prek run --all-files

# Write X.Y.Z into the manifest, the lockfile and CHANGELOG.md, for a pull request from release/vX.Y.Z
release version:
    bash scripts/release.sh bump "{{ version }}"

# Print the version main would publish now, which is empty while its version has a tag
release-plan:
    bash scripts/release.sh plan

# Run the build and test checks the pull request gate runs
ci:
    just lint
    just check
    just test-ci
    just test-doc
    just doc-check
    just package-check
    just audit
    just unused
    just msrv

# Remove every build artifact
clean:
    cargo clean
