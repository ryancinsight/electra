//! Pinned board/coupon comparison with an independent rectangular oracle.
use electra_ac::{
    Capacitance, Complex64, ElectricalImpedance, Frequency, Inductance, Length, PortImpedance,
    pdn::CapacitorBank,
};
use electra_board_field::{
    pdn::{PdnError, ViaPair, placed_capacitor},
    rail::Resistivity,
};

const GRID: [f64; 13] = [
    10e3, 20e3, 50e3, 100e3, 200e3, 500e3, 1e6, 2e6, 5e6, 10e6, 20e6, 50e6, 100e6,
];
const TARGET: f64 = 0.080;
const RHO: f64 = 1.724e-8;
#[derive(Clone, Copy)]
struct Geometry(f64, f64);
#[derive(Clone, Copy)]
struct Part(f64, f64, f64);
const GEOMETRIES: [Geometry; 3] = [
    Geometry(0.20e-3, 1.00e-3),
    Geometry(0.20e-3, 0.60e-3),
    Geometry(0.25e-3, 0.45e-3),
];
const BOMS: [[Part; 2]; 3] = [
    [Part(10e-6, 20e-3, 1.0e-9), Part(100e-9, 50e-3, 0.5e-9)],
    [Part(4.7e-6, 18e-3, 0.7e-9), Part(220e-9, 35e-3, 0.35e-9)],
    [Part(2.2e-6, 15e-3, 0.6e-9), Part(470e-9, 25e-3, 0.4e-9)],
];
#[rustfmt::skip]
const NAMES: [&str; 9] = [
    "fixed/bulk-local", "fixed/wideband", "fixed/midband",
    "moderate/bulk-local", "moderate/wideband", "moderate/midband",
    "compact/bulk-local", "compact/wideband", "compact/midband"];
#[rustfmt::skip]
const RANKING: [&str; 9] = [
    "moderate/bulk-local", "compact/bulk-local", "fixed/bulk-local",
    "fixed/wideband", "moderate/wideband", "compact/wideband",
    "fixed/midband", "moderate/midband", "compact/midband"];
fn via(geometry: Geometry) -> ViaPair {
    ViaPair::new(
        Length::from_base(1.6e-3),
        Length::from_base(geometry.0),
        Length::from_base(20e-6),
        Length::from_base(geometry.1),
        Resistivity::from_base(RHO),
    )
    .expect("invariant: pinned coupon geometry is valid")
}
fn error<T: std::fmt::Debug>(result: Result<T, PdnError>) -> PdnError {
    result.expect_err("invariant: rejected fixture input must carry an error")
}
fn trace(geometry: Geometry, bom: [Part; 2]) -> Vec<Complex64> {
    let connection = via(geometry);
    let branches = bom.map(|part| {
        placed_capacitor(
            Capacitance::from_base(part.0),
            ElectricalImpedance::from_base(part.1),
            Inductance::from_base(part.2),
            connection,
        )
        .expect("invariant: enumerated parts are passive")
    });
    let bank = CapacitorBank::new(&branches);
    GRID.map(|hz| {
        let frequency = Frequency::from_base(hz);
        let PortImpedance::Finite(bank) = bank
            .at(frequency)
            .expect("invariant: fixture remains in numeric range")
        else {
            panic!("invariant: a nonempty positive-frequency bank is finite");
        };
        bank.into_base()
            + connection
                .impedance(frequency)
                .expect("invariant: fixture feed reactance is representable")
                .into_base()
    })
    .into()
}

fn independent(geometry: Geometry, bom: [Part; 2], hz: f64) -> Complex64 {
    let resistance = 2.0 * RHO * 1.6e-3 / (std::f64::consts::PI * geometry.0 * 20e-6);
    let inductance = 4.0e-7 * 1.6e-3 * (geometry.1 / geometry.0).acosh();
    let omega = std::f64::consts::TAU * hz;
    let admittance = bom.into_iter().fold(Complex64::default(), |sum, part| {
        let z = Complex64::new(
            part.1 + resistance,
            omega * (part.2 + inductance) - (omega * part.0).recip(),
        );
        let norm = z.re * z.re + z.im * z.im;
        sum + Complex64::new(z.re / norm, -z.im / norm)
    });
    let norm = admittance.re * admittance.re + admittance.im * admittance.im;
    Complex64::new(
        admittance.re / norm + resistance,
        -admittance.im / norm + omega * inductance,
    )
}

#[test]
fn fixed_vias_and_joint_candidates_share_one_complete_minimax_search() {
    let mut evaluated = NAMES
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let geometry = GEOMETRIES[index / BOMS.len()];
            let bom = BOMS[index % BOMS.len()];
            let trace = trace(geometry, bom);
            for ((actual, expected), hz) in trace
                .iter()
                .zip(GRID.map(|hz| independent(geometry, bom, hz)))
                .zip(GRID)
            {
                let scale = expected.re.abs().max(expected.im.abs());
                assert!(scale > 0.0, "{name} nonzero oracle at {hz} Hz");
                assert!(
                    (actual.re - expected.re).abs() <= 64.0 * f64::EPSILON * scale,
                    "{name} real at {hz} Hz"
                );
                assert!(
                    (actual.im - expected.im).abs() <= 64.0 * f64::EPSILON * scale,
                    "{name} imaginary at {hz} Hz"
                );
            }
            let (worst_index, worst) = trace
                .iter()
                .map(|z| z.re.hypot(z.im))
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(&right.1))
                .expect("invariant: declared grid is nonempty");
            let (resonance_index, resonance) = trace
                .iter()
                .map(|z| z.re.hypot(z.im))
                .enumerate()
                .min_by(|left, right| left.1.total_cmp(&right.1))
                .expect("invariant: declared grid is nonempty");
            (
                name,
                worst / TARGET,
                worst - TARGET,
                GRID[worst_index],
                resonance,
                GRID[resonance_index],
            )
        })
        .collect::<Vec<_>>();
    evaluated.sort_by(|left, right| left.1.total_cmp(&right.1));

    assert_eq!(
        evaluated.iter().map(|result| result.0).collect::<Vec<_>>(),
        RANKING
    );
    assert!(evaluated.windows(2).all(|pair| pair[0].1 <= pair[1].1));
    assert!(evaluated.iter().all(|result| result.2 > 0.0));
}

#[test]
fn rejects_nonphysical_geometry_components_and_feed_range() {
    let length = Length::from_base;
    let pair = |h, d, t, p, rho| {
        ViaPair::new(
            length(h),
            length(d),
            length(t),
            length(p),
            Resistivity::from_base(rho),
        )
    };
    assert_eq!(
        error(pair(1e-3, 0.2e-3, 0.0, 0.5e-3, RHO)),
        PdnError::Geometry
    );
    assert_eq!(
        error(pair(1e-3, 0.2e-3, 20e-6, 0.2e-3, RHO)),
        PdnError::ViaClearance
    );
    assert_eq!(
        error(pair(f64::MIN_POSITIVE, 1.0, 1.0, 2.0, f64::MIN_POSITIVE)),
        PdnError::DynamicRange
    );
    let connection = via(GEOMETRIES[0]);
    let component = |capacitance, resistance, inductance| {
        placed_capacitor(
            Capacitance::from_base(capacitance),
            ElectricalImpedance::from_base(resistance),
            Inductance::from_base(inductance),
            connection,
        )
    };
    assert_eq!(error(component(1e-6, -1e-6, 0.0)), PdnError::Component);
    assert_eq!(error(component(1e-6, 0.0, -1e-12)), PdnError::Component);
    assert_eq!(error(component(0.0, 0.0, 0.0)), PdnError::Component);
    assert_eq!(
        error(component(1e-6, f64::MAX, 0.0)),
        PdnError::DynamicRange
    );
    assert_eq!(
        error(connection.impedance(Frequency::from_base(0.0))),
        PdnError::DynamicRange
    );
    assert_eq!(
        error(connection.impedance(Frequency::from_base(f64::MAX))),
        PdnError::DynamicRange
    );
    assert_eq!(
        error(connection.impedance(Frequency::from_base(f64::from_bits(1)))),
        PdnError::DynamicRange
    );
}
