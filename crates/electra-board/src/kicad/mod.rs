//! KiCad object identity and validation.
/// Format an emission sequence number using the driver's deterministic UUID convention.
pub fn sequence_uuid(sequence: u64) -> String {
    format!("00000000-0000-0000-0000-{sequence:012x}")
}
/// Return duplicate UUID values from a KiCad PCB text, sorted lexicographically.
///
/// KiCad identifies board objects by UUID. Reusing a UUID across footprints, pads, tracks, or vias can
/// make the ratsnest and DRC diagnostics refer to the wrong object, so generated boards are not
/// trustworthy until this returns an empty vector.
#[must_use]
pub fn duplicate_pcb_uuids(pcb_text: &str) -> Vec<String> {
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    let needle = "(uuid";
    let mut rest = pcb_text;
    while let Some(idx) = rest.find(needle) {
        rest = &rest[idx + needle.len()..];
        let trimmed = rest.trim_start();
        let Some(after_quote) = trimmed.strip_prefix('"') else {
            rest = trimmed;
            continue;
        };
        let mut escaped = false;
        let mut end = None;
        for (i, c) in after_quote.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                end = Some(i);
                break;
            }
        }
        let Some(end) = end else {
            break;
        };
        let value = &after_quote[..end];
        if is_uuid_shape(value) {
            *counts.entry(value).or_default() += 1;
        }
        rest = &after_quote[end + 1..];
    }

    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(uuid, _)| uuid.to_owned())
        .collect()
}

fn is_uuid_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, b)| match i {
        8 | 13 | 18 | 23 => *b == b'-',
        _ => b.is_ascii_hexdigit(),
    })
}
