# Repository Guidelines

## Agent Checklist (Do This)

- Run `cargo +nightly fmt` first to normalize formatting.
- Run `cargo clippy` early to catch lint failures.
- Quick smoke: `cargo test -p <crate>` for the edited crate(s).
- Local verification: `cargo x test --no-clippy` (skip duplicate clippy if already run).
- Final check: `cargo x test` (CI parity: build + clippy + tests).
- `cargo x test` sets `MAA_CONFIG_DIR` and `MAA_EXTRA_SHARE_NAME`; avoid overrides unless required.
- Avoid `cargo x release` or packaging tasks unless explicitly requested.

## Architecture Overview (Understand This)

### Core Flow

```text
User Command → CLI Parser (clap) → Task Config → MaaCore FFI → Game Automation
                ↓                      ↓              ↓
            maa-cli              maa-value       maa-sys
```

**Key principle**: CLI orchestrates, libraries provide building blocks, FFI stays contained in `maa-sys`.

### Crate Map (What Each Crate Does)

**Application Layer:**

- `maa-cli`: CLI entry point, command routing, task orchestration, config handling, and installers. **Use when**: Adding commands, task presets, or installer logic.

**Core Abstractions:**

- `maa-sys`: MaaCore FFI bindings and safe `Assistant` wrapper. **Use when**: Adding new MaaCore API calls or fixing FFI issues. Keep `unsafe` here.
- `maa-types`: Shared enums/primitives (TaskType, ClientType, TouchMode, etc.). **Use when**: Adding new task types or MaaCore options.
- `maa-dirs`: OS-specific path resolution with XDG support. **Use when**: Accessing config/cache/state/resource directories or finding the MaaCore library.
- `maa-value`: Dynamic config values with conditional parameters and user input prompts. **Use when**: Handling task parameters, merging configs, or prompting users.

**Utilities:**

- `maa-installer`: Download/extract/verify/manifest utilities. **Use when**: Adding update sources or improving download reliability.
- `maa-version`: Version manifest parsing and comparison. **Use when**: Handling version checks or update logic.
- `maa-value-macro`: Proc macros for `MAAValue` construction. **Use when**: Building complex config objects in code.
- `maa-str-ext`: UTF-8 conversion for `OsStr`/`Path`/bytes. **Use when**: Dealing with strings that might not be UTF-8.
- `maa-ffi-string`: UTF-8 `CString` for FFI. **Use when**: Passing Rust strings to MaaCore C API.

## Critical Patterns

- **Paths**: Always use `maa-dirs` for directory resolution. Never hardcode paths like `/usr/local/share` or `C:\Program Files`. The crate handles platform differences and environment overrides (`MAA_CONFIG_DIR`, etc.).
- **FFI Boundary**: Keep `unsafe` confined to `maa-sys`. The `Assistant` struct provides safe wrappers. Other crates should never call MaaCore directly.
- **Error Handling**: `maa-cli` uses `anyhow::Result` for application errors. Libraries use `thiserror` for structured errors. Never use `unwrap()` without a safety comment.
- **String Conversions**: Use `maa-str-ext` for `OsStr`/`Path` ↔ UTF-8, and `maa-ffi-string` for FFI `CString` conversion. Handle non-UTF-8 gracefully.
- **String Formatting**: When using `format!` and related macros (`write!`, `log::info!`, `bail!`, etc.), always inline variables into `{}` placeholders. Example: use `format!("Error: {error}")` instead of `format!("Error: {}", error)`.

## Code Quality & Safety

- Formatting and linting are required; follow the checklist.
- Clippy warnings fail CI; use `#[allow]` or `#[expect]` with a short reason when unavoidable.
- Errors: `maa-cli` uses `anyhow`; libraries use `thiserror` or explicit error types.
- Avoid `unwrap`/`expect` and `unsafe`; if used, add a brief justification comment.

## Testing Rules

- All changes must be covered by tests，except FFI or external network access.
- Prefer unit tests with `#[test]`; add regression tests for bug fixes.
- Mark tests `#[ignore]` only when they require external network access or read/write system or user home directories.
- Tests using local `test_server` helpers or writing under `temp_dir()` are OK and should not be ignored.
- Run ignored tests explicitly with `cargo test -- --ignored` when needed.

## Documentation Rules

- Update docs when commands or config behavior changes.
- Primary language is Simplified Chinese; add English when practical.
- One paragraph per line; use `<br>` for line breaks; no trailing spaces.
- Markdown is linted by `markdownlint-cli2`.

## MISC

- Use conventional commits: `feat: ...`, `fix: ...`, `refactor: ...`, `perf: ...`, `docs: ...`, `test: ...`, etc.
