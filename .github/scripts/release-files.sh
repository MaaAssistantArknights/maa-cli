#!/usr/bin/env bash
# Source-file operations shared by release preparation and its PR check.
set -euo pipefail

package_version() {
  cargo metadata --locked --no-deps --format-version 1 |
    jq -r '.packages[] | select(.name == "maa-cli") | .version'
}

release_notes() {
  awk -v heading="## Release $1" '
    /^## / { if (found) exit; if ($0 == heading) found = 1 }
    found { print }
    END { if (!found) exit 1 }
  ' CHANGELOG.md
}

case "${1:?expected prepare, check, notes, or prerelease-notes}" in
  prepare)
    : "${VERSION:?}" "${PREVIOUS_TAG:?}"
    current_version=$(package_version)
    if [[ "$current_version" != "$VERSION" ]]; then
      # Versions have already been parsed and validated by release select-version.
      sed -i.bak "s/^version = \"$current_version\"$/version = \"$VERSION\"/" \
        crates/maa-cli/Cargo.toml
      mv crates/maa-cli/Cargo.toml.bak "$RUNNER_TEMP/Cargo.toml.before-release"
      cargo update -p maa-cli --precise "$VERSION" --offline
    fi
    [[ "$(package_version)" == "$VERSION" ]]

    # A previous preparation may already be on main without a published tag.
    # Remove only its section; git-cliff owns generation and prepending.
    awk -v heading="## Release $VERSION" '
      /^## / { skip = ($0 == heading) }
      !skip { print }
    ' CHANGELOG.md > "$RUNNER_TEMP/changelog-base.md"
    mv "$RUNNER_TEMP/changelog-base.md" CHANGELOG.md
    git-cliff --config cliff.toml --tag "v$VERSION" \
      --prepend CHANGELOG.md "$PREVIOUS_TAG..$(git rev-parse HEAD)"

    # A separately bumped Cargo version leaves only CHANGELOG.md to change.
    git diff --name-only | while IFS= read -r file; do
      case "$file" in
        CHANGELOG.md|Cargo.lock|crates/maa-cli/Cargo.toml) ;;
        *) echo "Unexpected release preparation change: $file" >&2; exit 1 ;;
      esac
    done
    if git diff --quiet; then
      echo 'Release files already match; no preparation PR is needed.' >&2
      exit 1
    fi
    ;;
  check)
    : "${PREVIOUS_TAG:?}"
    version=$(package_version)
    release_notes "$version" > "$RUNNER_TEMP/actual-release-notes.md"
    git-cliff --config cliff.toml --tag "v$version" --strip all \
      --output "$RUNNER_TEMP/expected-release-notes.md" \
      "$PREVIOUS_TAG..$(git rev-parse HEAD)"
    perl -0pi -e 's/\s+\z/\n/' "$RUNNER_TEMP/"{actual,expected}-release-notes.md
    if ! diff -u "$RUNNER_TEMP/"{actual,expected}-release-notes.md; then
      echo 'Release PR is stale. Run Prepare Stable Release again before merging.' >&2
      exit 1
    fi
    ;;
  notes)
    release_notes "${2:?expected version}"
    ;;
  prerelease-notes)
    : "${VERSION:?}" "${TAG:?}"
    # Use an explicit range: on a dev branch that merges main, git-cliff's
    # --unreleased also lists fixes already published in the merged stable tags.
    previous=$(git tag --merged HEAD --list 'v*' |
      awk '/^v[0-9]+\.[0-9]+\.[0-9]+$/' | sort -V | tail -n 1)
    if [[ -z "$previous" ]]; then
      echo 'No reachable stable vX.Y.Z tag found' >&2
      exit 1
    fi
    git-cliff --config cliff.toml --strip all "$previous..$(git rev-parse HEAD)"
    ;;
  *) echo "Unknown operation: $1" >&2; exit 1 ;;
esac
