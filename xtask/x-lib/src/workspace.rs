//! Locate the repository workspace for automation commands.

pub fn root() -> &'static std::path::Path {
    // x-lib lives in xtask/x-lib, two levels below the workspace root.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("x-lib should be two levels below the workspace root")
}
