"""Compare the Electra CLI to the pinned Kwavers kernel and a complex circuit reduction."""
import argparse
import csv
import io
import math
from pathlib import Path
import subprocess
import tempfile

# crates/kwavers-driver/src/physics/pdn/impedance.rs at Kwavers daf3842e.
DRIVER_BLOB = "3a7bc6c1834d28ba698587c31cb665153c1f79bd"
BULK = (10e-6, 20e-3, 10e-9)
LOCAL = (100e-9, 50e-3, 0.5e-9)
MODELS = [[LOCAL], [BULK, LOCAL], [BULK, LOCAL, LOCAL]]
REFERENCE_MAIN = """
mod impedance;
use std::io::{self, BufRead};
fn main() {
    let args: Vec<f64> = std::env::args().skip(1).map(|arg| arg.parse().unwrap()).collect();
    let bank: Vec<_> = args.chunks_exact(3).map(|p| (p[0], p[1], p[2])).collect();
    for line in io::stdin().lock().lines() {
        let frequency = line.unwrap().parse().unwrap();
        println!("{}", impedance::pdn_impedance_at_freq(&bank, frequency));
    }
}
"""


def series_parallel(model, frequency):
    omega = math.tau * frequency
    parts = [complex(r, omega * l - 1 / (omega * c)) for c, r, l in model]
    value = parts[0]
    for branch in parts[1:]:
        value = value * branch / (value + branch)
    return value


def verify(binary, kwavers):
    source = subprocess.check_output(["git", "-C", kwavers, "cat-file", "blob", DRIVER_BLOB])
    worst_driver = worst_complex = 0.0
    count = 0
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / "impedance.rs").write_bytes(source)
        (root / "main.rs").write_text(REFERENCE_MAIN)
        driver = root / "driver"
        subprocess.run(["rustc", "--edition=2021", "-Adead_code", "-O", root / "main.rs",
                        "-o", driver], check=True)
        for model in MODELS:
            parts = [str(value) for part in model for value in part]
            for start, stop in [(100, 100_000_000), (100_000_000, 100)]:
                stdout = subprocess.check_output([binary, str(start), str(stop), "101", *parts], text=True)
                rows = list(csv.DictReader(io.StringIO(stdout)))
                frequencies = [float(row["frequency_Hz"]) for row in rows]
                if len(rows) != 101 or frequencies[0] != start or frequencies[-1] != stop:
                    raise ValueError("unexpected frequency grid")
                magnitudes = subprocess.check_output([driver, *parts], text=True,
                    input="".join(f"{f}\n" for f in frequencies)).splitlines()
                for row, f, legacy in zip(rows, frequencies, magnitudes, strict=True):
                    if row["port"] != "finite":
                        raise ValueError(f"unexpected open port at {f} Hz")
                    magnitude = float(row["z_magnitude_ohm"])
                    computed = complex(float(row["z_re_ohm"]), float(row["z_im_ohm"]))
                    expected = series_parallel(model, f)
                    driver_error = abs(magnitude / float(legacy) - 1)
                    complex_error = abs(computed - expected) / abs(expected)
                    if not (driver_error < 1e-12 and complex_error < 1e-12):
                        raise ValueError(f"at {f} Hz: driver {driver_error}, complex {complex_error}")
                    worst_driver = max(worst_driver, driver_error)
                    worst_complex = max(worst_complex, complex_error)
                    count += 1
    print(f"{count} points passed; maximum relative errors against Kwavers {worst_driver:.3g}, "
          f"complex parallel reduction {worst_complex:.3g}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="built electra-pdn executable")
    parser.add_argument("kwavers", type=Path, help="Kwavers checkout containing the reference blob")
    args = parser.parse_args()
    verify(args.binary.resolve(), args.kwavers.resolve())
