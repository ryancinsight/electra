//! Version 3 model table, ordered by project occurrence.
use super::{invalid, read_bytes};
use crate::Schematic;
use std::io::{self, Read, Write};

pub(super) fn write(model: &Schematic, output: &mut impl Write) -> io::Result<()> {
    output.write_all(
        &u64::try_from(model.models.len())
            .map_err(|_| invalid("model count overflow"))?
            .to_le_bytes(),
    )?;
    for (id, binding) in &model.models {
        output.write_all(&id.to_le_bytes())?;
        let name = binding.name().as_bytes();
        output.write_all(
            &u16::try_from(name.len())
                .map_err(|_| invalid("model name overflow"))?
                .to_le_bytes(),
        )?;
        output.write_all(name)?;
        output.write_all(
            &u16::try_from(binding.pins().len())
                .map_err(|_| invalid("model pin count overflow"))?
                .to_le_bytes(),
        )?;
        for pin in binding.pins() {
            output.write_all(&pin.to_le_bytes())?;
        }
    }
    Ok(())
}
pub(super) fn read(model: &mut Schematic, input: &mut impl Read) -> io::Result<()> {
    let count = u64::from_le_bytes(read_bytes(input)?);
    for _ in 0..count {
        let id = u64::from_le_bytes(read_bytes(input)?);
        if model.model(id).is_some() {
            return Err(invalid("duplicate occurrence model"));
        }
        let len = u16::from_le_bytes(read_bytes(input)?);
        let mut name = vec![0; usize::from(len)];
        input.read_exact(&mut name)?;
        let name = String::from_utf8(name).map_err(|_| invalid("invalid model UTF-8"))?;
        let size = u16::from_le_bytes(read_bytes(input)?);
        let mut pins = Vec::with_capacity(usize::from(size));
        for _ in 0..size {
            pins.push(u32::from_le_bytes(read_bytes(input)?));
        }
        model.bind_model(id, name, pins).map_err(invalid)?;
    }
    Ok(())
}
