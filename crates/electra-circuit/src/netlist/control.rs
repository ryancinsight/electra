use super::{ParseError, magnitude};
use crate::{Circuit, Element, LinearSweep, SweepError, SweepRange, analysis::sweep::SweepPlan};
use aequitas::Quantity;

pub(super) fn parse_card(
    circuit: &Circuit,
    line: usize,
    card: &str,
) -> Result<SweepPlan, ParseError> {
    let err = |message: String| ParseError { line, message };
    let mut fields = card.split_whitespace().skip(1);
    let (Some(name), Some(start), Some(stop), Some(step), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        return Err(err("expected .dc SOURCE START STOP STEP".to_owned()));
    };
    let numbers = [start, stop, step].map(magnitude);
    let [Some(start), Some(stop), Some(step)] = numbers else {
        return Err(err(
            "expected finite sweep values in source units with optional scale factors".to_owned(),
        ));
    };
    fn range<D>(start: f64, stop: f64, step: f64) -> LinearSweep<D> {
        LinearSweep {
            start: Quantity::from_base(start),
            stop: Quantity::from_base(stop),
            step: Quantity::from_base(step),
        }
    }
    let index = circuit.source(name).map_err(|e| err(e.to_string()))?;
    let range = match circuit.parts[index].element {
        Element::Voltage { .. } => SweepRange::Voltage(range(start, stop, step)),
        Element::Current { .. } => SweepRange::Current(range(start, stop, step)),
        _ => return Err(err(SweepError::SourceType(name.to_owned()).to_string())),
    };
    circuit
        .plan_at(index, range)
        .map_err(|e| err(e.to_string()))
}
