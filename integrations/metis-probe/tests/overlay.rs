//! Headless selection against the native field fixture.
use electra_metis_probe::overlay::RailMap;
#[test]
fn select_same_net_at_both_zooms() {
    let map = RailMap::parse(include_str!("../../../examples/field/two-rails.rail")).unwrap();
    let view = map.view();
    for scale in [24., 48.] {
        let sample = view
            .probe([100. + 1.5 * scale, 50. + 0.5 * scale], [100., 50.], scale)
            .unwrap();
        assert_eq!(sample.net.0, 7);
        assert!((sample.center[0].into_base() - 0.013).abs() < 1e-15);
        assert!((sample.center[1].into_base() - 0.021).abs() < 1e-15);
        assert!(sample.drop.into_base() > 0.2 && sample.rise.into_base() > 25.);
        assert!(view.probe([99., 50.], [100., 50.], scale).is_none());
        assert!(
            view.probe([100. + 2. * scale, 50.], [100., 50.], scale)
                .is_none()
        );
    }
    assert!(RailMap::parse("1 1 0 0 1").is_none());
}
