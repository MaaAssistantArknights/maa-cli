#!/usr/bin/env bash
set -euo pipefail

# Enable debug mode if DEBUG environment variable is set
if [ "${DEBUG:-}" = "1" ]; then
    set -x
fi

# Default values
CHANNEL="${MAA_CHANNEL:-stable}"
INSTALL_DIR="${MAA_INSTALL_DIR:-$HOME/.local/bin}"
REPO="MaaAssistantArknights/maa-cli"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

info() {
    echo -e "${GREEN}$1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}$1${NC}" >&2
}

# Detect platform and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="apple-darwin" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)       error "Unsupported architecture: $(uname -m)" ;;
    esac

    # Determine full target triple
    if [ "$os" = "linux" ]; then
        echo "${arch}-unknown-${os}-gnu"
    else
        echo "${arch}-${os}"
    fi
}

# Fetch version info from GitHub (shell-friendly .txt format)
fetch_version_info() {
    local channel=$1
    local url="https://raw.githubusercontent.com/${REPO}/version/${channel}.txt"

    info "Fetching version information from ${channel}"

    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$url"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download and verify file
download_and_verify() {
    local url=$1
    local output=$2
    local expected_hash=$3

    info "Downloading from $url..."

    if command -v curl > /dev/null 2>&1; then
        curl -fL -o "$output" "$url"
    elif command -v wget > /dev/null 2>&1; then
        wget -O "$output" "$url"
    else
        error "Neither curl nor wget found."
    fi

    info "Verifying checksum..."
    local actual_hash
    if command -v sha256sum > /dev/null 2>&1; then
        actual_hash=$(sha256sum "$output" | awk '{print $1}')
    elif command -v shasum > /dev/null 2>&1; then
        actual_hash=$(shasum -a 256 "$output" | awk '{print $1}')
    else
        warn "No SHA256 tool found, skipping verification"
        return 0
    fi

    if [ "$actual_hash" != "$expected_hash" ]; then
        error "Checksum verification failed!\nExpected: $expected_hash\nActual:   $actual_hash"
    fi

    info "Checksum verified successfully"
}

# Extract archive
extract_archive() {
    local archive=$1
    local dest_dir=$2

    info "Extracting archive..."

    case "$archive" in
        *.tar.gz)
            tar -xzf "$archive" -C "$dest_dir"
            ;;
        *.zip)
            unzip -q "$archive" -d "$dest_dir"
            ;;
        *)
            error "Unsupported archive format: $archive"
            ;;
    esac
}

# Main installation function
main() {
    local target version archive_name download_url sha256sum

    # Detect platform
    target=$(detect_platform)
    info "Detected platform: $target"

    # Fetch version information
    local version_info
    version_info=$(fetch_version_info "$CHANNEL") || error "Failed to fetch version information"

    # Source the version info (it's in shell variable format)
    eval "$version_info"

    # Get asset info for the detected platform
    local target_var="${target//-/_}"
    target_var=$(echo "$target_var" | tr '[:lower:]' '[:upper:]')  # Convert to uppercase

    # Extract variables for this target
    eval "archive_name=\$${target_var}_NAME"
    eval "sha256sum=\$${target_var}_SHA256"

    if [ -z "$archive_name" ]; then
        error "No release found for platform: $target"
    fi

    # shellcheck disable=SC2153
    version="$VERSION"

    info "Version: $version"
    info "Archive: $archive_name"

    # Construct download URL
    download_url="https://github.com/${REPO}/releases/download/v${version}/${archive_name}"

    # Create temporary directory
    tmp_dir=$(mktemp -d)
    # shellcheck disable=SC2064
    # We want use the current and not the future values,
    trap "rm -rf '$tmp_dir'" EXIT

    # Download and verify
    download_and_verify "$download_url" "${tmp_dir}/${archive_name}" "$sha256sum"

    # Extract archive
    extract_archive "${tmp_dir}/${archive_name}" "$tmp_dir"

    # Find the binary (it might be in a subdirectory)
    local binary_path
    binary_path=$(find "$tmp_dir" -name "maa" -type f | head -n 1)

    if [ -z "$binary_path" ]; then
        error "Binary not found in extracted archive"
    fi

    # Install binary
    mkdir -p "$INSTALL_DIR"
    install -m 755 "$binary_path" "${INSTALL_DIR}/maa"

    info "Successfully installed maa to ${INSTALL_DIR}/maa"

    # Check if in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        warn "Warning: ${INSTALL_DIR} is not in your PATH"
        warn "Add it by running: export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi
}

# Show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Install maa-cli from GitHub releases.

OPTIONS:
    -h, --help              Show this help message
    -c, --channel CHANNEL   Specify release channel (stable, beta, alpha) [default: stable]
    -d, --dir DIRECTORY     Installation directory [default: \$HOME/.local/bin]

ENVIRONMENT VARIABLES:
    MAA_CHANNEL             Release channel (same as --channel)
    MAA_INSTALL_DIR         Installation directory (same as --dir)

EXAMPLES:
    # Install stable release to default location
    $0

    # Install beta release
    $0 --channel beta

    # Install to custom directory
    $0 --dir /usr/local/bin

    # Using environment variables
    MAA_CHANNEL=alpha MAA_INSTALL_DIR=/opt/bin $0

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -c|--channel)
            CHANNEL="$2"
            shift 2
            ;;
        -d|--dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        *)
            error "Unknown option: $1\nUse --help for usage information"
            ;;
    esac
done

# Validate channel
case "$CHANNEL" in
    stable|beta|alpha) ;;
    *) error "Invalid channel: $CHANNEL (must be stable, beta, or alpha)" ;;
esac

main
