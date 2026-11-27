//! Cross-platform replay of the native editor's exact command and selection paths.
use electra_metis_probe::workbench::Workbench;
use electra_schematic::Schematic;
use std::process::Command;

#[test]
fn repair_two_sheets_and_compare_reopened_cursor_with_exported_cli_deck() {
    let project =
        std::env::temp_dir().join(format!("electra-workbench-{}.elcp", std::process::id()));
    let mut model = Schematic::default();
    let mut w = Workbench::new(&mut model);
    let run = |w: &mut Workbench<'_>, cmd| w.execute(cmd, &project).unwrap();
    let select = |w: &mut Workbench<'_>, row: u32| {
        let point = [25., f64::from(row * 22 + 7)];
        w.press(point).unwrap();
        w.release(point).unwrap();
    };
    run(&mut w, "place A");
    run(&mut w, "place B");
    run(&mut w, "bus taps 4 2");
    run(&mut w, "bus taps 2 4");
    assert!(w.execute("bus taps 1 1", &project).is_err());
    run(&mut w, "undo");
    select(&mut w, 6);
    run(&mut w, "wire 1");
    select(&mut w, 4);
    run(&mut w, "wire off");
    let faults = run(&mut w, "erc");
    assert!(faults.contains("OutputConflict") && faults.contains("Unconnected"));
    assert!(w.execute("simulate", &project).is_err());
    run(&mut w, "wire 2");
    select(&mut w, 6);
    run(&mut w, "wire 3");
    assert_eq!(run(&mut w, "erc"), "ERC []");
    select(&mut w, 4);
    run(&mut w, "label A/TAP");
    run(&mut w, "bind R2k 1 2");
    run(&mut w, "save");
    let before = std::fs::read(&project).unwrap();
    run(&mut w, "undo");
    run(&mut w, "redo");
    run(&mut w, "save");
    assert_eq!(std::fs::read(&project).unwrap(), before);
    run(&mut w, "open");
    select(&mut w, 4);
    assert_eq!(
        run(&mut w, "inspect"),
        "sheet A part 3 pin 1 model R2k [1, 2]"
    );
    assert_eq!(run(&mut w, "lanes taps"), "lanes 4 2");
    assert_eq!(run(&mut w, "junction 2"), "pins 2.2 3.1");
    assert!(w.execute("place A", &project).is_err());
    run(&mut w, "export");
    let first = run(&mut w, "simulate");
    let second = run(&mut w, "cursor 1");
    let third = run(&mut w, "cursor 2");
    let csv = Command::new(env!("CARGO"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args([
            "run",
            "--quiet",
            "--locked",
            "-p",
            "electra-circuit",
            "--bin",
            "electra",
            "--",
            "dc",
        ])
        .arg(project.with_extension("cir"))
        .output()
        .unwrap();
    assert!(
        csv.status.success(),
        "{}",
        String::from_utf8_lossy(&csv.stderr)
    );
    let output = String::from_utf8(csv.stdout).unwrap();
    let numbers = |prefix: &str| {
        output
            .lines()
            .filter_map(|l| l.strip_prefix(prefix))
            .map(|v| v.split_whitespace().next().unwrap().parse::<f64>().unwrap())
            .collect::<Vec<_>>()
    };
    let volts = numbers("V(2) = ");
    let amps = numbers("I(R3) = ");
    assert_eq!((volts.len(), amps.len()), (3, 3));
    for (index, sample) in [first, second, third].iter().enumerate() {
        assert!(sample.starts_with("A/TAP drive="));
        let field = |prefix| {
            sample
                .split_whitespace()
                .find_map(|v| v.strip_prefix(prefix))
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };
        assert_eq!(field("drive="), (index + 1) as f64);
        assert!((field("voltage=") - volts[index]).abs() < 1e-11);
        assert!((field("current=") - amps[index]).abs() < 1e-14);
    }
    std::fs::remove_file(project.with_extension("cir")).unwrap();
    std::fs::remove_file(project).unwrap();
}
