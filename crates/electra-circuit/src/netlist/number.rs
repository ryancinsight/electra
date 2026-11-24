/// Accept a finite base-unit magnitude with one optional SPICE scale factor.
/// ASCII M is milli. Mega is spelled meg. Trailing unit text is rejected.
pub(crate) fn magnitude(token: &str) -> Option<f64> {
    let numeric = token.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &token[numeric.len()..];
    let factor = match suffix.as_bytes() {
        [] => 1.0,
        [b't' | b'T'] => 1e12,
        [b'g' | b'G'] => 1e9,
        s if s.eq_ignore_ascii_case(b"meg") => 1e6,
        [b'k' | b'K'] => 1e3,
        [b'm' | b'M'] => 1e-3,
        [b'u' | b'U'] => 1e-6,
        [b'n' | b'N'] => 1e-9,
        [b'p' | b'P'] => 1e-12,
        [b'f' | b'F'] => 1e-15,
        _ => return None,
    };
    let value = numeric.parse::<f64>().ok()?;
    let scaled = value * factor;
    if scaled == 0.0 {
        let mantissa = numeric.split(['e', 'E']).next()?;
        if mantissa.bytes().any(|digit| matches!(digit, b'1'..=b'9')) {
            return None;
        }
    }
    scaled.is_finite().then_some(scaled)
}
