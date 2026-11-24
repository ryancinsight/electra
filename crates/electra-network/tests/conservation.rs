//! Conservation, boundary elimination and failure contracts.
use aequitas::systems::si::{
    dimensions,
    quantities::{
        ElectricConductance, ElectricCurrent, ElectricPotential, HydraulicConductance, Pressure,
        VolumetricFlowRate,
    },
};
use electra_network::{Branch, Error, SolveOptions, solve};

#[test]
fn monomorphized_stamps_eliminate_either_boundary() {
    let branch = Branch {
        from: 4,
        to: 7,
        conductance: ElectricConductance::from_base(2.0_f32),
    };
    for (fixed, expected_matrix, expected_rhs) in [
        (
            [None, None],
            vec![(4, 4, 2.), (4, 7, -2.), (7, 7, 2.), (7, 4, -2.)],
            vec![],
        ),
        ([Some(3.), None], vec![(7, 7, 2.)], vec![(7, 6.)]),
        ([None, Some(-4.)], vec![(4, 4, 2.)], vec![(4, -8.)]),
        ([Some(3.), Some(-4.)], vec![], vec![]),
    ] {
        let (mut matrix, mut rhs) = (vec![], vec![]);
        branch.stamp(
            fixed.map(|v| v.map(ElectricPotential::from_base)),
            |i, j, g| matrix.push((i, j, g.into_base())),
            |i, q| rhs.push((i, ElectricCurrent::<f32>::into_base(q))),
        );
        assert_eq!(matrix, expected_matrix);
        assert_eq!(rhs, expected_rhs);
    }
}

#[test]
fn hydraulic_network_with_a_flow_source_and_two_pressure_boundaries() {
    // Pressure in Pa, conductance in m³/(s·Pa), injections in m³/s.
    let branches = [
        Branch {
            from: 0,
            to: 1,
            conductance: HydraulicConductance::from_base(2e-9),
        },
        Branch {
            from: 1,
            to: 2,
            conductance: HydraulicConductance::from_base(1e-9),
        },
    ];
    let point = solve(
        &[Some(100.), None, Some(10.)].map(|p| p.map(Pressure::from_base)),
        &[0., 3e-9, 0.].map(VolumetricFlowRate::from_base),
        &branches,
        Default::default(),
    )
    .unwrap();
    assert!((point.potentials[1].into_base() - 71.).abs() < 1e-12);
    assert!((point.reactions[0].into_base() - 58e-9).abs() < 1e-20);
    assert!((point.reactions[2].into_base() + 61e-9).abs() < 1e-20);
}

#[test]
fn floating_islands_invalid_branches_and_overflow_are_errors() {
    let branch = |a, b, g| Branch {
        from: a,
        to: b,
        conductance: ElectricConductance::from_base(g),
    };
    let fixed = [Some(ElectricPotential::default()), None, None];
    let run = |edges: &[Branch<dimensions::ElectricConductance>]| {
        solve(
            &fixed,
            &[ElectricCurrent::default(); 3],
            edges,
            Default::default(),
        )
        .unwrap_err()
    };
    assert!(matches!(run(&[branch(1, 2, 1.)]), Error::Floating(_)));
    for edge in [
        branch(1, 1, 1.),
        branch(0, 3, 1.),
        branch(0, 1, 0.),
        branch(0, 1, f64::NAN),
    ] {
        assert_eq!(run(&[edge]), Error::InvalidBranch(0));
    }
    assert_eq!(
        run(&[
            branch(0, 1, f64::MAX),
            branch(0, 1, f64::MAX),
            branch(1, 2, 1.)
        ]),
        Error::Arithmetic
    );
}

#[test]
fn a_finite_iterate_is_insufficient_without_conservation() {
    let branch = |from, to| Branch {
        from,
        to,
        conductance: ElectricConductance::from_base(1.),
    };
    let err = solve(
        &[Some(1.), None, None, Some(0.)].map(|v| v.map(ElectricPotential::from_base)),
        &[ElectricCurrent::default(); 4],
        &[branch(0, 1), branch(1, 2), branch(2, 3)],
        SolveOptions {
            max_sweeps: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(err,Error::NoConvergence { sweeps:1,max_residual } if max_residual > ElectricCurrent::default())
    );
}

#[test]
fn disconnected_but_individually_referenced_components_are_defined() {
    let r = solve(
        &[Some(2.), None, Some(-3.), None].map(|v| v.map(ElectricPotential::from_base)),
        &[0., 0.5, 0., -0.25].map(ElectricCurrent::from_base),
        &[
            Branch {
                from: 0,
                to: 1,
                conductance: ElectricConductance::from_base(1.),
            },
            Branch {
                from: 2,
                to: 3,
                conductance: ElectricConductance::from_base(2.),
            },
        ],
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        r.potentials,
        [2., 2.5, -3., -3.125].map(ElectricPotential::from_base)
    );
    assert_eq!(
        r.reactions,
        [-0.5, 0., 0.25, 0.].map(ElectricCurrent::from_base)
    );
}

#[test]
fn prepared_topology_rejects_mask_changes_and_recovers_from_failures() {
    use electra_network::PreparedNetwork;
    let edges: Vec<_> = (0..3)
        .map(|i| Branch {
            from: i,
            to: i + 1,
            conductance: ElectricConductance::from_base(1.0),
        })
        .collect();
    let mut fixed = [Some(1.), None, None, Some(0.5)].map(|v| v.map(ElectricPotential::from_base));
    let inj = [0., 0.25, -0.125, 0.].map(ElectricCurrent::from_base);
    let mut prepared = PreparedNetwork::new(&fixed, &edges).unwrap();
    let bad_budget = SolveOptions {
        max_sweeps: 1,
        ..Default::default()
    };
    assert!(matches!(
        prepared.solve(&fixed, &inj, bad_budget),
        Err(Error::NoConvergence { .. })
    ));
    fixed.swap(0, 1);
    assert!(matches!(
        prepared.solve(&fixed, &inj, Default::default()),
        Err(Error::InvalidInput)
    ));
    fixed.swap(0, 1);
    assert!(matches!(
        prepared.solve(&fixed, &inj[..3], Default::default()),
        Err(Error::InvalidInput)
    ));
    for change in [3.0, -1.0, 0.0] {
        fixed[3] = Some(ElectricPotential::from_base(change));
        let p = prepared.solve(&fixed, &inj, Default::default()).unwrap();
        let q = solve(&fixed, &inj, &edges, Default::default()).unwrap();
        assert_eq!(p.potentials, q.potentials);
        assert_eq!(p.reactions, q.reactions);
        assert_eq!(p.sweeps, q.sweeps);
        assert_eq!(p.max_residual, q.max_residual);
    }
}

#[test]
fn extra_conservation_checks_can_reject_a_candidate_without_poisoning_the_workspace() {
    let fixed = [Some(ElectricPotential::default()), None];
    let edges = [Branch {
        from: 0,
        to: 1,
        conductance: ElectricConductance::from_base(1.0),
    }];
    let rhs = [ElectricCurrent::default(); 2];
    let mut network = electra_network::PreparedNetwork::new(&fixed, &edges).unwrap();
    for invalid in [f64::NAN, -1.0, f64::INFINITY] {
        assert!(matches!(
            network.solve_checked(&fixed, &rhs, Default::default(), |_| Ok(Some(
                ElectricCurrent::from_base(invalid)
            ))),
            Err(Error::Arithmetic)
        ));
    }
    let point = network
        .solve_checked(&fixed, &rhs, Default::default(), |p| {
            Ok((p.sweeps == 1).then_some(ElectricCurrent::from_base(1.0)))
        })
        .unwrap();
    assert_eq!(point.sweeps, 2);
    assert_eq!(
        network
            .solve(&fixed, &rhs, Default::default())
            .unwrap()
            .sweeps,
        1
    );
}
