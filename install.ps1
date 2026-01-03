#Requires -Version 5.1

<#
.SYNOPSIS
    Install maa-cli from GitHub releases.

.DESCRIPTION
    This script downloads and installs maa-cli from GitHub releases.
    It automatically detects the platform and architecture, fetches the
    appropriate release, verifies the checksum, and installs the binary.

.PARAMETER Channel
    The release channel to install from (stable, beta, or alpha).
    Default: stable

.PARAMETER InstallDir
    The directory to install the binary to.
    Default: $env:LOCALAPPDATA\Programs\maa-cli

.EXAMPLE
    .\install.ps1
    Install the stable release to the default location.

.EXAMPLE
    .\install.ps1 -Channel beta
    Install the beta release.

.EXAMPLE
    .\install.ps1 -InstallDir "C:\Tools"
    Install to a custom directory.

.EXAMPLE
    $env:MAA_CHANNEL = "alpha"; .\install.ps1
    Install using environment variable.
#>

[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet("stable", "beta", "alpha")]
    [string]$Channel = $(if ($env:MAA_CHANNEL) { $env:MAA_CHANNEL } else { "stable" }),

    [Parameter()]
    [string]$InstallDir = $(
        if ($env:MAA_INSTALL_DIR) {
            $env:MAA_INSTALL_DIR
        } elseif ($env:LOCALAPPDATA) {
            "$env:LOCALAPPDATA\Programs\maa-cli"
        } else {
            "$HOME/.local/bin"
        }
    )
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$REPO = "MaaAssistantArknights/maa-cli"

# Color output functions
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$ForegroundColor = "White"
    )
    Write-Host $Message -ForegroundColor $ForegroundColor
}

function Write-Error-Msg {
    param([string]$Message)
    Write-ColorOutput "Error: $Message" -ForegroundColor Red
    exit 1
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput $Message -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-ColorOutput $Message -ForegroundColor Yellow
}

# Detect platform and architecture
function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE

    # For testing on macOS/Linux, use a mock value
    if (-not $arch) {
        if ($IsMacOS -or $IsLinux) {
            Write-Warn "Running on non-Windows system. Using x86_64-pc-windows-msvc for testing."
            return "x86_64-pc-windows-msvc"
        }
        Write-Error-Msg "Unable to detect processor architecture"
    }

    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        default { Write-Error-Msg "Unsupported architecture: $arch" }
    }
}

# Fetch version info from GitHub (shell-friendly .txt format)
function Get-VersionInfo {
    param([string]$Channel)

    $url = "https://raw.githubusercontent.com/$REPO/version/$Channel.txt"

    Write-Info "Fetching version information from $Channel channel..."

    try {
        $response = Invoke-WebRequest -Uri $url -UseBasicParsing
        return $response.Content
    }
    catch {
        Write-Error-Msg "Failed to fetch version information: $_"
    }
}

# Download and verify file
function Get-FileWithVerification {
    param(
        [string]$Url,
        [string]$OutputPath,
        [string]$ExpectedHash
    )

    Write-Info "Downloading from $Url..."

    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
    }
    catch {
        Write-Error-Msg "Failed to download file: $_"
    }

    Write-Info "Verifying checksum..."

    $actualHash = (Get-FileHash -Path $OutputPath -Algorithm SHA256).Hash.ToLower()
    $expectedHashLower = $ExpectedHash.ToLower()

    if ($actualHash -ne $expectedHashLower) {
        Write-Error-Msg "Checksum verification failed!`nExpected: $expectedHashLower`nActual:   $actualHash"
    }

    Write-Info "Checksum verified successfully"
}

# Extract archive
function Expand-Archive-Custom {
    param(
        [string]$ArchivePath,
        [string]$DestinationPath
    )

    Write-Info "Extracting archive..."

    # Use built-in Expand-Archive for zip files
    Expand-Archive -Path $ArchivePath -DestinationPath $DestinationPath -Force
}

# Add directory to PATH
function Add-ToPath {
    param([string]$Directory)

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -notlike "*$Directory*") {
        Write-Warn "Adding $Directory to user PATH..."
        $newPath = "$Directory;$userPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$Directory;$env:Path"
        Write-Info "Added to PATH. Please restart your shell for changes to take effect."
    }
}

# Main installation function
function Install-Maa {
    # Detect platform
    $target = Get-Platform
    Write-Info "Detected platform: $target"

    # Fetch version information
    $versionInfo = Get-VersionInfo -Channel $Channel

    # Parse the .txt format (KEY=VALUE pairs)
    $versionData = @{}
    foreach ($line in $versionInfo -split "`n") {
        $line = $line.Trim()
        if ($line -match '^([A-Z_0-9]+)=(.+)$') {
            $versionData[$matches[1]] = $matches[2]
        }
    }

    # Get version
    $version = $versionData['VERSION']

    # Convert target to variable name format (e.g., x86_64-pc-windows-msvc -> X86_64_PC_WINDOWS_MSVC)
    $targetVar = $target.ToUpper().Replace('-', '_')

    # Get asset info for this platform
    $archiveName = $versionData["${targetVar}_NAME"]
    $sha256sum = $versionData["${targetVar}_SHA256"]

    if (-not $archiveName) {
        Write-Error-Msg "No release found for platform: $target"
    }

    Write-Info "Version: $version"
    Write-Info "Archive: $archiveName"

    # Construct download URL
    $downloadUrl = "https://github.com/$REPO/releases/download/v$version/$archiveName"

    # Create temporary directory
    $tempPath = if ($env:TEMP) { $env:TEMP } else { "/tmp" }
    $tmpDir = New-Item -ItemType Directory -Path "$tempPath/maa-cli-install-$(Get-Random)"

    try {
        # Download and verify
        $archivePath = Join-Path $tmpDir $archiveName
        Get-FileWithVerification -Url $downloadUrl -OutputPath $archivePath -ExpectedHash $sha256sum

        # Extract archive
        $extractDir = Join-Path $tmpDir "extract"
        Expand-Archive-Custom -ArchivePath $archivePath -DestinationPath $extractDir

        # Install binary
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        # Find the binary (it might be in a subdirectory)
        $binPath = Get-ChildItem -Path $extractDir -Filter "maa.exe" -Recurse -File | Select-Object -First 1 -ExpandProperty FullName

        if (-not $binPath) {
            Write-Error-Msg "Binary not found in extracted archive"
        }

        $destPath = Join-Path $InstallDir "maa.exe"

        Copy-Item -Path $binPath -Destination $destPath -Force

        Write-Info "Successfully installed maa to $destPath"

        # Add to PATH if not already there
        Add-ToPath -Directory $InstallDir

        Write-Info "Installation complete!"
        Write-Info "Run 'maa --help' to get started."
    }
    finally {
        # Cleanup temporary directory
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Validate channel
if ($Channel -notin @("stable", "beta", "alpha")) {
    Write-Error-Msg "Invalid channel: $Channel (must be stable, beta, or alpha)"
}

# Run installation
Install-Maa
