//! Locate the repository workspace for automation commands.

pub fn root() -> &'static std::path::Path {
    // x-workspace lives in xtask/x-lib/x-workspace, three levels below the workspace root.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("x-workspace should be three levels below the workspace root")
}
