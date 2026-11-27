//! Stream a supported KiCad property edit to standard output.
use electra_schematic::kicad::Document;
use std::{env, error::Error, fs, io};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path, id, key, value] = args.as_slice() else {
        return Err("expected FILE SYMBOL PROPERTY VALUE".into());
    };
    Document::parse(&fs::read_to_string(path)?)?.write_property(
        id,
        key,
        value,
        io::stdout().lock(),
    )?;
    Ok(())
}
