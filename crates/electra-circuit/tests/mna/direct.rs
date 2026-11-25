use electra_circuit::{
    ElectricPotential, LinearSweep, MnaError, SweepRange,
    direct::{DirectFactor, Matrix},
    parse,
};
use std::cell::Cell;

thread_local! { static FACTORS: Cell<usize> = const { Cell::new(0) }; }

// A single ideal source has the normalized operator [0 1; 1 0].
struct Exchange<const BAD: u8>;
impl<const BAD: u8> DirectFactor for Exchange<BAD> {
    const MAX_ORDER: usize = 2;
    fn factor(matrix: Matrix<'_>) -> Result<Self, MnaError> {
        assert_eq!(matrix.order(), 2);
        let mut entries = [[0.; 2]; 2];
        matrix.for_each_entry(|r, c, v| entries[r][c] += v);
        assert_eq!(entries, [[0., 1.], [1., 0.]]);
        FACTORS.set(FACTORS.get() + 1);
        Ok(Self)
    }

    fn solve_into(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MnaError> {
        assert_eq!(rhs.len(), 2);
        out.copy_from_slice(&[rhs[1], rhs[0]]);
        match BAD {
            1 => out[1] = 1.,
            2 => out[0] += 1.,
            3 => out[1] = f64::NAN,
            _ => (),
        }
        Ok(())
    }
}

#[test]
fn named_sweep_factors_once_and_returns_typed_probes() {
    FACTORS.set(0);
    let c = parse("Vdrive a 0 5").unwrap();
    let volts = ElectricPotential::from_base;
    let range = SweepRange::Voltage(LinearSweep {
        start: volts(2.),
        stop: volts(-2.),
        step: volts(-1.),
    });
    let mut sweep = c
        .sweep_source_direct::<Exchange<0>>("vdrive", range, Default::default())
        .unwrap();
    assert_eq!(sweep.source_name(), "Vdrive");
    for expected in [2., 1., 0., -1., -2.] {
        let p = sweep.next_point().unwrap().unwrap().result;
        assert_eq!(p.probes.volts[1].1, volts(expected));
        assert_eq!(p.probes.amperes[0].1.into_base(), 0.);
        assert_eq!(p.iterations, 0);
    }
    assert!(sweep.next_point().unwrap().is_none());
    assert!(sweep.next_point().unwrap().is_none());
    assert_eq!(FACTORS.get(), 1);
}

#[test]
fn rejects_bad_provider_output_and_fuses_after_error() {
    let c = parse("Vdrive a 0 5\n.dc Vdrive 1 2 1").unwrap();
    assert!(matches!(
        c.operating_point_direct::<Exchange<1>>(Default::default()),
        Err(MnaError::CurrentBalance { .. })
    ));
    assert!(matches!(
        c.operating_point_direct::<Exchange<3>>(Default::default()),
        Err(MnaError::Arithmetic)
    ));
    let mut sweep = c
        .dc_sweep_direct::<Exchange<2>>(Default::default())
        .unwrap();
    assert!(
        matches!(sweep.next_point(), Err(MnaError::Point(_, inner)) if matches!(*inner, MnaError::SourceVoltage{..}))
    );
    assert!(sweep.next_point().unwrap().is_none());
}

#[test]
fn rejects_oversize_before_factor_or_floating_check() {
    let c = parse("Vdrive a 0 5\nRfloat b c 1").unwrap();
    assert!(matches!(
        c.operating_point_direct::<Exchange<0>>(Default::default()),
        Err(MnaError::DirectOrder { order: 4, limit: 2 })
    ));
}
