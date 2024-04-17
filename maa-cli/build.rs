fn main() {
    println!("cargo:rerun-if-env-changed=MAA_VERSION");
    if let Ok(version) = std::env::var("MAA_VERSION") {
        println!("cargo:rustc-env=MAA_VERSION={}", version);
    } else {
        println!("cargo:rustc-env=MAA_VERSION={}", env!("CARGO_PKG_VERSION"));
    }
}
