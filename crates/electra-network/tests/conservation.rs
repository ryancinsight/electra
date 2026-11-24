//! Conservation, boundary elimination and failure contracts.
use electra_network::{Branch, Error, SolveOptions, solve};

#[test]
fn monomorphized_stamps_eliminate_either_boundary() {
    let branch = Branch {
        from: 4,
        to: 7,
        conductance: 2.0_f32,
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
            fixed,
            |i, j, g| matrix.push((i, j, g)),
            |i, q| rhs.push((i, q)),
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
            conductance: 2e-9,
        },
        Branch {
            from: 1,
            to: 2,
            conductance: 1e-9,
        },
    ];
    let point = solve(
        &[Some(100.), None, Some(10.)],
        &[0., 3e-9, 0.],
        &branches,
        Default::default(),
    )
    .unwrap();
    assert!((point.potentials[1] - 71.).abs() < 1e-12);
    assert!((point.reactions[0] - 58e-9).abs() < 1e-20);
    assert!((point.reactions[2] + 61e-9).abs() < 1e-20);
}

#[test]
fn floating_islands_invalid_branches_and_overflow_are_errors() {
    let branch = |a, b, g| Branch {
        from: a,
        to: b,
        conductance: g,
    };
    let fixed = [Some(0.), None, None];
    let run =
        |edges: &[Branch<f64>]| solve(&fixed, &[0.; 3], edges, Default::default()).unwrap_err();
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
        conductance: 1.,
    };
    let err = solve(
        &[Some(1.), None, None, Some(0.)],
        &[0.; 4],
        &[branch(0, 1), branch(1, 2), branch(2, 3)],
        SolveOptions {
            max_sweeps: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err,Error::NoConvergence { sweeps:1,max_residual } if max_residual > 0.));
}

#[test]
fn disconnected_but_individually_referenced_components_are_defined() {
    let r = solve(
        &[Some(2.), None, Some(-3.), None],
        &[0., 0.5, 0., -0.25],
        &[
            Branch {
                from: 0,
                to: 1,
                conductance: 1.,
            },
            Branch {
                from: 2,
                to: 3,
                conductance: 2.,
            },
        ],
        Default::default(),
    )
    .unwrap();
    assert_eq!(r.potentials, [2., 2.5, -3., -3.125]);
    assert_eq!(r.reactions, [-0.5, 0., 0.25, 0.]);
}
