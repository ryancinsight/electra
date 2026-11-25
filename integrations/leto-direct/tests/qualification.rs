use electra_circuit::{MnaError, parse};
use electra_leto_direct::PivotedLu;
use std::fmt::Write;

const PIVOT: &str = include_str!("../fixtures/pivot.cir");
fn padded(order: usize) -> String {
    let mut deck = PIVOT.to_owned();
    for i in 6..order {
        writeln!(deck, "Rpad{i} p{i} 0 1").unwrap();
    }
    deck
}

#[test]
fn solves_zero_diagonal_circuit_through_limit() {
    for n in [6, 33, 256] {
        let c = parse(&padded(n)).unwrap();
        let p = c
            .operating_point_direct::<PivotedLu>(Default::default())
            .unwrap();
        for &(name, value) in &p.probes.volts {
            let expected = match name {
                "a" | "tap" => -7. / 3.,
                "b" => 14. / 3.,
                "mid" => 8. / 3.,
                _ => 0.,
            };
            assert!((value.into_base() - expected).abs() < 1e-12, "{name}");
        }
        assert!(p.probes.residual_amperes.into_base() < 1e-12);
    }
    let c = parse(&padded(257)).unwrap();
    assert!(matches!(
        c.operating_point_direct::<PivotedLu>(Default::default()),
        Err(MnaError::DirectOrder {
            order: 257,
            limit: 256
        })
    ));
}

#[test]
fn rejects_unresolvable_drop_and_singular_feedback() {
    for deck in [
        "Vbase a 0 1e16\nVsmall b a 1\nR1 b 0 1e16",
        "E1 out 0 out 0 1\nR1 out 0 1",
    ] {
        assert!(
            parse(deck)
                .unwrap()
                .operating_point_direct::<PivotedLu>(Default::default())
                .is_err()
        );
    }
}
