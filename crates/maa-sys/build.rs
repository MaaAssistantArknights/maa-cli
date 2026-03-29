#[cfg(not(feature = "runtime"))]
fn dynamic_link() {
    use std::{env::var_os, path::PathBuf};

    println!("cargo:rerun-if-env-changed=MAA_CORE_DIR");
    if let Some(core_dir) = var_os("MAA_CORE_DIR").map(PathBuf::from) {
        println!("cargo:rustc-link-search=native={}", core_dir.display());
    }
}

fn main() {
    #[cfg(not(feature = "runtime"))]
    dynamic_link();
}
