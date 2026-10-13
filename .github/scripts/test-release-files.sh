#!/usr/bin/env bash
# Exercise real git-cliff/Cargo commands without touching the project checkout.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
xtask=${XTASK_BIN:-$repo/target/debug/xtask}
files="$repo/.github/scripts/release-files.sh"
test_root=$(mktemp -d)
export RUNNER_TEMP="$test_root/output"
export GITHUB_OUTPUT="$test_root/outputs"
export GIT_CLIFF_OFFLINE=true
mkdir -p "$RUNNER_TEMP" "$test_root/repo/crates/maa-cli/src"
cd "$test_root/repo"
git init -q -b main
git config user.name 'Release Test'
git config user.email 'release@example.invalid'
git config commit.gpgsign false
cp "$repo/cliff.toml" .
printf '[workspace]\nmembers = ["crates/maa-cli"]\nresolver = "2"\n' > Cargo.toml
printf '[package]\nname = "maa-cli"\nversion = "0.7.5"\nedition = "2024"\n' > crates/maa-cli/Cargo.toml
printf 'pub fn example() {}\n' > crates/maa-cli/src/lib.rs
printf '# Release Notes\n\n## Release 0.7.5\n\nPrevious release\n' > CHANGELOG.md
cargo generate-lockfile --offline
git add .
git commit -qm 'chore: initial'
git tag v0.7.5

select_version() {
  : > "$GITHUB_OUTPUT"
  "$xtask" release select-version --version "$1" > "$RUNNER_TEMP/select.log"
  VERSION=$(sed -n 's/^version=//p' "$GITHUB_OUTPUT")
  PREVIOUS_TAG=$(sed -n 's/^previous_tag=//p' "$GITHUB_OUTPUT")
  export VERSION PREVIOUS_TAG
}

git commit -q --allow-empty -m 'docs: clarify usage'
select_version auto
[[ "$VERSION" == 0.7.5 ]]
git commit -q --allow-empty -m 'fix: repair A'
# Unconventional subjects must not block version selection or changelogs.
git commit -q --allow-empty -m 'Revert "feat: experiment"' -m 'This reverts commit 1234567.'
git commit -q --allow-empty -m 'Update README.md'
select_version auto
[[ "$VERSION" == 0.7.6 ]]
git checkout -qb release-prep/v0.7.6
bash "$files" prepare
bash "$files" check
git add .
git commit -qm 'chore(release): prepare for v0.7.6'
prepared_sha=$(git rev-parse HEAD)
grep -q '^- Feat: experiment$' CHANGELOG.md

readiness() {
  EVENT_NAME=$1 LABELED=${2:-} HEAD_REF=${3:-} BASE_SHA=$(git rev-parse main) HEAD_SHA=$(git rev-parse HEAD) \
    XTASK="$xtask" bash "$repo/.github/scripts/release-readiness.sh" > "$RUNNER_TEMP/readiness.log" 2>&1
}
readiness pull_request true release-prep/v0.7.6
if readiness pull_request false release-prep/v0.7.6; then
  echo 'Expected an unlabeled release-prep PR to fail' >&2
  exit 1
fi
grep -q 'no release label' "$RUNNER_TEMP/readiness.log"
readiness pull_request false feature/other
git checkout -q --detach
printf 'unrelated\n' > unrelated.txt
git add unrelated.txt
git commit -qm 'chore(release): prepare for v0.7.6'
if readiness pull_request true release-prep/v0.7.6; then
  echo 'Expected an unrelated file in a release PR to fail' >&2
  exit 1
fi
grep -q 'Unexpected file in release PR: unrelated.txt' "$RUNNER_TEMP/readiness.log"
# Merge queue entries: the squashed release is checked; entries behind it are not.
git checkout -q --detach main
git merge -q --squash "$prepared_sha" > /dev/null
git commit -qm 'chore(release): prepare for v0.7.6 (#9)'
readiness merge_group
git commit -q --allow-empty -m 'fix: queued behind release'
readiness merge_group
grep -q 'Not a release queue entry' "$RUNNER_TEMP/readiness.log"
# A releasable change queued ahead of the release makes it stale.
git checkout -q --detach main
git commit -q --allow-empty -m 'fix: queued ahead of release'
git merge -q --squash "$prepared_sha" > /dev/null
git commit -qm 'chore(release): prepare for v0.7.6 (#9)'
if readiness merge_group; then
  echo 'Expected a release behind newer fixes to fail in the merge queue' >&2
  exit 1
fi
grep -q 'Release PR is stale' "$RUNNER_TEMP/readiness.log"
git checkout -q release-prep/v0.7.6
bash "$files" notes 0.7.6 > "$RUNNER_TEMP/before-tag.md"
git tag v0.7.6
bash "$files" notes 0.7.6 > "$RUNNER_TEMP/after-tag.md"
cmp "$RUNNER_TEMP/before-tag.md" "$RUNNER_TEMP/after-tag.md"
git tag -d v0.7.6 > /dev/null

# A stale PR must be refreshable even after its version files reached main.
git checkout -q main
git commit -q --allow-empty -m 'fix: repair B'
git cherry-pick "$prepared_sha" > /dev/null
if bash "$files" check > "$RUNNER_TEMP/stale.log" 2>&1; then
  echo 'Expected stale release notes to fail' >&2
  exit 1
fi
select_version auto
[[ "$VERSION" == 0.7.6 ]]
bash "$files" prepare
[[ "$(git diff --name-only)" == CHANGELOG.md ]]
bash "$files" check
[[ "$(grep -c '^## Release 0.7.6$' CHANGELOG.md)" == 1 ]]
grep -q 'Repair B' CHANGELOG.md
git add CHANGELOG.md
git commit -qm 'chore(release): prepare for v0.7.6'

# A normal version-only PR can be followed by a changelog-only release PR.
git checkout -qb manual-bump v0.7.5
sed -i.bak 's/0.7.5/0.8.0/' crates/maa-cli/Cargo.toml
mv crates/maa-cli/Cargo.toml.bak "$RUNNER_TEMP/old-manifest"
cargo update -p maa-cli --precise 0.8.0 --offline
git add .
git commit -qm 'chore: bump maa-cli to 0.8.0'
select_version auto
[[ "$VERSION" == 0.8.0 ]]
bash "$files" prepare
[[ "$(git diff --name-only)" == CHANGELOG.md ]]
bash "$files" check
select_version 0.9.0
[[ "$VERSION" == 0.9.0 ]]
select_version patch
[[ "$VERSION" == 0.7.6 ]]

# The beta CLI reads its published index but must not write source files.
mkdir version
git init -q version
git -C version config user.name 'Release Test'
git -C version config user.email 'release@example.invalid'
git -C version config commit.gpgsign false
for channel in stable beta alpha; do
  jq -n --arg channel "$channel" \
    '{version:"0.7.5",details:{tag:"v0.7.5",commit:"old",assets:{}}}' > "version/$channel.json"
done
git -C version add .
git -C version commit -qm 'chore: published index'
git diff > "$RUNNER_TEMP/before-beta.diff"
commit=$(git rev-parse HEAD)
: > "$GITHUB_OUTPUT"
"$xtask" release meta --channel beta --commit "$commit" --version 0.8.0 --publish
grep -qx 'version=0.8.0-beta.1' "$GITHUB_OUTPUT"
git diff > "$RUNNER_TEMP/after-beta.diff"
cmp "$RUNNER_TEMP/before-beta.diff" "$RUNNER_TEMP/after-beta.diff"
git -C version diff --exit-code

# A stable tag may already exist while its version-index job still needs a retry.
git tag v0.8.0
: > "$GITHUB_OUTPUT"
"$xtask" release meta --channel beta --commit "$commit" --version auto --publish
grep -qx 'skip=true' "$GITHUB_OUTPUT"

# Publishing nightly without its index must block the next allocation, then recover.
mkdir -p "$test_root/alpha/crates/maa-cli" "$test_root/bin"
cd "$test_root/alpha"
git init -q -b main
git config user.name 'Release Test'
git config user.email 'release@example.invalid'
git config commit.gpgsign false
printf '[package]\nname = "maa-cli"\nversion = "0.8.0"\n' > crates/maa-cli/Cargo.toml
git add crates
git commit -qm 'chore: initial'
git tag v0.7.5
mkdir -p version release-bundle/version
printf '{"version":"0.7.5","details":{"tag":"v0.7.5","commit":"old","assets":{}}}' > version/stable.json
printf '{"version":"0.8.0-alpha.1+sha.old","details":{"tag":"nightly","commit":"old","assets":{}}}' > version/alpha.json
commit=$(git rev-parse HEAD)
: > "$GITHUB_OUTPUT"
"$xtask" release meta --channel alpha --commit "$commit" --version 0.8.0 --publish
export TEST_NIGHTLY_VERSION
TEST_NIGHTLY_VERSION=$(sed -n 's/^version=//p' "$GITHUB_OUTPUT")
[[ "$TEST_NIGHTLY_VERSION" == 0.8.0-alpha.2+sha.* ]]
jq -n --arg version "$TEST_NIGHTLY_VERSION" --arg commit "$commit" \
  '{version:$version,details:{tag:"nightly",commit:$commit,assets:{}}}' > release-bundle/version/alpha.json
git tag nightly
cat > "$test_root/bin/gh" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'api repos/{owner}/{repo}/releases/tags/nightly' ]]
if [[ "${TEST_NIGHTLY_DENIED:-false}" == true ]]; then exit 1; fi
jq -n --arg name "v$TEST_NIGHTLY_VERSION" '{name:$name}'
MOCK
chmod +x "$test_root/bin/gh"
export PATH="$test_root/bin:$PATH"
git commit -q --allow-empty -m 'fix: next nightly'
next_commit=$(git rev-parse HEAD)
: > "$GITHUB_OUTPUT"
if "$xtask" release meta --channel alpha --commit "$next_commit" --version 0.8.0 --publish > "$RUNNER_TEMP/alpha-stale.log" 2>&1; then
  echo 'Expected unindexed nightly to block the next allocation' >&2
  exit 1
fi
grep -q 'rerun its index job' "$RUNNER_TEMP/alpha-stale.log"
[[ ! -s "$GITHUB_OUTPUT" ]]
CHANNEL=alpha VERSION="$TEST_NIGHTLY_VERSION" TAG=nightly COMMIT="$commit" "$xtask" release index
"$xtask" release meta --channel alpha --commit "$next_commit" --version 0.8.0 --publish
grep -q '^version=0.8.0-alpha.3+sha\.' "$GITHUB_OUTPUT"
if TEST_NIGHTLY_DENIED=true "$xtask" release meta --channel alpha --commit "$next_commit" --version 0.8.0 --publish > "$RUNNER_TEMP/alpha-denied.log" 2>&1; then
  echo 'Expected an unreadable nightly release to block allocation' >&2
  exit 1
fi
# Packaging produces a complete bundle without rewriting its index input.
mkdir maa_cli-x86_64-unknown-linux-gnu
printf 'fixture binary\n' > "$test_root/bin/maa"
tar -cf maa_cli-x86_64-unknown-linux-gnu/x86_64-unknown-linux-gnu.tar -C "$test_root/bin" maa
printf 'fixture license\n' > licenses.md
cp version/alpha.json "$RUNNER_TEMP/index-before-package.json"
CHANNEL=alpha VERSION="$TEST_NIGHTLY_VERSION" TAG=nightly COMMIT="$commit" "$xtask" release package
cmp version/alpha.json "$RUNNER_TEMP/index-before-package.json"
asset=$(jq -r '.details.assets["x86_64-unknown-linux-gnu"].name' release-bundle/version/alpha.json)
[[ -f "release-bundle/$asset" && ! -f "$asset" ]]
[[ -f release-bundle/version/alpha.txt ]]
tar -tzf "release-bundle/$asset" | grep -qx maa
echo "Release file, prerelease recovery and packaging checks passed (fixtures: $test_root)"
