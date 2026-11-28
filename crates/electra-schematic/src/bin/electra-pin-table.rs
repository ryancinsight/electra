//! Export stable schematic occurrence, terminal and net IDs for board synchronization.
use electra_schematic::Schematic;
use std::{
    error::Error,
    fs::File,
    io::{BufReader, BufWriter, Write},
};
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: electra-pin-table PROJECT.elcp")?;
    let model = Schematic::read_connectivity(BufReader::new(File::open(path)?))?;
    let mut output = BufWriter::new(std::io::stdout().lock());
    for (id, p) in model.pins() {
        write!(output, "{} {} ", id.instance, id.pin)?;
        if let Some(net) = p.net {
            writeln!(output, "{}", net.0)?;
        } else {
            writeln!(output, "off")?;
        }
    }
    Ok(output.flush()?)
}
