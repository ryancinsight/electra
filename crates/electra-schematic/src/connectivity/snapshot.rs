//! Native connectivity snapshot v1. All integers are little-endian.
use super::{NetId, Pin, PinId, Role, Schematic};
use std::io::{self, Read, Write};

const HEADER: &[u8; 8] = b"ELCP\x01\0\0\0";
const REQUIRED: u8 = 1;
const OUTPUT: u8 = 2;
const WIRED: u8 = 4;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_bytes<const N: usize>(input: &mut impl Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    input.read_exact(&mut bytes)?;
    Ok(bytes)
}

impl Schematic {
    /// Write a canonical native snapshot in occurrence and pin order.
    /// Buffer file-backed writers to amortize small writes.
    pub fn write_connectivity(&self, mut output: impl Write) -> io::Result<()> {
        output.write_all(HEADER)?;
        let count = u64::try_from(self.pins.len()).map_err(|_| invalid("pin count overflow"))?;
        output.write_all(&count.to_le_bytes())?;
        for (id, pin) in &self.pins {
            output.write_all(&id.instance.to_le_bytes())?;
            output.write_all(&id.pin.to_le_bytes())?;
            let flags = (u8::from(pin.required) * REQUIRED)
                | (u8::from(pin.role == Role::Output) * OUTPUT)
                | (u8::from(pin.net.is_some()) * WIRED);
            output.write_all(&[flags])?;
            if let Some(net) = pin.net {
                output.write_all(&net.0.to_le_bytes())?;
            }
        }
        Ok(())
    }

    /// Load one complete snapshot from a bounded input stream ending at EOF.
    /// Reject unsupported versions, unknown flags, duplicate pins and trailing data.
    pub fn read_connectivity(mut input: impl Read) -> io::Result<Self> {
        if &read_bytes(&mut input)? != HEADER {
            return Err(invalid("unsupported connectivity header or version"));
        }
        let count = u64::from_le_bytes(read_bytes(&mut input)?);
        let mut schematic = Self::default();
        for _ in 0..count {
            let instance = u64::from_le_bytes(read_bytes(&mut input)?);
            let pin = u32::from_le_bytes(read_bytes(&mut input)?);
            let [flags] = read_bytes(&mut input)?;
            if flags & !(REQUIRED | OUTPUT | WIRED) != 0 {
                return Err(invalid("unknown pin flags"));
            }
            let role = if flags & OUTPUT != 0 {
                Role::Output
            } else {
                Role::Passive
            };
            let net = if flags & WIRED != 0 {
                Some(NetId(u64::from_le_bytes(read_bytes(&mut input)?)))
            } else {
                None
            };
            if !schematic.insert(
                PinId { instance, pin },
                Pin {
                    required: flags & REQUIRED != 0,
                    role,
                    net,
                },
            ) {
                return Err(invalid("duplicate pin address"));
            }
        }
        match read_bytes::<1>(&mut input) {
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(schematic),
            Err(error) => Err(error),
            Ok(_) => Err(invalid("trailing data after connectivity snapshot")),
        }
    }
}
