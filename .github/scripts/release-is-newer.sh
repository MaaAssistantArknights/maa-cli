#!/usr/bin/env bash
# Exit 0 when stable version $1 is newer than packaged version $2.
# Used to avoid moving a beta package back to an older stable release.
set -euo pipefail
stable=${1:?expected stable version}
current=${2:?expected current version}
core() { grep -oE '^[0-9]+\.[0-9]+\.[0-9]+' <<< "$1"; }
stable_core=$(core "$stable")
current_core=$(core "$current")
if [[ "$stable_core" == "$current_core" ]]; then
  # The stable release supersedes a prerelease of the same version.
  [[ "$current" != "$current_core" ]]
  exit
fi
[[ "$(printf '%s\n%s\n' "$stable_core" "$current_core" | sort -V | tail -n 1)" == "$stable_core" ]]
