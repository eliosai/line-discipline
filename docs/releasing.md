# Releasing

A release is a pull request, and nothing but a merged pull request changes `main`.

## Cutting a release

1. Branch `release/vX.Y.Z` from `main` and run `git fetch --tags`; `just release` needs git-cliff.
2. Run `just release X.Y.Z`. It writes the version into `[package]`, refreshes the crate's entry
   in `Cargo.lock` and writes `CHANGELOG.md` with git-cliff from `cliff.toml`.
3. Commit `chore: release vX.Y.Z` and open the pull request. Keep that commit last: the merge
   publishes and tags the commit which set the version.

On that pull request the `release` check refuses a branch which names another version, a version
which does not follow the last tag, and an API break the version may not carry against the last
tag. The `deep` checks run too: Miri, the careful build, the minimal versions and the
links.

## What a merge does

`release.yml` runs on every push to `main` which touches `Cargo.toml`. When the manifest carries a
version no tag names, it publishes the crate through crates.io trusted publishing, tags the
commit which set the version and creates the GitHub release with the notes git-cliff writes for
that version. A rerun through `workflow_dispatch` skips a publish which went through, so it
finishes a release that stopped half way. `just release-plan` prints what `main`
would publish now.

While `main` carries a version no tag names, every pull request fails `release`. Rerun
`release.yml` on `main` through `workflow_dispatch` before anything else merges.

## What the repository needs

- trusted publishing on crates.io for `line-discipline`, naming the `eliosai/line-discipline`
  repository and the `release.yml` workflow, so the job holds no crates.io secret
- the organization rulesets: `main` takes rebase-merged pull requests which pass `gate`, and a
  `v*` tag never moves or goes away

## Pull requests

The `semver` check compares the public API against the base branch and fails a breaking change
unless the pull request carries the `semver-major` label. The `size` check counts added lines
outside `Cargo.lock`, `CHANGELOG.md`, `.agents/skills` and `tests/kernel/cases.txt`, with a
budget of 1000, 3000 under the `mechanical` label, and none under `size-exempt`.
