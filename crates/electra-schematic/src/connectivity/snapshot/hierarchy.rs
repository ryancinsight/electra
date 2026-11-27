//! Version 4 sheet and bus records. Each name appears once per group.
use super::{invalid, read_bytes};
use crate::{NetId, PinId, Schematic};
use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
};

fn count(out: &mut impl Write, value: usize) -> io::Result<()> {
    out.write_all(
        &u64::try_from(value)
            .map_err(|_| invalid("group count overflow"))?
            .to_le_bytes(),
    )
}
fn name(out: &mut impl Write, value: &str) -> io::Result<()> {
    out.write_all(
        &u16::try_from(value.len())
            .map_err(|_| invalid("name overflow"))?
            .to_le_bytes(),
    )?;
    out.write_all(value.as_bytes())
}
fn read_name(input: &mut impl Read) -> io::Result<String> {
    let len = u16::from_le_bytes(read_bytes(input)?);
    if len == 0 {
        return Err(invalid("empty group name"));
    }
    let mut bytes = vec![0; usize::from(len)];
    input.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| invalid("invalid group name"))
}
pub(super) fn write(model: &Schematic, out: &mut impl Write) -> io::Result<()> {
    count(out, model.sheets.len())?;
    for (path, parts) in &model.sheets {
        name(out, path)?;
        count(out, parts.len())?;
        for id in parts {
            out.write_all(&id.to_le_bytes())?;
        }
    }
    count(out, model.buses.len())?;
    for (bus, lanes) in &model.buses {
        name(out, bus)?;
        out.write_all(
            &u16::try_from(lanes.len())
                .map_err(|_| invalid("bus width overflow"))?
                .to_le_bytes(),
        )?;
        for net in lanes {
            out.write_all(&net.0.to_le_bytes())?;
        }
    }
    Ok(())
}
pub(super) fn read(model: &mut Schematic, input: &mut impl Read) -> io::Result<()> {
    let mut assigned = BTreeSet::new();
    for _ in 0..u64::from_le_bytes(read_bytes(input)?) {
        let path = read_name(input)?;
        let mut parts = BTreeSet::new();
        let size = u64::from_le_bytes(read_bytes(input)?);
        if size == 0 {
            return Err(invalid("empty sheet record"));
        }
        for _ in 0..size {
            let instance = u64::from_le_bytes(read_bytes(input)?);
            if !assigned.insert(instance)
                || model
                    .pins
                    .range(
                        PinId { instance, pin: 0 }..=PinId {
                            instance,
                            pin: u32::MAX,
                        },
                    )
                    .next()
                    .is_none()
            {
                return Err(invalid("duplicate or missing sheet component"));
            }
            parts.insert(instance);
        }
        if model.sheets.insert(path, parts).is_some() {
            return Err(invalid("duplicate sheet record"));
        }
    }
    for _ in 0..u64::from_le_bytes(read_bytes(input)?) {
        let bus = read_name(input)?;
        let size = u16::from_le_bytes(read_bytes(input)?);
        let mut lanes = Vec::with_capacity(usize::from(size));
        for _ in 0..size {
            lanes.push(NetId(u64::from_le_bytes(read_bytes(input)?)));
        }
        if model.set_bus(bus, lanes).map_err(invalid)?.is_some() {
            return Err(invalid("duplicate bus record"));
        }
    }
    Ok(())
}
