use electra_circuit::parse_file;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "electra-hierarchy-{}-{}",
            process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn write(&self, name: &str, data: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, data).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn check_origin(root: &Path, file: &Path, line: usize, msg: &str) {
    let e = parse_file(root).unwrap_err();
    assert_eq!(e.path, file.canonicalize().unwrap());
    assert_eq!(e.error.line, line);
    assert!(e.error.message.contains(msg), "{e}");
}

#[test]
fn cli_loads_relative_to_each_including_file_and_probes_current() {
    let d = Fixture::new();
    let root = d.write(
        "main.cir",
        ".include \"parts dir/library.cir\"\nV1 a 0 5\nX1 a load\n.end",
    );
    d.write("parts dir/library.cir", ".include inner/model.cir\n.end");
    d.write(
        "parts dir/inner/model.cir",
        "* shared model\n.subckt load a\nR1 a 0 1k\n.ends\n.end",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_electra"))
        .arg("op")
        .arg(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("I(X1/R1) = 5.000000000000e-3 A")
    );
}

#[test]
fn included_errors_report_the_local_line_for_binding_and_expanded_connections() {
    let d = Fixture::new();
    let root = d.write("root", ".include lib/model\nXtop a a cell");
    let model = d.write("lib/model", "\n.subckt cell p n\nR1 p n {r}\n.ends");
    check_origin(&root, &model, 3, "unknown local parameter r");
    d.write("lib/model", "\n.subckt cell p n\nR1 p n 1k\n.ends");
    check_origin(&root, &model, 3, "Xtop/R1: element endpoints must differ");
    d.write("root", ".include lib/model\nVrail a 0 1\nXtop a 0 cell");
    d.write("lib/model", "\n.subckt cell p n\n\nVbad p n 2\n.ends");
    check_origin(&root, &model, 4, "Xtop/Vbad");
    let out = Command::new(env!("CARGO_BIN_EXE_electra"))
        .arg("op")
        .arg(&root)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("model:4: Xtop/Vbad")
    );
}

#[test]
fn missing_include_and_canonical_cycles_point_to_the_requesting_card() {
    let d = Fixture::new();
    let root = d.write("root", ".include lib/model");
    let model = d.write("lib/model", "\n.include nowhere");
    check_origin(&root, &model, 2, "nowhere");
    d.write("lib/model", "\n.include ../root");
    check_origin(&root, &model, 2, "recursive include");
    d.write("lib/model", "\n.include \"unterminated");
    check_origin(&root, &model, 2, "expected one file path");
}
