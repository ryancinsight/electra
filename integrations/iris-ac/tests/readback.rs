//! Public API qualification for spectral readback.
use electra_ac::{Dimensionless, ElectricPotential, Frequency, noise::VoltageNoisePsd};
use electra_iris_ac::{
    readback::{GainDb, VoltageDbv, VoltageNoiseAsd},
    rms_voltage_spectrum, voltage_noise_spectrum,
};

#[test]
fn reads_levels_and_density_at_the_exported_frequency() {
    let f = [100., 1000.].map(Frequency::from_base);
    let rms = [1., 0.1].map(ElectricPotential::from_base);
    let psd = [0., 4e-18].map(VoltageNoisePsd::from_base);
    let voltage = rms_voltage_spectrum(&f, &rms).unwrap();
    let noise = voltage_noise_spectrum(&f, &psd).unwrap();
    assert!(core::ptr::eq(voltage.samples(), &rms[..]));
    assert!(core::ptr::eq(voltage.coordinates(), noise.coordinates()));
    assert_eq!(voltage.coordinates()[1].into_base(), 1000.);
    let level = |i| {
        VoltageDbv::from_rms(voltage.samples()[i])
            .unwrap()
            .decibels_relative_to_one_volt()
    };
    assert_eq!(level(0), 0.);
    assert!((level(1) + 20.).abs() < 1e-12);
    let gain = GainDb::from_magnitude(Dimensionless::from_base(2.)).unwrap();
    assert!((gain.decibels() - 6.020599913279624).abs() < 1e-12);
    let asd = VoltageNoiseAsd::from_psd(noise.samples()[1]).unwrap();
    assert!((asd.volts_per_root_hertz() / 2e-9 - 1.).abs() < 1e-14);
    assert!(rms_voltage_spectrum(&f, &rms[..1]).is_err());
}

#[test]
fn defines_zero_and_invalid_input_behavior() {
    for x in [-1., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(VoltageDbv::from_rms(ElectricPotential::from_base(x)).is_none());
        assert!(GainDb::from_magnitude(Dimensionless::from_base(x)).is_none());
        assert!(VoltageNoiseAsd::from_psd(VoltageNoisePsd::from_base(x)).is_none());
    }
    assert_eq!(
        GainDb::from_magnitude(Dimensionless::default())
            .unwrap()
            .decibels(),
        f64::NEG_INFINITY
    );
    assert_eq!(
        VoltageDbv::from_rms(ElectricPotential::default())
            .unwrap()
            .decibels_relative_to_one_volt(),
        f64::NEG_INFINITY
    );
    assert_eq!(
        VoltageNoiseAsd::from_psd(VoltageNoisePsd::default())
            .unwrap()
            .volts_per_root_hertz(),
        0.
    );
}
