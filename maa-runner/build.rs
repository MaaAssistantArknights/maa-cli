use maa_utils::dirs::{Dirs, ProjectDirs};

fn main() {
    let project = ProjectDirs::from("maa");
    let dirs = project.data_dir().unwrap();
    println!(
        "cargo:rustc-link-search=native={}/lib",
        dirs.to_str().unwrap()
    );
}
