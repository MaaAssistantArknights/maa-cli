use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(cfg!(feature = "client"))
        .build_transport(false)
        .extern_path(".types", "::maa_types")
        .compile_protos(&["protos/task.proto", "protos/core.proto"], &[
            "protos",
            "../maa-types",
        ])?;
    Ok(())
}
