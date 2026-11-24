use super::near;
use electra_circuit::{
    ElectricConductance, ElectricCurrent, ElectricPotential, SolveError, SolveOptions, parse,
};
use electra_network::{Branch, solve};

#[test]
fn original_kcl_can_require_more_iterations_than_reduced_kcl() {
    // In the original circuit all resistor currents are zero. In reduced coordinates
    // the equivalent excitation is -1000 A at z and +3000 A at x.
    let c = parse("Vbias bias 0 1000\nRz z 0 1\nVcell x y 1000\nRx x bias 1\nRy y 0 1\nRyz y z 1")
        .unwrap();
    let edges = [(0, 1), (0, 2), (0, 2), (1, 2)].map(|(from, to)| Branch {
        from,
        to,
        conductance: ElectricConductance::from_base(1.),
    });
    let reduced = solve(
        &[Some(ElectricPotential::default()), None, None],
        &[0., -1000., 3000.].map(ElectricCurrent::from_base),
        &edges,
        Default::default(),
    )
    .unwrap();
    let budget = SolveOptions {
        max_sweeps: reduced.sweeps,
        ..Default::default()
    };
    assert!(
        matches!(c.operating_point(budget),Err(SolveError::NoConvergence {sweeps,max_residual})
        if sweeps==reduced.sweeps && max_residual.into_base()>1e-12)
    );
    let full = c.operating_point(Default::default()).unwrap();
    assert!(full.sweeps > reduced.sweeps);
    for &(_, i) in &full.amperes {
        near(i, 0.);
    }
    assert!(full.residual_amperes.into_base() <= 1.01e-12);
}
