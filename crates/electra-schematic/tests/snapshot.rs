//! Verify the native wire format independently of its writer.
use electra_schematic::{Diagnostic, NetId, Pin, PinId, Role, Schematic};
use std::io::ErrorKind;

const EMPTY: &[u8] = b"ELCP\x01\0\0\0\0\0\0\0\0\0\0\0";
const RECORD: &[u8] = b"\x03\0\0\0\0\0\0\0\x07\0\0\0\x07\0\0\0\0\0\0\0\0";

#[test]
fn independently_encoded_record_preserves_identity_and_properties() {
    let mut bytes = EMPTY.to_vec();
    bytes[8] = 1;
    bytes.extend(RECORD);
    let mut model = Schematic::read_connectivity(bytes.as_slice()).unwrap();
    let id = PinId {
        instance: 3,
        pin: 7,
    };
    assert_eq!(
        *model.pin_mut(id).unwrap(),
        Pin {
            required: true,
            role: Role::Output,
            net: Some(NetId(0))
        }
    );
    let mut findings = Vec::new();
    model.check(|item| findings.push(item));
    assert_eq!(findings, [Diagnostic::Unconnected(id)]);
    let mut actual = Vec::new();
    model.write_connectivity(&mut actual).unwrap();
    assert_eq!(actual, bytes);
    for end in 0..bytes.len() {
        assert!(Schematic::read_connectivity(&bytes[..end]).is_err());
    }
}

#[test]
fn rejects_unknown_version_flags_duplicate_and_trailing_data() {
    let mut bytes = EMPTY.to_vec();
    bytes[4] = 3;
    let rejected = |input: &[u8]| {
        assert_eq!(
            Schematic::read_connectivity(input).err().unwrap().kind(),
            ErrorKind::InvalidData
        )
    };
    rejected(&bytes);
    bytes[4] = 1;
    bytes.extend(RECORD);
    rejected(&bytes);
    bytes[8] = 1;
    bytes[28] |= 8;
    rejected(&bytes);
    bytes[28] &= !8;
    bytes[8] = 2;
    bytes.extend(RECORD);
    rejected(&bytes);
}

#[test]
fn canonical_bytes_survive_reopen_and_ignore_insertion_order() {
    let encode = |order: [u64; 2]| {
        let mut model = Schematic::default();
        for instance in order {
            model.insert(
                PinId {
                    instance,
                    pin: u32::MAX,
                },
                Pin {
                    required: instance == 0,
                    role: Role::Passive,
                    net: (instance == 0).then_some(NetId(u64::MAX)),
                },
            );
        }
        let mut bytes = Vec::new();
        model.write_connectivity(&mut bytes).unwrap();
        bytes
    };
    let bytes = encode([0, u64::MAX]);
    assert_eq!(bytes, encode([u64::MAX, 0]));
    let model = Schematic::read_connectivity(bytes.as_slice()).unwrap();
    let mut reopened = Vec::new();
    model.write_connectivity(&mut reopened).unwrap();
    assert_eq!(reopened, bytes);
}
