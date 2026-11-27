#!/usr/bin/env bash
# One version names the crate: bumps it, judges a release pull request and plans what main publishes
# Usage:
#   scripts/release.sh bump <X.Y.Z>           write the version into the manifest, the lockfile and CHANGELOG.md
#   scripts/release.sh check <branch> <head>  a release pull request names its version and breaks no API that version may not break
#   scripts/release.sh plan                   version=<X.Y.Z> and commit=<sha that set it> when HEAD carries a version no tag names, else version=
#   scripts/release.sh notes <X.Y.Z>          the release notes of the version main is about to tag
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
readonly root

# the version the crate carries
current() {
    sed -nE '/^\[package\]/,/^\[/ s/^version = "([^"]+)"/\1/p' "$root/Cargo.toml"
}

# the newest release tag, or nothing before the first release
previous() {
    git -C "$root" tag --list 'v[0-9]*' --sort=-version:refname | head -n 1
}

# fails unless the version is X.Y.Z and above every release
fresh() {
    local version="$1" last newest
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "'$version' is not X.Y.Z" >&2; exit 2; }
    last="$(previous)"
    [ -n "$last" ] || return 0
    newest="$(printf '%s\n' "${last#v}" "$version" | sort -V | tail -n 1)"
    [ "$newest" = "$version" ] && [ "v$version" != "$last" ] || { echo "$version does not follow $last" >&2; exit 1; }
}

bump() {
    local version="$1" old
    fresh "$version"
    # the fetch fills the cache before any manifest moves, so the offline update below only moves the crate's own entry
    cargo fetch --manifest-path "$root/Cargo.toml" --locked
    old="$(current)"
    old="${old//./\\.}"
    sed -i -E "/^\[package\]/,/^\[/ s/^version = \"$old\"/version = \"$version\"/" "$root/Cargo.toml"
    [ "$(current)" = "$version" ] || { echo "the manifest did not take $version" >&2; exit 1; }
    # the README's dependency line names the major
    sed -i -E "s/^(line-discipline = \")[0-9]+\"/\1${version%%.*}\"/" "$root/README.md"
    cargo update --manifest-path "$root/Cargo.toml" --workspace --offline
    (cd "$root" && git-cliff --tag "v$version" --output CHANGELOG.md)
}

# the release publishes and tags the commit which set the version, even on a later rerun
shipped() {
    local version="$1" commit
    commit="$(git -C "$root" log -1 --format=%H -G "^version = \"${version//./\\.}\"" -- Cargo.toml)"
    [ -n "$commit" ] || { echo "no commit sets $version" >&2; exit 1; }
    echo "$commit"
}

check() {
    local branch="$1" head="$2" version last
    version="$(current)"
    [ "$branch" = "release/v$version" ] || { echo "$branch releases $version: name it release/v$version" >&2; exit 1; }
    fresh "$version"
    [ "$(shipped "$version")" = "$head" ] || { echo "the release commit is not the head of $branch: keep it last" >&2; exit 1; }
    last="$(previous)"
    [ -n "$last" ] || { echo "v$version is the first release: no API to compare"; return 0; }
    cargo semver-checks --manifest-path "$root/Cargo.toml" --all-features --baseline-rev "$last"
}

plan() {
    local version commit
    version="$(current)"
    if git -C "$root" rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
        [ "v$version" = "$(previous)" ] || { echo "$version is older than $(previous)" >&2; exit 1; }
        echo "version="
    else
        fresh "$version"
        echo "version=$version"
        commit="$(shipped "$version")"
        echo "commit=$commit"
    fi
}

notes() {
    (cd "$root" && git-cliff --tag "v$1" --unreleased --strip all)
}

case "${1:-}" in
    bump) [ $# -eq 2 ] || exit 2; bump "$2" ;;
    check) [ $# -eq 3 ] || exit 2; check "$2" "$3" ;;
    plan) plan ;;
    notes) [ $# -eq 2 ] || exit 2; notes "$2" ;;
    *) sed -n '2,7p' "${BASH_SOURCE[0]}" >&2; exit 2 ;;
esac
