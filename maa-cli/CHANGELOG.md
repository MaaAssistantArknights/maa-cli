# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2023-08-16)

Initial release.

### Refactor (BREAKING)

 - <csr-id-744e0ad67b4fbf3937169c96821f98b6176e3816/> rename workspace members to avoid confusion
   The `maa-runner` crate is renamed to `maa-run` which is the binary name,
   and the `maa-helper` crate is renamed to `maa-cli` as because it is providing
   the command line interface.

### New Features (BREAKING)

 - <csr-id-b95d1ed2687c8cfba73e2558a2835e627e5d34a6/> maa-run as a subcommand of maa-helper to set env vars for maa-run
   * feat!: maa-run as a subcommand of maa-helper to set env vars for maa-run
   
   - Rename maa-cli to maa-runner, and bin name form maa to maa-run
   - Rename maa-updater to maa-helper, and bin name form maa-updater to maa
   - Add subcommand for maa-helper to run maa-run

### Documentation

 - <csr-id-9f196ae99193e45e9c6625ac92fefd0a8f04c4eb/> add CHANGELOG of maa-cli

### Chore

 - <csr-id-e4b134dbfb2800760fca355750e3e0ec26e8437a/> change package name to maa-cli
   This don't change anything, but follow the dir name.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#7](https://github.com/wangl-cc/maa-cli/issues/7), [#9](https://github.com/wangl-cc/maa-cli/issues/9)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#7](https://github.com/wangl-cc/maa-cli/issues/7)**
    - Maa-run as a subcommand of maa-helper to set env vars for maa-run ([`b95d1ed`](https://github.com/wangl-cc/maa-cli/commit/b95d1ed2687c8cfba73e2558a2835e627e5d34a6))
 * **[#9](https://github.com/wangl-cc/maa-cli/issues/9)**
    - Rename workspace members to avoid confusion ([`744e0ad`](https://github.com/wangl-cc/maa-cli/commit/744e0ad67b4fbf3937169c96821f98b6176e3816))
 * **Uncategorized**
    - Add CHANGELOG of maa-cli ([`9f196ae`](https://github.com/wangl-cc/maa-cli/commit/9f196ae99193e45e9c6625ac92fefd0a8f04c4eb))
    - Change package name to maa-cli ([`e4b134d`](https://github.com/wangl-cc/maa-cli/commit/e4b134dbfb2800760fca355750e3e0ec26e8437a))
</details>

