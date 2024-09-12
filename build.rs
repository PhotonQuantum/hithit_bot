use vergen_gix::{Emitter, GixBuilder};

fn main() -> anyhow::Result<()> {
    // Generate the default 'cargo:' instruction output
    Emitter::new().add_instructions(&GixBuilder::default().sha(true).build()?)?.emit()?;
    Ok(())
}
