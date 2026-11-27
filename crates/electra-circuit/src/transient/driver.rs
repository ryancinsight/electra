//! Piecewise digital drive of a single RC load.
use super::{ElectricPotential, SeriesRc, StepError, Time, step};

/// Stateful RC driver with an absolute-time cursor. Call at source transitions and
/// observation times. Each interval uses the previously installed drive.
pub struct RcDriver {
    load: SeriesRc,
    time: Time,
    voltage: ElectricPotential,
    drive: ElectricPotential,
}

impl RcDriver {
    /// Start at time zero with a capacitor voltage and a held source voltage.
    pub fn new(
        load: SeriesRc,
        voltage: ElectricPotential,
        drive: ElectricPotential,
    ) -> Result<Self, StepError> {
        load.advance(voltage, drive, Time::default())?;
        Ok(Self {
            load,
            time: Time::default(),
            voltage,
            drive,
        })
    }

    /// Advance to a clock instant and install the next drive. Equal-time updates
    /// apply in caller order with no integration between them; the last drives
    /// the next interval. Returns the continuous capacitor voltage at the event.
    pub fn update(
        &mut self,
        time: Time,
        next_drive: ElectricPotential,
    ) -> Result<ElectricPotential, StepError> {
        step::interval(time)?;
        if !next_drive.into_base().is_finite() {
            return Err(StepError::State);
        }
        let voltage = self
            .load
            .advance(self.voltage, self.drive, time - self.time)?;
        self.voltage = voltage;
        self.drive = next_drive;
        self.time = time;
        Ok(voltage)
    }
}
