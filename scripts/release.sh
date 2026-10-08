#!/usr/bin/env bash
# Prepares a release commit on a new branch: picks the next version from the Conventional Commits
# since the last tag, prepends its section to CHANGELOG.md, and sets it in Cargo.toml and Cargo.lock.
# Needs git-cliff (https://git-cliff.org) and cargo.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree is not clean." >&2
  exit 1
fi

tag=$(git cliff --bumped-version 2>/dev/null)
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "No releasable commits since $tag." >&2
  exit 1
fi
version=${tag#v}

git switch -c "release/$tag"
git cliff --unreleased --bump --prepend CHANGELOG.md 2>/dev/null
perl -pi -e 's/^version = "[^"]*"/version = "'"$version"'"/ if !$done && /^version = / && ($done = 1)' Cargo.toml
cargo update --workspace --quiet
git commit -q -am "chore(release): $tag"

echo "Created branch release/$tag. Review CHANGELOG.md (amend if needed), then push and open a PR."
echo "After merge: git switch main && git pull && git tag -a $tag -m $tag && git push origin $tag"
