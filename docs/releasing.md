# Releasing

Every merge to `main` may release. `release.yml` runs on each push to `main`, skips its own
`chore(release)` commits, reruns `just check` and `just test-ci`, and hands the rest to
`scripts/release.sh`. `just release-plan` prints the version the next merge would release without
touching anything.

## Picking the version

The script reads the version from `Cargo.toml` and the last `v*` tag.

1. With no tag at all, it releases the `Cargo.toml` version as it stands. That is how 1.0.0 ships
   over the 0.2.0 that was published by hand.
2. With a tag that matches `Cargo.toml` but no matching version on crates.io, it publishes that
   version again, which is how a run that failed after the push is retried through
   `workflow_dispatch`.
3. With a `Cargo.toml` version past the last tag, it releases that version, so a bump merged by
   hand wins.
4. Otherwise it reads the commits since the last tag. A subject with `!` or a public API break
   that cargo-semver-checks reports against the last tag bumps the major, a `feat` the minor, a
   `fix`, `perf` or `refactor` the patch, and anything else (`docs`, `chore`, `ci`, `test`)
   releases nothing.

## What one release does

The script writes the new version into `Cargo.toml`, refreshes `Cargo.lock`, writes
`CHANGELOG.md` with git-cliff from `cliff.toml`, commits `chore(release): vX.Y.Z` as
`github-actions[bot]`, tags `vX.Y.Z`, pushes both over the release deploy key, runs
`cargo publish --locked`, and creates the GitHub release with the notes for that version.

## What the repository needs

- trusted publishing on crates.io for `line-discipline`, naming the `eliosai/line-discipline`
  repository and the `release.yml` workflow, so the job mints a short-lived token through GitHub's
  OIDC and holds no crates.io secret
- the `RELEASE_DEPLOY_KEY` secret, the private half of the `release` deploy key with write access,
  which `actions/checkout` installs so the push comes from the deploy key
- the `main` ruleset, which requires a pull request with one approval and the `gate`, `msrv`,
  `semver` and `size` checks, and names the deploy key as its only bypass
- the `v*` tag ruleset, which lets only the deploy key create, move or delete a release tag

## Pull requests

The `semver` check compares the public API against the base branch and fails a breaking change
unless the pull request carries the `semver-major` label. The `size` check counts added lines
outside `Cargo.lock`, `CHANGELOG.md`, `.agents/skills` and `tests/kernel/cases.txt`, with a
budget of 1000, 3000 under the `mechanical` label, and none under `size-exempt`.
