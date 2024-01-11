use std::io::Result;

fn main() -> Result<()> {
    #[cfg(feature = "generate_proto")]
    prost_build::compile_protos(&["src/vector_tile.proto"], &["src/"])?;
    Ok(())
}
