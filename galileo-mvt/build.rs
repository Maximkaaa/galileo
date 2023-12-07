use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/vector_tile.proto"], &["src/"])?;
    Ok(())
}
