#!/usr/bin/env bash
# Install the development tools used by CI and `cargo x` in Claude Code cloud
# sessions. Safe to run repeatedly: tools that are already installed are skipped.
set -euo pipefail

if [[ "${CLAUDE_CODE_REMOTE:-}" != "true" ]]; then
  exit 0
fi

# SessionStart hook stdout is added to the session context; keep logs on stderr.
exec 1>&2

# Bump these together with the versions CI is expected to run.
CARGO_DENY_VERSION="0.20.2"
TYPOS_VERSION="1.51.1"

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
export PATH="$CARGO_BIN:$PATH"
TARGET="$(uname -m)-unknown-linux-musl"

log() {
  echo "[session-start] $*" >&2
}

# Install a binary from a prebuilt GitHub release tarball. Falls back to
# `cargo install` when the download fails, which is slower but always works.
#   install_release <bin> <url> <crate> [version]
install_release() {
  local bin="$1" url="$2" crate="$3" version="${4:-}"
  local tmp
  tmp="$(mktemp -d)"
  log "Installing $bin from $url"
  if curl -fsSL --retry 3 "$url" | tar -xz -C "$tmp"; then
    install -m 755 "$(find "$tmp" -type f -name "$bin" | head -n 1)" "$CARGO_BIN/$bin"
  else
    log "Download failed, building $crate from source"
    cargo install --locked ${version:+--version "$version"} "$crate"
  fi
  rm -rf "$tmp"
}

# Nightly toolchain: rustfmt for `cargo +nightly fmt`, llvm-tools for
# `cargo +nightly llvm-cov`, clippy for nightly lint checks.
log "Installing nightly toolchain"
rustup toolchain install nightly --profile minimal --no-self-update \
  --component rustfmt,clippy,llvm-tools

# Stable components used by `cargo x test --clippy`.
rustup component add --toolchain stable rustfmt clippy

# Coverage, same as CI (always the latest release).
if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  install_release cargo-llvm-cov \
    "https://github.com/taiki-e/cargo-llvm-cov/releases/latest/download/cargo-llvm-cov-$TARGET.tar.gz" \
    cargo-llvm-cov
fi

# Dependency audit (`cargo deny check`).
if ! cargo-deny --version 2>/dev/null | grep -qF "$CARGO_DENY_VERSION"; then
  install_release cargo-deny \
    "https://github.com/EmbarkStudios/cargo-deny/releases/download/$CARGO_DENY_VERSION/cargo-deny-$CARGO_DENY_VERSION-$TARGET.tar.gz" \
    cargo-deny "$CARGO_DENY_VERSION"
fi

# Spell check (`typos`).
if ! typos --version 2>/dev/null | grep -qF "$TYPOS_VERSION"; then
  install_release typos \
    "https://github.com/crate-ci/typos/releases/download/v$TYPOS_VERSION/typos-v$TYPOS_VERSION-$TARGET.tar.gz" \
    typos-cli "$TYPOS_VERSION"
fi

# Markdown linter for docs.
if ! command -v markdownlint-cli2 >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  log "Installing markdownlint-cli2"
  npm install --global --no-fund --no-audit --loglevel=error markdownlint-cli2
fi

log "Done"
