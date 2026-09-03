# line-discipline justfile
# Usage: just <command> [args]

# List every command
default:
    @just --list

# Scan comments, docs and layout, then format-check, type-check and lint both feature sets
check:
    bash scripts/comment-scan.sh
    bash scripts/doc-scan.sh
    bash scripts/layout-scan.sh
    cargo fmt --all -- --check
    cargo check --all-targets --all-features
    cargo check --all-targets --no-default-features
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features -- -D warnings

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
bench:
    cargo codspeed build
    cargo codspeed run

# Record the kernel's behavior from a real pty into tests/kernel/cases.txt (Linux only)
capture:
    python3 scripts/capture-cases.py

# Install the git hooks
hooks:
    prek install --hook-type pre-commit --hook-type pre-push

# Run the hooks against every file
hooks-run:
    prek run --all-files

# Print the version the next merge to main would release
release-plan:
    bash scripts/release.sh --dry-run

# Run everything the gate runs
ci:
    just check
    just test-ci
    just test-doc
    just doc-check
    just package-check
    just audit

# Remove every build artifact
clean:
    cargo clean
