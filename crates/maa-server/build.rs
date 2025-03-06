use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(cfg!(feature = "client"))
        .build_transport(false)
        .compile_protos(&["protos/task.proto", "protos/core.proto"], &["protos"])?;
    Ok(())
}
