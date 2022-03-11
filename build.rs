use vergen::{vergen, Config};

fn main() -> anyhow::Result<()> {
    // Generate the default 'cargo:' instruction output
    vergen(Config::default())
}
