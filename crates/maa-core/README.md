# maa-core

Safe, idiomatic Rust API for [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) (MaaCore).

Built on top of [`maa-sys`](../maa-sys), which provides the raw FFI bindings.

## Usage

### Loading MaaCore

By default, `maa-core` enables the `runtime` feature, so load MaaCore before
calling any API and set up its resources:

```rust
use maa_core::Assistant;

// With the `runtime` feature: load the shared library explicitly
Assistant::load("/path/to/libMaaCore.so")?;

// Set the user directory (for logs, cache, etc.)
Assistant::set_user_dir("/path/to/user/dir")?;

// Load MaaCore's resource files
Assistant::load_resource("/path/to/maa")?;
```

Without the `runtime` feature, `load()` is a no-op and MaaCore is linked at
link time instead.

### Creating an Assistant

Use `Assistant::new()` when no callback is needed, or
`Assistant::new_with_callback()` to receive messages from MaaCore.
The callback can be a closure, a plain function, or an `Arc<T>` for shared state:

```rust
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use maa_core::{Assistant, Callback};
use maa_types::MessageKind;

// No callback
let asst = Assistant::new()?;

// Plain closure
let asst = Assistant::new_with_callback(|kind: MessageKind, msg: Option<&str>| {
    println!("{kind:?}: {msg:?}");
})?;

// Shared state via Arc
struct Counter(AtomicU32);
impl Callback for Counter {
    fn on_message(&self, _kind: MessageKind, _msg: Option<&str>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
let counter = Arc::new(Counter(AtomicU32::new(0)));
let asst = Assistant::new_with_callback(Arc::clone(&counter))?;
```

### Running tasks

```rust
// Connect to a device
asst.async_connect("adb", "emulator-5554", "General", true)?;

// Append tasks and start
asst.append_task("StartUp", r#"{"client_type":"Official"}"#)?;
asst.start()?;

// Wait for completion
while asst.running() {
    std::thread::sleep(std::time::Duration::from_millis(500));
}
asst.stop()?;
```

## Features

The `runtime` feature mirrors the one in `maa-sys`: when enabled, MaaCore is
loaded dynamically via `Assistant::load` instead of being linked at link time.
See [`maa-sys`](../maa-sys/README.md) for details on how the library is located.

<!-- markdownlint-disable-file MD013 -->
