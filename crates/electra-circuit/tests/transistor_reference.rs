//! Pinned ngspice transfer, terminal-current and common-source amplifier qualification.
use ElectricPotential as V;
use electra_circuit::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential,
    semiconductor::{LongChannelNmos, Npn, ThermodynamicTemperature},
};
fn rows<const N: usize>(text: &str) -> impl Iterator<Item = [f64; N]> + '_ {
    text.lines().map(|line| {
        let mut values = line.split_whitespace().map(|s| s.parse::<f64>().unwrap());
        let row = std::array::from_fn(|_| values.next().unwrap());
        assert!(values.next().is_none());
        row
    })
}
fn relative(value: f64, reference: f64, floor: f64) -> f64 {
    (value - reference).abs() / reference.abs().max(floor)
}
fn mos() -> LongChannelNmos {
    LongChannelNmos::new(
        V::from_base(1.),
        V::from_base(1.),
        ElectricConductance::from_base(0.001),
    )
    .unwrap()
}
#[test]
fn npn_all_terminals() {
    let model = Npn::new(
        ElectricCurrent::from_base(1e-15),
        Dimensionless::from_base(100.),
        Dimensionless::from_base(2.),
        ThermodynamicTemperature::from_base(300.15),
    )
    .unwrap();
    let mut worst = 0.0_f64;
    let data = include_str!("../../../examples/semiconductor/npn-op.dat");
    assert_eq!(data.lines().count(), 65);
    for [vb, vc, ic, ib, ie] in rows(data) {
        let p = model.at(V::from_base(vb), V::from_base(vb - vc)).unwrap();
        let (c, b, e) = (
            p.collector.into_base(),
            p.base.into_base(),
            p.emitter.into_base(),
        );
        for (value, reference) in [(c, ic), (b, ib), (e, ie)] {
            worst = worst.max(relative(value, reference, 1e-12));
        }
        assert!((c + b + e).abs() < 1e-15 * (c.abs() + b.abs() + e.abs()).max(1e-30));
    }
    println!("NPN max relative error {worst:e}");
    assert!(worst < 0.01);
}

#[test]
fn nmos_transfer_and_common_source_gain() {
    let model = mos();
    let mut worst = 0.0_f64;
    let data = include_str!("../../../examples/semiconductor/nmos-transfer.dat");
    assert_eq!(data.lines().count(), 39);
    for [vg, vd, current, gm] in rows(data) {
        let p = model.at(V::from_base(vg), V::from_base(vd)).unwrap();
        worst = worst
            .max(relative(p.drain.into_base(), current, 1e-11))
            .max(relative(p.gm.into_base(), gm, 1e-12));
    }
    println!("NMOS max normalized error {worst:e}");
    assert!(worst < 0.01);
    // Solve the resistor load line by bisection independently of channel region.
    let (mut low, mut high) = (0., 5.);
    for _ in 0..55 {
        let vd = (low + high) * 0.5;
        let point = model.at(V::from_base(2.), V::from_base(vd)).unwrap();
        if point.drain.into_base() > (5. - vd) / 2000. {
            high = vd;
        } else {
            low = vd;
        }
    }
    assert!((low - 4.).abs() < 1e-12);
    let bias = model.at(V::from_base(2.), V::from_base(low)).unwrap();
    let gain = -bias.gm.into_base() / (0.0005 + bias.gds.into_base());
    let ac = include_str!("../../../examples/semiconductor/nmos-gain.dat");
    assert_eq!(ac.lines().count(), 3);
    for [_, real, imag] in rows(ac) {
        assert!(relative(gain, real, 1e-12) < 1e-9 && imag.abs() < 1e-12);
    }
}
