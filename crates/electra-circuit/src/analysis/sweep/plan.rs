use super::{Grid, SourceValue, SweepError, SweepRange};
use crate::{Circuit, Element};
use aequitas::systems::si::dimensions;

#[derive(Clone, Copy, Debug)]
pub(super) enum Axis {
    Voltage(Grid<dimensions::ElectricPotential>),
    Current(Grid<dimensions::ElectricCurrent>),
}
impl Axis {
    pub fn next(&mut self) -> Result<Option<SourceValue>, SweepError> {
        match self {
            Self::Voltage(grid) => Ok(grid.next_value()?.map(SourceValue::Voltage)),
            Self::Current(grid) => Ok(grid.next_value()?.map(SourceValue::Current)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SweepPlan {
    pub(in crate::analysis) source: usize,
    pub(super) axis: Axis,
}

impl SweepPlan {
    pub(in crate::analysis) fn next_value(&mut self) -> Result<Option<SourceValue>, SweepError> {
        self.axis.next()
    }
}

impl Circuit {
    pub(crate) fn source(&self, name: &str) -> Result<usize, SweepError> {
        self.parts
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| SweepError::UnknownSource(name.to_owned()))
    }

    pub(in crate::analysis) fn plan(
        &self,
        source: &str,
        range: SweepRange,
    ) -> Result<SweepPlan, SweepError> {
        self.plan_at(self.source(source)?, range)
    }

    pub(crate) fn plan_at(&self, index: usize, range: SweepRange) -> Result<SweepPlan, SweepError> {
        let axis = match (&self.parts[index].element, range) {
            (Element::Voltage { .. }, SweepRange::Voltage(r)) => Axis::Voltage(Grid::new(r)?),
            (Element::Current { .. }, SweepRange::Current(r)) => Axis::Current(Grid::new(r)?),
            _ => return Err(SweepError::SourceType(self.parts[index].name.clone())),
        };
        Ok(SweepPlan {
            source: index,
            axis,
        })
    }
}
