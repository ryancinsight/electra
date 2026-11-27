//! Replay native virtual-key decoding and pointer capture over two occurrences.
use electra_metis_probe::PinList;
use electra_schematic::PinId;
use metis_core::input::Key;

#[test]
fn pointer_and_keyboard_share_focus_without_committing_drags_or_cancelled_presses() {
    let rows = [1, 2].map(|instance| PinId { instance, pin: 1 });
    let mut list = PinList::new(&rows, 120., 20.).unwrap();
    list.press([10., 23.]).unwrap();
    list.release([12., 24.]).unwrap();
    assert_eq!(list.selected(), Some(rows[1]));
    assert!(list.key(Key::from_windows_virtual_key(0x26).unwrap()));
    assert_eq!(list.selected(), Some(rows[0]));
    for end in [[18., 23.], [10., 19.], [120., 23.]] {
        list.press([10., 23.]).unwrap();
        list.release(end).unwrap();
        assert_eq!(list.selected(), Some(rows[0]));
    }
    list.press([10., 23.]).unwrap();
    list.cancel_press();
    list.release([10., 23.]).unwrap();
    assert_eq!(list.selected(), Some(rows[0]));
    list.key(Key::Escape);
    assert_eq!(list.selected(), None);
}
