//! Lossless edit boundary, including nested and escaped opaque fields.
use electra_schematic::kicad::Document;
#[test]
fn edits_only_the_selected_property_and_preserves_unknown_subtrees() {
    let s = r#"(kicad_symbol_lib (version 20241209) (symbol "X" (property "Value" "old") (unknown (nested ")(\""))))"#;
    let doc = Document::parse(s).unwrap();
    let mut bytes = Vec::new();
    doc.write(&mut bytes).unwrap();
    assert_eq!(bytes, s.as_bytes());
    bytes.clear();
    doc.write_property("X", "Value", "μ\"", &mut bytes).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        s.replace("\"old\"", "\"μ\\\"\"")
    );
    for input in [
        s.replace("20241209", "20240101"),
        s[..s.len() - 1].to_owned(),
        format!("{s} ()"),
    ] {
        assert!(Document::parse(&input).is_err());
    }
    let duplicate = s.replace(
        "(property \"Value\" \"old\")",
        "(property \"Value\" \"a\")(property \"Value\" \"b\")",
    );
    let mut empty = Vec::new();
    assert!(
        Document::parse(&duplicate)
            .unwrap()
            .write_property("X", "Value", "bad", &mut empty)
            .is_err()
    );
    assert!(empty.is_empty());
}
