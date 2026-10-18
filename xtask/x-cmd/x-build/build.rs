fn main() {
    // Cargo sets TARGET for every build script.
    let target = std::env::var("TARGET").expect("Cargo should set TARGET");
    println!("cargo:rustc-env=TARGET={target}");
}
