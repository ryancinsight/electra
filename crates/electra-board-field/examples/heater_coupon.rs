//! Compare measured or independently simulated heater temperatures with a passive sheet model.
//! Supply one UTF-8 coupon file. Each sensor and the full-field peak carry a temperature in degrees Celsius and a comparison interval in kelvin.
use aequitas::systems::si::quantities::{
    Length, Power, TemperatureDifference, ThermalConductivity, ThermodynamicTemperature,
};
use std::{collections::HashSet, env, fs::File, io::Read, path::Path};

#[path = "heater_coupon/model.rs"]
mod model;
use model::{Comparison, Field, Heat, HeatTransferCoefficient, Layer, Mesh, sample, solve_mesh};

const FORMAT: &str = "electra-heater-coupon-v1";
const CELSIUS_ZERO_K: f64 = 273.15;
const MAX_FILE_BYTES: u64 = 1_048_576;
const MAX_CELLS: usize = 4_096;
const MAX_READINGS: usize = 1_024;
struct Reading {
    name: String,
    x: Length,
    y: Length,
    temperature: ThermodynamicTemperature,
    interval: TemperatureDifference,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReferenceKind {
    Measurement,
    Simulation,
}
struct Coupon {
    name: String,
    reference: (ReferenceKind, String),
    ambient: ThermodynamicTemperature,
    power: Power,
    mesh: Mesh,
    sensors: Vec<Reading>,
    peak: Reading,
}
#[derive(Default)]
struct Input {
    name: Option<String>,
    reference: Option<(ReferenceKind, String)>,
    geometry: Option<(Length, Length, usize, usize)>,
    ambient: Option<ThermodynamicTemperature>,
    power: Option<Power>,
    air: Option<HeatTransferCoefficient>,
    layers: Vec<Layer>,
    heaters: Vec<(usize, usize, f64)>,
    sensors: Vec<Reading>,
    peak: Option<Reading>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("heater coupon replay failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err("usage: heater_coupon <reference-coupon.txt>".into());
    };
    let coupon = load(Path::new(path))?;
    report(&coupon, &solve_mesh(&coupon.mesh)?)
}

fn load(path: &Path) -> Result<Coupon, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut text = String::new();
    let bytes = file
        .take(MAX_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| format!("{} is not valid UTF-8: {error}", path.display()))?;
    if u64::try_from(bytes).map_err(|_| "input length cannot be represented")? > MAX_FILE_BYTES {
        return Err(format!("input exceeds {MAX_FILE_BYTES} bytes"));
    }
    parse(&text)
}

fn parse(text: &str) -> Result<Coupon, String> {
    let mut input = Input::default();
    let mut format = false;
    let (mut layers, mut heaters, mut sensors) = (HashSet::new(), HashSet::new(), HashSet::new());
    for (offset, raw) in text.lines().enumerate() {
        let line = offset + 1;
        let content = raw.split('#').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        let fields: Vec<_> = content.split_whitespace().collect();
        match fields.as_slice() {
            ["format", value] if *value == FORMAT && !format => format = true,
            ["coupon", value] => set(&mut input.name, (*value).into(), "coupon", line)?,
            ["reference", kind, source] => {
                let kind = match *kind {
                    "measurement" => ReferenceKind::Measurement,
                    "simulation" => ReferenceKind::Simulation,
                    _ => return Err(format!("line {line}: invalid reference kind")),
                };
                set(
                    &mut input.reference,
                    (kind, (*source).into()),
                    "reference",
                    line,
                )?;
            }
            ["geometry_mm", w, h, r, c] => {
                let value = (
                    Length::from_base(parse_value::<f64>(w, line)? * 1e-3),
                    Length::from_base(parse_value::<f64>(h, line)? * 1e-3),
                    parse_value(r, line)?,
                    parse_value(c, line)?,
                );
                set(&mut input.geometry, value, "geometry", line)?;
            }
            ["ambient_c", value] => set(
                &mut input.ambient,
                ThermodynamicTemperature::from_base(
                    parse_value::<f64>(value, line)? + CELSIUS_ZERO_K,
                ),
                "ambient",
                line,
            )?,
            ["input_power_w", value] => set(
                &mut input.power,
                Power::from_base(parse_value(value, line)?),
                "input power",
                line,
            )?,
            ["air_conductance_w_m2k", value] => {
                set(
                    &mut input.air,
                    HeatTransferCoefficient::from_base(parse_value(value, line)?),
                    "air conductance",
                    line,
                )?;
            }
            ["layer_mm_w_mk", name, thickness, conductivity] if layers.insert(*name) => {
                input.layers.push(Layer {
                    thickness: Length::from_base(parse_value::<f64>(thickness, line)? * 1e-3),
                    conductivity: ThermalConductivity::from_base(parse_value(conductivity, line)?),
                });
            }
            ["heater", row, column, fraction] => {
                let location = (parse_value(row, line)?, parse_value(column, line)?);
                if !heaters.insert(location) {
                    return Err(format!("line {line}: duplicate heater cell"));
                }
                input
                    .heaters
                    .push((location.0, location.1, parse_value(fraction, line)?));
            }
            ["sensor_c", name, x, y, temperature, interval] if sensors.insert(*name) => input
                .sensors
                .push(reading(name, x, y, temperature, interval, line)?),
            ["peak_c", x, y, temperature, interval] => {
                let value = reading("full-field-peak", x, y, temperature, interval, line)?;
                set(&mut input.peak, value, "peak", line)?;
            }
            _ => {
                return Err(format!(
                    "line {line}: unknown, duplicate, or malformed record"
                ));
            }
        }
        if input.layers.len() > 128
            || input.heaters.len() > MAX_CELLS
            || input.sensors.len() > MAX_READINGS
        {
            return Err(format!("line {line}: record-count limit exceeded"));
        }
    }
    if !format {
        return Err(format!("missing `format {FORMAT}`"));
    }
    input.finish()
}

fn set<T>(slot: &mut Option<T>, value: T, name: &str, line: usize) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("line {line}: duplicate {name}"))
    } else {
        Ok(())
    }
}

fn parse_value<T: std::str::FromStr>(text: &str, line: usize) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("line {line}: invalid value `{text}`"))
}

fn reading(name: &str, x: &str, y: &str, t: &str, u: &str, line: usize) -> Result<Reading, String> {
    Ok(Reading {
        name: name.into(),
        x: Length::from_base(parse_value::<f64>(x, line)? * 1e-3),
        y: Length::from_base(parse_value::<f64>(y, line)? * 1e-3),
        temperature: ThermodynamicTemperature::from_base(
            parse_value::<f64>(t, line)? + CELSIUS_ZERO_K,
        ),
        interval: TemperatureDifference::from_base(parse_value(u, line)?),
    })
}

impl Input {
    fn finish(self) -> Result<Coupon, String> {
        let (width, height, rows, columns) = self.geometry.ok_or("missing geometry")?;
        let power = self.power.ok_or("missing input power")?;
        let ambient = self.ambient.ok_or("missing ambient")?;
        let peak = self.peak.ok_or("missing peak")?;
        let positive = |value: f64| value.is_finite() && value > 0.;
        if !positive(power.into_base())
            || !positive(ambient.into_base())
            || self.heaters.is_empty()
            || self.sensors.is_empty()
            || self
                .heaters
                .iter()
                .any(|entry| !positive(entry.2) || entry.2 > 1.)
        {
            return Err("missing, nonpositive, or nonfinite coupon input".into());
        }
        let sum: f64 = self.heaters.iter().map(|entry| entry.2).sum();
        let n = f64::from(u32::try_from(self.heaters.len()).map_err(|_| "too many heaters")?);
        if !sum.is_finite() || (sum - 1.).abs() > 2. * f64::EPSILON * n * sum.abs() {
            return Err(format!("heater fractions sum to {sum:.17}, not 1"));
        }
        let valid = |r: &Reading| {
            (Length::default()..=width).contains(&r.x)
                && (Length::default()..=height).contains(&r.y)
                && positive(r.temperature.into_base())
                && positive(r.interval.into_base())
        };
        if !self.sensors.iter().all(valid) || !valid(&peak) {
            return Err("invalid reference position, temperature, or interval".into());
        }
        Ok(Coupon {
            name: self.name.ok_or("missing coupon")?,
            reference: self.reference.ok_or("missing reference")?,
            ambient,
            power,
            mesh: Mesh {
                width,
                height,
                rows,
                columns,
                air: self.air.ok_or("missing air conductance")?,
                layers: self.layers,
                heat: self
                    .heaters
                    .into_iter()
                    .map(|(row, column, fraction)| Heat {
                        row,
                        column,
                        power: power * fraction,
                    })
                    .collect(),
            },
            sensors: self.sensors,
            peak,
        })
    }
}

fn report(c: &Coupon, field: &Field) -> Result<(), String> {
    let kind = match c.reference.0 {
        ReferenceKind::Measurement => "measurement",
        ReferenceKind::Simulation => "simulation",
    };
    println!(
        "coupon {}; ambient {:.6} °C; input {:e} W; {} sweeps; residual {:.3e} W; solver bound {:.3e} K",
        c.name,
        c.ambient.into_base() - CELSIUS_ZERO_K,
        c.power.into_base(),
        field.sweeps,
        field.residual.into_base(),
        field.solver_error.into_base()
    );
    println!("reference {kind} {}", c.reference.1);
    let mut failures = 0;
    for reading in &c.sensors {
        failures += usize::from(
            report_one(
                c,
                reading,
                &reading.name,
                sample(field, reading.x, reading.y)?,
                field,
            ) != Comparison::Within,
        );
    }
    failures += usize::from(
        report_one(
            c,
            &c.peak,
            "full-field-peak-at-coordinate",
            sample(field, c.peak.x, c.peak.y)?,
            field,
        ) != Comparison::Within,
    );
    let (index, peak) = field
        .rises
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.into_base().total_cmp(&b.1.into_base()))
        .ok_or("empty field")?;
    println!(
        "peak locations: simulated ({:.6}, {:.6}) mm; reference ({:.6}, {:.6}) mm",
        field.x_centres[index % c.mesh.columns].into_base() * 1e3,
        field.y_centres[index / c.mesh.columns].into_base() * 1e3,
        c.peak.x.into_base() * 1e3,
        c.peak.y.into_base() * 1e3
    );
    failures += usize::from(
        report_one(c, &c.peak, "full-field-peak-global", *peak, field) != Comparison::Within,
    );
    if failures == 0 {
        Ok(())
    } else {
        Err(format!(
            "{failures} comparison(s) mismatch or remain solver-unresolved"
        ))
    }
}

fn report_one(
    c: &Coupon,
    reading: &Reading,
    label: &str,
    predicted: TemperatureDifference,
    field: &Field,
) -> Comparison {
    let reference = reading.temperature - c.ambient;
    let residual = predicted - reference;
    let comparison = field.compare(residual, reading.interval);
    let status = match comparison {
        Comparison::Within => "within",
        Comparison::Outside => "outside",
        Comparison::Unresolved => "solver-unresolved",
    };
    println!(
        "{} ({:.6}, {:.6}) mm: simulated {:.9} K/{:.6} °C; reference {:.9} K; residual {:+.9} K; interval {:.9} K; solver bound {:.3e} K: {}",
        label,
        reading.x.into_base() * 1e3,
        reading.y.into_base() * 1e3,
        predicted.into_base(),
        c.ambient.into_base() + predicted.into_base() - CELSIUS_ZERO_K,
        reference.into_base(),
        residual.into_base(),
        reading.interval.into_base(),
        field.solver_error.into_base(),
        status
    );
    comparison
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDGE: &str = "format electra-heater-coupon-v1\n\
coupon synthetic\n\
reference simulation independent-solver\n\
geometry_mm 3000 1000 1 3\n\
ambient_c 26.85\n\
input_power_w 1\n\
air_conductance_w_m2k 1\n\
layer_mm_w_mk copper 1000 1\n\
heater 0 0 1\n\
sensor_c point 500 500 27.475 0.000001\n\
peak_c 2500 500 27.475 0.000001";

    #[test]
    fn published_layouts_match_independent_cell_and_peak_references() {
        let cases = [
            (
                include_str!("heater_coupon/fixtures/hannover_hilbert_fipy.txt"),
                7.22277,
            ),
            (
                include_str!("heater_coupon/fixtures/hannover_meander_fipy.txt"),
                6.27372,
            ),
        ];
        for (source, power_watts) in cases {
            let coupon = parse(source).unwrap();
            assert_eq!(coupon.power.into_base(), power_watts);
            assert_eq!(coupon.sensors.len(), coupon.mesh.rows * coupon.mesh.columns);
            let field = solve_mesh(&coupon.mesh).unwrap();
            report(&coupon, &field).unwrap();
        }
    }

    fn parse_error(text: &str) -> String {
        parse(text).err().expect("invariant: fixture is malformed")
    }

    #[test]
    fn parses_provenance_and_rejects_invalid_reference_data() {
        let coupon = parse(EDGE).unwrap();
        assert_eq!(coupon.reference.0, ReferenceKind::Simulation);
        assert_eq!(coupon.reference.1, "independent-solver");
        assert_eq!(
            parse_error("format electra-heater-coupon-v1\nreference oracle source"),
            "line 2: invalid reference kind"
        );
        assert_eq!(
            parse_error(&EDGE.replace("heater 0 0 1", "heater 0 0 1e308")),
            "missing, nonpositive, or nonfinite coupon input"
        );
        assert_eq!(
            report_one(
                &coupon,
                &coupon.sensors[0],
                "point",
                TemperatureDifference::from_base(1.),
                &solve_mesh(&coupon.mesh).unwrap(),
            ),
            Comparison::Outside
        );
        let field = solve_mesh(&coupon.mesh).unwrap();
        assert_eq!(
            report(&coupon, &field).unwrap_err(),
            "1 comparison(s) mismatch or remain solver-unresolved"
        );
    }

    #[test]
    fn scaled_reference_accepts_the_exact_rise_and_rejects_the_first_sweep() {
        // The unit and scaled equations differ only by a positive factor.
        // Their solution is [5/8, 1/4, 1/8] K, including the left-edge peak.
        let scaled = EDGE
            .replace("peak_c 2500", "peak_c 500")
            .replace("input_power_w 1", "input_power_w 1e-13")
            .replace("air_conductance_w_m2k 1", "air_conductance_w_m2k 1e-13")
            .replace("copper 1000 1", "copper 1000 1e-13");
        let coupon = parse(&scaled).unwrap();
        let field = solve_mesh(&coupon.mesh).unwrap();
        report(&coupon, &field).unwrap();
        let first_sweep = parse(&scaled.replace("27.475", "27.35")).unwrap();
        assert_eq!(
            report(&first_sweep, &field).unwrap_err(),
            "3 comparison(s) mismatch or remain solver-unresolved"
        );
        let narrow = parse(&scaled.replace("0.000001", "1e-24")).unwrap();
        assert_ne!(
            report_one(&narrow, &narrow.peak, "peak", field.rises[0], &field),
            Comparison::Within
        );
    }
}
