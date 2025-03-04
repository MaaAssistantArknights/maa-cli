use std::io::Result;

fn main() -> Result<()> {
    tonic_build::compile_protos("protos/task.proto")?;
    tonic_build::compile_protos("protos/core.proto")?;
    Ok(())
}