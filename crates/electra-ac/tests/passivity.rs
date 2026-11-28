//! Frequency- and time-domain evidence for reciprocal model passivity repair.
use electra_ac::{Complex64, Frequency, Time, interconnect::DelayedReciprocalModel};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

const DELAY: f64 = 0.75e-9;
const EVEN_REFERENCE: f64 = 0.65;
const ODD_REFERENCE: f64 = -0.61;
const STEPS: usize = 192;
const SAMPLE_DELAY: usize = 3;
const STEP_SECONDS: f64 = 0.25e-9;
// Bounds cover 256 complex fit operations and 192 ten-operation time steps.
const FIT_ROUNDING_BOUND: f64 = 256. * f64::EPSILON;
const ENERGY_ROUNDING_BOUND: f64 = 2_048. * f64::EPSILON * STEP_SECONDS;

fn reference(frequency: f64) -> [Complex64; 2] {
    let phase = Complex64::cis(-TAU * frequency * DELAY);
    [phase * 0.02, phase * 0.63]
}

fn fit_reference(training_frequencies: &[f64]) -> [f64; 2] {
    let mut even = 0.;
    let mut odd = 0.;
    for frequency in training_frequencies {
        let [reflection, transmission] = reference(*frequency);
        let remove_delay = Complex64::cis(TAU * frequency * DELAY);
        even += ((reflection + transmission) * remove_delay).re;
        odd += ((reflection - transmission) * remove_delay).re;
    }
    let samples = f64::from(u32::try_from(training_frequencies.len()).unwrap());
    [even / samples, odd / samples]
}

fn held_out_error(model: DelayedReciprocalModel) -> f64 {
    let mut squared_error = 0.0_f64;
    let mut values = 0.0_f64;
    for frequency in [37e6, 113e6, 241e6, 419e6] {
        let actual = model.at(Frequency::from_base(frequency)).unwrap();
        let expected = reference(frequency);
        squared_error += (actual.reflection - expected[0]).norm_sqr();
        squared_error += (actual.transmission - expected[1]).norm_sqr();
        values += 2.;
    }
    (squared_error / values).sqrt()
}

#[derive(Clone, Copy)]
struct Energy {
    source: f64,
    load: f64,
    model: f64,
    residual: f64,
    second_reflection: f64,
}

fn transient_energy(model: DelayedReciprocalModel, load_reflection: f64) -> Energy {
    let port = model.at(Frequency::from_base(0.)).unwrap();
    let reflection = port.reflection.re;
    let transmission = port.transmission.re;
    let even_gain = reflection + transmission;
    let odd_gain = reflection - transmission;
    let mut incident_source = [0.; STEPS];
    let mut incident_load = [0.; STEPS];
    let mut reflected_source = [0.; STEPS];
    let mut transmitted_load = [0.; STEPS];
    let mut model_energy = 0.;
    let mut residual = 0.;
    incident_source[0] = 1.;

    for sample in 0..STEPS {
        incident_load[sample] = load_reflection * transmitted_load[sample];
        let input = [incident_source[sample], incident_load[sample]];
        if sample + SAMPLE_DELAY < STEPS {
            let even = (input[0] + input[1]) * FRAC_1_SQRT_2;
            let odd = (input[0] - input[1]) * FRAC_1_SQRT_2;
            model_energy += ((1. - even_gain.powi(2)) * even.powi(2)
                + (1. - odd_gain.powi(2)) * odd.powi(2))
                * STEP_SECONDS;
            reflected_source[sample + SAMPLE_DELAY] =
                reflection * input[0] + transmission * input[1];
            transmitted_load[sample + SAMPLE_DELAY] =
                transmission * input[0] + reflection * input[1];
        } else {
            residual += (input[0].powi(2) + input[1].powi(2)) * STEP_SECONDS;
        }
    }
    let source = incident_source
        .iter()
        .zip(&reflected_source)
        .map(|(incident, reflected)| incident * incident - reflected * reflected)
        .sum::<f64>()
        * STEP_SECONDS;
    let load = transmitted_load
        .iter()
        .zip(&incident_load)
        .map(|(outgoing, returned)| outgoing * outgoing - returned * returned)
        .sum::<f64>()
        * STEP_SECONDS;
    Energy {
        source,
        load,
        model: model_energy,
        residual,
        second_reflection: reflected_source[2 * SAMPLE_DELAY],
    }
}

fn fitted_models() -> ([f64; 2], DelayedReciprocalModel, DelayedReciprocalModel) {
    let trained = fit_reference(&[20e6, 80e6, 160e6, 320e6]);
    let injected = [trained[0] + 0.01, trained[1] - 0.47];
    let active =
        DelayedReciprocalModel::new(Time::from_base(DELAY), injected[0], injected[1]).unwrap();
    (injected, active, active.project_passive())
}

fn odd_pulse_loss(model: DelayedReciprocalModel) -> f64 {
    let port = model.at(Frequency::from_base(0.)).unwrap();
    let incident = [FRAC_1_SQRT_2, -FRAC_1_SQRT_2];
    let outgoing = [
        port.reflection.re * incident[0] + port.transmission.re * incident[1],
        port.transmission.re * incident[0] + port.reflection.re * incident[1],
    ];
    (1. - outgoing[0].powi(2) - outgoing[1].powi(2)) * STEP_SECONDS
}

#[test]
fn modal_projection_detects_and_repairs_active_fit() {
    let trained_gains = fit_reference(&[20e6, 80e6, 160e6, 320e6]);
    let reference_fit =
        DelayedReciprocalModel::new(Time::from_base(DELAY), trained_gains[0], trained_gains[1])
            .unwrap();
    let dc_fit = reference_fit.at(Frequency::from_base(0.)).unwrap();
    assert!((dc_fit.reflection.re - 0.02).abs() < FIT_ROUNDING_BOUND);
    assert!((dc_fit.transmission.re - 0.63).abs() < FIT_ROUNDING_BOUND);

    let passive = DelayedReciprocalModel::new(Time::from_base(DELAY), 0.66, -0.94).unwrap();
    assert_eq!(passive.project_passive(), passive);
    assert!((passive.passivity_margin() - 0.06).abs() < FIT_ROUNDING_BOUND);

    let (injected_gains, fitted, repaired) = fitted_models();
    assert!((fitted.passivity_margin() + 0.08).abs() < FIT_ROUNDING_BOUND);
    assert!(
        (fitted
            .at(Frequency::from_base(241e6))
            .unwrap()
            .maximum_gain()
            - 1.08)
            .abs()
            < FIT_ROUNDING_BOUND
    );
    assert!(repaired.passivity_margin().abs() < FIT_ROUNDING_BOUND);
    for mhz in 1..=500 {
        let frequency = f64::from(mhz) * 1e6;
        assert!(
            repaired
                .at(Frequency::from_base(frequency))
                .unwrap()
                .maximum_gain()
                <= 1. + FIT_ROUNDING_BOUND
        );
    }

    let before = held_out_error(fitted);
    let after = held_out_error(repaired);
    let expected_before = ((injected_gains[0] - EVEN_REFERENCE).powi(2)
        + (injected_gains[1] - ODD_REFERENCE).powi(2))
    .sqrt()
        * 0.5;
    let expected_after =
        ((injected_gains[0] - EVEN_REFERENCE).powi(2) + (-1. - ODD_REFERENCE).powi(2)).sqrt() * 0.5;
    assert!((before - expected_before).abs() < FIT_ROUNDING_BOUND);
    assert!((after - expected_after).abs() < FIT_ROUNDING_BOUND);
    assert!(expected_before > expected_after);
    assert!(
        ((before - after) - (expected_before - expected_after)).abs() < 2. * FIT_ROUNDING_BOUND
    );

    assert_eq!(
        DelayedReciprocalModel::new(Time::from_base(-1e-9), 0., 0.),
        None
    );
    assert_eq!(
        DelayedReciprocalModel::new(Time::from_base(0.), f64::NAN, 0.),
        None
    );
    assert_eq!(
        DelayedReciprocalModel::new(Time::from_base(0.), f64::INFINITY, 0.),
        None
    );
    assert_eq!(
        repaired
            .at(Frequency::from_base(f64::INFINITY))
            .map(|port| (port.reflection, port.transmission)),
        None
    );
    let short = DelayedReciprocalModel::new(Time::from_base(0.), 1., -1.)
        .unwrap()
        .at(Frequency::from_base(f64::MAX))
        .unwrap();
    assert_eq!(short.reflection, Complex64::from(0.));
    assert_eq!(short.transmission, Complex64::from(1.));
    let extreme = DelayedReciprocalModel::new(Time::from_base(0.), f64::MAX, -f64::MAX)
        .unwrap()
        .project_passive()
        .at(Frequency::from_base(1e9))
        .unwrap();
    assert_eq!(extreme.reflection, Complex64::from(0.));
    assert_eq!(extreme.transmission, Complex64::from(1.));
    assert_eq!(
        repaired
            .at(Frequency::from_base(-1.))
            .map(|port| (port.reflection, port.transmission)),
        None
    );
}

#[test]
fn passive_terminations_absorb_no_more_than_source_energy() {
    let (_, active, repaired) = fitted_models();
    let active_odd_loss = (1. - 1.08_f64.powi(2)) * STEP_SECONDS;
    let observed_active_odd_loss = odd_pulse_loss(active);
    assert!((observed_active_odd_loss - active_odd_loss).abs() < ENERGY_ROUNDING_BOUND);
    assert!(observed_active_odd_loss < 0.);
    assert!(odd_pulse_loss(repaired).abs() < ENERGY_ROUNDING_BOUND);

    for load_reflection in [0.55_f64, -0.45] {
        let energy = transient_energy(repaired, load_reflection);
        let denominator = 1. - (0.17 * load_reflection).powi(2);
        let expected_load =
            STEP_SECONDS * (1. - load_reflection.powi(2)) * 0.83_f64.powi(2) / denominator;
        let tail_exponent = i32::try_from(2 * (STEPS / SAMPLE_DELAY - 2)).unwrap();
        let expected_residual = STEP_SECONDS
            * load_reflection.powi(2)
            * 0.83_f64.powi(2)
            * (0.17 * load_reflection).abs().powi(tail_exponent);
        assert!((energy.load - expected_load).abs() < ENERGY_ROUNDING_BOUND);
        // The last return is tiny compared with the injected pulse. Scale the
        // rounding budget to that return so that dropping it fails this check.
        assert!(
            (energy.residual - expected_residual).abs() < 2_048. * f64::EPSILON * expected_residual
        );
        assert!(energy.source > energy.load);
        assert!(energy.model > 0.);
        assert!(
            (energy.source - energy.load - energy.model - energy.residual).abs()
                < ENERGY_ROUNDING_BOUND
        );
        assert_eq!(
            energy.second_reflection.is_sign_positive(),
            load_reflection.is_sign_positive()
        );
    }
    let high = transient_energy(repaired, 0.55);
    let low = transient_energy(repaired, -0.45);
    assert!(high.load < low.load);
    assert!(high.second_reflection > 0.);
    assert!(low.second_reflection < 0.);
}
