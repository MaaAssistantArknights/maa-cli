#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
install_script=${1:-"$repo_root/install.sh"}
test_dir=$(mktemp -d)
cleanup() {
    if [[ "$(uname -s)" == Darwin ]]; then
        /usr/bin/trash --stopOnError "$test_dir"
    else
        rm -rf "$test_dir"
    fi
}
trap cleanup EXIT

mkdir -p "$test_dir/bin" "$test_dir/archive"
cat > "$test_dir/archive/maa" <<'EOF'
#!/usr/bin/env bash
set -eu
[[ "$#" == 1 && "$1" == --version ]]
if [[ "${TEST_STARTUP_STATUS:-0}" != 0 ]]; then
    printf '%s\n' "${TEST_STARTUP_ERROR:-startup failed}" >&2
    exit "$TEST_STARTUP_STATUS"
fi
printf '%s\n' 'maa 0.7.6'
EOF
chmod 644 "$test_dir/archive/maa"
tar -czf "$test_dir/maa.tar.gz" -C "$test_dir/archive" maa

case "$(uname -s)" in
    Linux) target="$(uname -m)-unknown-linux-gnu" ;;
    Darwin) target="$(uname -m)-apple-darwin" ;;
esac
target=${target/arm64/aarch64}
target_var=$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')
if command -v sha256sum > /dev/null 2>&1; then
    checksum=$(sha256sum "$test_dir/maa.tar.gz" | awk '{print $1}')
else
    checksum=$(shasum -a 256 "$test_dir/maa.tar.gz" | awk '{print $1}')
fi
printf 'VERSION=0.7.6\n%s_NAME=maa.tar.gz\n%s_SHA256=%s\n' \
    "$target_var" "$target_var" "$checksum" > "$test_dir/manifest.txt"

cat > "$test_dir/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == -fsSL ]]; then
    cat "$TEST_MANIFEST"
else
    [[ "$1" == -fL && "$2" == -o ]]
    cp "$TEST_ARCHIVE" "$3"
fi
EOF
chmod 755 "$test_dir/bin/curl"
export PATH="$test_dir/bin:$PATH"
export TEST_MANIFEST="$test_dir/manifest.txt"
export TEST_ARCHIVE="$test_dir/maa.tar.gz"
export MAA_CHANNEL=stable

export MAA_INSTALL_DIR="$test_dir/successful install"
bash "$install_script" > "$test_dir/success.log" 2>&1
[[ "$("$MAA_INSTALL_DIR/maa" --version)" == 'maa 0.7.6' ]]
printf '%s\n' 'PASS: runnable download is installed even without archive execute permissions'

export TEST_STARTUP_STATUS=127
export TEST_STARTUP_ERROR="maa: /lib/libc.so.6: version 'GLIBC_2.39' not found"
export MAA_INSTALL_DIR="$test_dir/existing install"
mkdir -p "$MAA_INSTALL_DIR"
printf '%s\n' 'existing maa' > "$MAA_INSTALL_DIR/maa"
cp "$MAA_INSTALL_DIR/maa" "$test_dir/previous-maa"
if bash "$install_script" > "$test_dir/failure.log" 2>&1; then
    printf '%s\n' 'FAIL: installer accepted an incompatible download' >&2
    exit 1
fi
cmp "$test_dir/previous-maa" "$MAA_INSTALL_DIR/maa"
grep -Fq "$TEST_STARTUP_ERROR" "$test_dir/failure.log"
grep -Fq 'Downloaded maa-cli could not run on this system.' "$test_dir/failure.log"
grep -Fq 'Installation aborted; existing installation was not changed.' "$test_dir/failure.log"
printf '%s\n' 'PASS: startup failure reports the original error and preserves the existing installation'

export MAA_INSTALL_DIR="$test_dir/failed fresh install"
if bash "$install_script" > "$test_dir/fresh-failure.log" 2>&1; then
    printf '%s\n' 'FAIL: fresh installation accepted an incompatible download' >&2
    exit 1
fi
[[ ! -e "$MAA_INSTALL_DIR" ]]
printf '%s\n' 'PASS: startup failure does not create a fresh installation directory'
