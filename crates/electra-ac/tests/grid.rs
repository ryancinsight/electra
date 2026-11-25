//! Streaming frequency grid endpoints, spacing and failure semantics.
use electra_ac::{AcError, Frequency, LogSweep};

fn grid(a: f64, b: f64, n: u32) -> Result<LogSweep, AcError> {
    LogSweep::new(Frequency::from_base(a), Frequency::from_base(b), n)
}
#[test]
fn ascending_descending_and_single_tone_preserve_endpoints() {
    for (a, b) in [(100., 10000.), (10000., 100.)] {
        let mut sweep = grid(a, b, 3).unwrap();
        assert_eq!(sweep.next().unwrap().unwrap().into_base(), a);
        let f = sweep.next().unwrap().unwrap().into_base();
        assert!((f - 1000.).abs() < 1e-10);
        assert_eq!(sweep.next().unwrap().unwrap().into_base(), b);
        assert!(sweep.next().is_none());
    }
    assert_eq!(
        grid(1000., 1000., 1)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .into_base(),
        1000.
    );
}
#[test]
fn invalid_or_unresolvable_grids_report_an_error_and_fuse() {
    for (a, b, n) in [
        (1., 1., 0),
        (1., 1., 2),
        (1., 2., 1),
        (-1., 1., 5),
        (0., 1., 3),
        (1., f64::INFINITY, 4),
    ] {
        assert_eq!(grid(a, b, n).unwrap_err(), AcError::Grid);
    }
    let mut grid = grid(1., 1.0_f64.next_up(), 3).unwrap();
    assert!(grid.next().unwrap().is_ok());
    assert_eq!(grid.next(), Some(Err(AcError::Grid)));
    assert!(grid.next().is_none());
}
#[test]
fn samples_widely_separated_endpoints_without_endpoint_ratio_overflow() {
    let grid = grid(1e-300, 1e300, 3)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!((grid[1].into_base() - 1.).abs() < 2e-13);
}
