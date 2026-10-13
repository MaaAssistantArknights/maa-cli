#!/usr/bin/env bash
# Decide whether a PR or merge-queue entry carries a release, then verify its files.
set -euo pipefail
: "${EVENT_NAME:?}" "${RUNNER_TEMP:?}"
files="$(cd "$(dirname "$0")" && pwd)/release-files.sh"
read -ra xtask <<< "${XTASK:-cargo x --}"

case "$EVENT_NAME" in
  pull_request)
    : "${LABELED:?}" "${HEAD_REF:?}" "${BASE_SHA:?}" "${HEAD_SHA:?}"
    if [[ "$LABELED" != true ]]; then
      if [[ "$HEAD_REF" == release-prep/* ]]; then
        echo 'This release-prep PR has no release label; merging it would not publish.' >&2
        exit 1
      fi
      echo 'Not a release PR.'
      exit 0
    fi
    changed=$(git diff --name-only "$BASE_SHA...$HEAD_SHA")
    ;;
  merge_group)
    # Each queue entry is checked at its own head. Later entries contain earlier
    # releases, so only the entry that lands the release commit is checked.
    subject=$(git log -1 --format=%s HEAD)
    prepare='^chore\(release\): prepare for v'
    merge='^Merge pull request #[0-9]+ from [^ ]+/release-prep/v'
    if [[ ! "$subject" =~ $prepare && ! "$subject" =~ $merge ]]; then
      echo 'Not a release queue entry.'
      exit 0
    fi
    changed=$(git diff --name-only HEAD^1 HEAD)
    ;;
  *) echo "Unsupported event: $EVENT_NAME" >&2; exit 1 ;;
esac

while IFS= read -r file; do
  case "$file" in
    CHANGELOG.md|Cargo.lock|crates/maa-cli/Cargo.toml|'') ;;
    *) echo "Unexpected file in release PR: $file" >&2; exit 1 ;;
  esac
done <<< "$changed"

: > "$RUNNER_TEMP/readiness-candidate"
GITHUB_OUTPUT="$RUNNER_TEMP/readiness-candidate" "${xtask[@]}" release select-version
PREVIOUS_TAG=$(sed -n 's/^previous_tag=//p' "$RUNNER_TEMP/readiness-candidate") \
  bash "$files" check
