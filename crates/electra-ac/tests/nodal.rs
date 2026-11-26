//! Analytical and range qualification for sparse passive AC stamps.
use core::f64::consts::TAU;
use electra_ac::{
    Capacitance, Complex64, ElectricPotential, ElectricalImpedance, Frequency, Inductance,
    nodal::{PassiveBranch, PassiveElement, StampError},
};

fn resistor(r: f64) -> PassiveElement {
    PassiveElement::resistor(ElectricalImpedance::from_base(r)).unwrap()
}

#[test]
fn cross_coupled_rc_bridge_satisfies_independent_node_equations_in_both_orientations() {
    let c = PassiveElement::capacitor(Capacitance::from_base(1.)).unwrap();
    let elements = [
        (0, 1, resistor(1.)),
        (1, 3, resistor(1.)),
        (2, 0, resistor(1.)),
        (3, 2, resistor(0.5)),
        (1, 2, c),
    ];
    let drive = Complex64::new(1., 2.);
    let boundary = [
        Some(ElectricPotential::from_base(drive)),
        None,
        None,
        Some(ElectricPotential::default()),
    ];
    for reverse in [false, true] {
        let mut matrix = [[Complex64::default(); 4]; 4];
        let mut rhs = [Complex64::default(); 4];
        for &(a, b, model) in &elements {
            let (a, b) = if reverse { (b, a) } else { (a, b) };
            PassiveBranch::new(a, b, model)
                .stamp(
                    Frequency::from_base(TAU.recip()),
                    &boundary,
                    |i, j, y| matrix[i][j] += y.into_base(),
                    |i, v| rhs[i] += v.into_base(),
                )
                .unwrap();
        }
        // At unit angular frequency the determinant is 6 + 5j.
        let denominator = Complex64::new(6., 5.);
        let v1 = drive * Complex64::new(3., 2.) / denominator;
        let v2 = drive * Complex64::new(2., 2.) / denominator;
        for i in [1, 2] {
            let residual = matrix[i][1] * v1 + matrix[i][2] * v2 - rhs[i];
            assert!(residual.re.hypot(residual.im) < 2e-15);
            assert_eq!(rhs[i], drive);
        }
        assert_eq!(matrix[1][2], Complex64::new(0., -1.));
        assert_eq!(matrix[2][1], matrix[1][2]);
    }
}

#[test]
fn handles_dc_opens_coincident_nodes_and_unrepresentable_stamps_atomically() {
    let c = PassiveElement::capacitor(Capacitance::from_base(1.)).unwrap();
    let l = PassiveElement::inductor(Inductance::from_base(1.)).unwrap();
    let boundary = [
        Some(ElectricPotential::from_base(Complex64::from(f64::MAX))),
        None,
    ];
    let check = |a, b, e, hz| {
        PassiveBranch::new(a, b, e).stamp(
            Frequency::from_base(hz),
            &boundary,
            |_, _, _| panic!("matrix emitted"),
            |_, _| panic!("rhs emitted"),
        )
    };
    assert_eq!(check(0, 1, c, 0.), Ok(()));
    assert_eq!(check(1, 1, l, 0.), Ok(()));
    assert_eq!(check(0, 1, l, 0.), Err(StampError::DcInductor));
    assert_eq!(check(2, 1, c, 1.), Err(StampError::Node(2)));
    assert_eq!(check(0, 1, resistor(0.5), 1.), Err(StampError::Range));
    assert_eq!(check(0, 1, c, f64::MAX), Err(StampError::Range));
    assert_eq!(check(0, 1, c, f64::NAN), Err(StampError::Frequency));
}

#[test]
fn reciprocal_inductive_stamp_survives_intermediate_overflow() {
    let l = PassiveElement::inductor(Inductance::from_base(2.)).unwrap();
    let mut diagonal = Complex64::default();
    PassiveBranch::new(0, 1, l)
        .stamp(
            Frequency::from_base(f64::MAX),
            &[None, Some(ElectricPotential::default())],
            |_, _, v| diagonal = v.into_base(),
            |_, _| {},
        )
        .unwrap();
    assert_eq!(diagonal.re, 0.);
    assert!((diagonal.im * f64::MAX + 0.5 / TAU).abs() < 2e-15);
}

#[test]
fn capacitive_stamp_recovers_a_representable_subnormal() {
    let c = PassiveElement::capacitor(Capacitance::from_base(0.25)).unwrap();
    let mut diagonal = Complex64::default();
    PassiveBranch::new(0, 1, c)
        .stamp(
            Frequency::from_base(f64::from_bits(1)),
            &[None, Some(ElectricPotential::default())],
            |_, _, v| diagonal = v.into_base(),
            |_, _| {},
        )
        .unwrap();
    assert_eq!(diagonal.im.to_bits(), 2);
}

#[test]
fn prescribed_ac_voltage_drives_an_inductive_rhs_and_fixed_rows_are_omitted() {
    let model = PassiveElement::inductor(Inductance::from_base(0.5)).unwrap();
    let v = Some(ElectricPotential::from_base(Complex64::new(2., 3.)));
    let f = Frequency::from_base(TAU.recip());
    let branch = PassiveBranch::new(0, 1, model);
    branch
        .stamp(
            f,
            &[v, None],
            |i, j, y| assert_eq!((i, j, y.into_base()), (1, 1, Complex64::new(0., -2.))),
            |i, b| assert_eq!((i, b.into_base()), (1, Complex64::new(6., -4.))),
        )
        .unwrap();
    branch
        .stamp(
            f,
            &[v, v],
            |_, _, _| panic!("fixed row"),
            |_, _| panic!("fixed row"),
        )
        .unwrap();
    let bad = Some(ElectricPotential::from_base(Complex64::from(f64::NAN)));
    assert_eq!(
        branch.stamp(
            f,
            &[bad, None],
            |_, _, _| unreachable!(),
            |_, _| unreachable!()
        ),
        Err(StampError::Boundary(0))
    );
}

#[test]
fn component_domains_reject_nonphysical_values() {
    for x in [-1., f64::NAN, f64::INFINITY] {
        assert_eq!(
            PassiveElement::capacitor(Capacitance::from_base(x)).unwrap_err(),
            StampError::Capacitance
        );
    }
    for x in [-1., 0., f64::INFINITY, f64::NAN] {
        assert_eq!(
            PassiveElement::resistor(ElectricalImpedance::from_base(x)).unwrap_err(),
            StampError::Resistance
        );
        assert_eq!(
            PassiveElement::inductor(Inductance::from_base(x)).unwrap_err(),
            StampError::Inductance
        );
    }
}

#[test]
fn rejects_erased_source_quadratures_before_emitting_terms() {
    let voltage = Some(ElectricPotential::from_base(Complex64::new(1., 1e-40)));
    for element in [
        resistor(1e308),
        PassiveElement::capacitor(Capacitance::from_base(1e-308)).unwrap(),
    ] {
        let branch = PassiveBranch::new(0, 1, element);
        assert_eq!(
            branch.stamp(
                Frequency::from_base(1.),
                &[voltage, None],
                |_, _, _| panic!(),
                |_, _| panic!()
            ),
            Err(StampError::Range)
        );
    }
}
