//! R9K data types

use std::fmt::{Display, Formatter};

use chrono::{Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::Result;
use crate::error::Error;

const MAX_DELAY_SECS: i64 = 60;
const MIN_DELAY_SECS: i64 = -30;

/// R9K train update message as deserialized from the XML received from
/// KiwiRail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct R9kMessage {
    /// The train update.
    #[serde(rename(deserialize = "ActualizarDatosTren"))]
    pub train_update: TrainUpdate,
}

impl TryFrom<String> for R9kMessage {
    type Error = Error;

    fn try_from(xml: String) -> anyhow::Result<Self, Self::Error> {
        quick_xml::de::from_str(&xml).map_err(Into::into)
    }
}

impl TryFrom<&[u8]> for R9kMessage {
    type Error = Error;

    fn try_from(xml: &[u8]) -> anyhow::Result<Self, Self::Error> {
        quick_xml::de::from_reader(xml).map_err(Into::into)
    }
}

/// R9000 (R9K) train update as received from KiwiRail.
/// Defines the XML mappings as defined by the R9K provider - in Spanish.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrainUpdate {
    /// Train ID for even trains.
    #[serde(rename(deserialize = "trenPar"))]
    pub even_train_id: Option<String>,

    /// Train ID for odd trains.
    #[serde(rename(deserialize = "trenImpar"))]
    pub odd_train_id: Option<String>,

    /// The creation date of the train update.
    #[serde(rename(deserialize = "fechaCreacion"))]
    #[serde(deserialize_with = "r9k_date")]
    pub created_date: NaiveDate,

    /// Train's registration number.
    #[serde(rename(deserialize = "numeroRegistro"))]
    pub registration_number: String,

    /// Type of train.
    #[serde(rename(deserialize = "operadorComercial"))]
    pub train_type: TrainType,

    /// Train type code.
    #[serde(rename(deserialize = "codigoOperadorComercial"))]
    pub train_type_code: String,

    /// Full train
    #[serde(rename(deserialize = "trenCompleto"))]
    pub full_train: Option<String>,

    /// Source of the train update.
    #[serde(rename(deserialize = "origenActualizaTren"))]
    pub source: String,

    /// Changes to train trip by station.     
    ///
    /// The list includes one entry for the station that the train has arrived
    /// at, with additional entries for stations not yet visited.
    ///
    /// N.B. Only the first entry is used as the remainder are a schedule only.
    #[serde(rename(deserialize = "pasoTren"), default)]
    pub changes: Vec<Change>,
}

fn r9k_date<'de, D>(deserializer: D) -> anyhow::Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, "%d/%m/%Y").map_err(serde::de::Error::custom)
}

impl TrainUpdate {
    /// Get the train ID, preferring even over odd.
    #[must_use]
    pub fn train_id(&self) -> String {
        self.even_train_id.clone().unwrap_or_else(|| self.odd_train_id.clone().unwrap_or_default())
    }

    /// Validate the message.
    ///
    /// # Errors
    ///
    /// Will return one of the following errors:
    ///  - `Error::NoUpdate` if there are no changes
    ///  - `Error::NoActualUpdate` if the arrival or departure time is -ve or 0
    ///  - `Error::Outdated` if the message is too old
    ///  - `Error::WrongTime` if the message is from the future
    pub fn validate(&self) -> Result<()> {
        if self.changes.is_empty() {
            return Err(Error::NoUpdate);
        }

        // an *actual* update will have a +ve arrival or departure time
        let change = &self.changes[0];
        let from_midnight_secs = if change.has_departed {
            change.actual_departure_time
        } else if change.has_arrived {
            change.actual_arrival_time
        } else {
            return Err(Error::NoActualUpdate);
        };

        if from_midnight_secs <= 0 {
            return Err(Error::NoActualUpdate);
        }

        // check for outdated message
        let naive_time = self.created_date.and_hms_opt(0, 0, 0).unwrap_or_default();
        let Some(local_time) = Local.from_local_datetime(&naive_time).earliest() else {
            return Err(Error::WrongTime(format!("invalid local time: {naive_time}")));
        };

        let midnight_ts = local_time.timestamp();
        let event_ts = midnight_ts + i64::from(from_midnight_secs);
        let delay_secs = Local::now().timestamp() - event_ts;

        // TODO: do we need this metric?;
        tracing::info!(gauge.r9k_delay = delay_secs);

        if delay_secs > MAX_DELAY_SECS {
            return Err(Error::Outdated(format!("message delayed by {delay_secs} seconds")));
        }
        if delay_secs < MIN_DELAY_SECS {
            return Err(Error::WrongTime(format!("message ahead by {delay_secs} seconds")));
        }

        Ok(())
    }
}

/// R9K train update change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Type of change that triggered the update message.
    #[serde(rename(deserialize = "tipoCambio"))]
    pub r#type: ChangeType,

    /// Station identifier.
    #[serde(rename(deserialize = "estacion"))]
    pub station: u32,

    /// Unique id for the entry.
    #[serde(rename(deserialize = "idPaso"))]
    pub entry_id: String,

    /// Scheduled arrival time as per schedule.
    /// In seconds from train update creation date at midnight.
    #[serde(rename(deserialize = "horaEntrada"))]
    pub arrival_time: i32,

    /// Actual arrival, or estimated arrival time (based on the latest actual
    /// arrival or departure time of the preceding stations).
    ///
    /// In seconds from train update creation date at midnight. `-1` if not
    /// available.
    #[serde(rename(deserialize = "horaEntradaReal"))]
    pub actual_arrival_time: i32,

    /// The train has arrived.
    #[serde(rename(deserialize = "haEntrado"))]
    pub has_arrived: bool,

    /// Difference between the actual and scheduled arrival times if the train
    /// has already arrived at the station, 0 otherwise.
    #[serde(rename(deserialize = "retrasoEntrada"))]
    pub arrival_delay: i32,

    /// Scheduled departure time as per schedule.
    ///
    /// In seconds from train update creation date at midnight.
    #[serde(rename(deserialize = "horaSalida"))]
    pub departure_time: i32,

    /// Actual departure, or estimated departure time (based on the latest
    /// actual arrival or departure time of the preceding stations).
    ///
    /// In seconds from train update creation date at midnight. -1 if not
    /// available.
    #[serde(rename(deserialize = "horaSalidaReal"))]
    pub actual_departure_time: i32,

    /// The train has departed.
    #[serde(rename(deserialize = "haSalido"))]
    pub has_departed: bool,

    /// Difference between the actual and scheduled arrival times if the train
    /// has already arrived at the station, 0 otherwise.
    #[serde(rename(deserialize = "retrasoSalida"))]
    pub departure_delay: i32,

    /// The time at which the train was detained.
    #[serde(rename(deserialize = "horaInicioDetencion"))]
    pub detention_time: i32,

    /// The duration for which the train was detained.
    #[serde(rename(deserialize = "duracionDetencion"))]
    pub detention_duration: i32,

    /// The platform at which the train arrived.
    #[serde(rename(deserialize = "viaEntradaMallas"))]
    pub platform: String,

    /// The exit line from a station.
    #[serde(rename(deserialize = "viaCirculacionMallas"))]
    pub exit_line: String,

    /// Train direction in reference to the platform.
    #[serde(rename(deserialize = "sentido"))]
    pub train_direction: Direction,

    /// Should be an enum, but again, we don't have the full list.
    /// 4 - Original, Passing (non-stop/skip), or Destination (no dwell time in timetable)
    /// 5 - Intermediate stop (there is a dwell time in the time table).
    #[serde(rename(deserialize = "tipoParada"))]
    pub stop_type: StopType,

    /// N.B. Not sure what this is used for.
    #[serde(rename(deserialize = "paridad"))]
    pub parity: String,
}

/// The type of change that triggered the update message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ChangeType {
    /// Train has exited the first station.
    ExitedFirstStation = 1,

    /// Train has reached the final destination.
    ReachedFinalDestination = 2,

    /// Train has arrived at the station.
    ArrivedAtStation = 3,

    /// Train has exited the station.
    ExitedStation = 4,

    /// Train has passed the station without stopping.
    PassedStationWithoutStopping = 5,

    /// Train has been parked between stations.
    DetainedInPark = 6,

    /// Train has been detained at the station.
    DetainedAtStation = 7,

    /// Station is no longer part of the run.
    StationNoLongerPartOfTheRun = 8,

    /// Platform has changed.
    PlatformChange = 9,

    /// Exit line has changed.
    ExitLineChange = 10,

    /// Schedule has changed.
    ScheduleChange = 11,
}

impl Display for ChangeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReachedFinalDestination => write!(f, "ReachedFinalDestination"),
            Self::ArrivedAtStation => write!(f, "ArrivedAtStation"),
            Self::ExitedFirstStation => write!(f, "ExitedFirstStation"),
            Self::ExitedStation => write!(f, "ExitedStation"),
            Self::PassedStationWithoutStopping => write!(f, "PassedStationWithoutStopping"),
            Self::DetainedInPark => write!(f, "DetainedInPark"),
            Self::DetainedAtStation => write!(f, "DetainedAtStation"),
            Self::StationNoLongerPartOfTheRun => write!(f, "StationNoLongerPartOfTheRun"),
            Self::PlatformChange => write!(f, "PlatformChange"),
            Self::ExitLineChange => write!(f, "ExitLineChange"),
            Self::ScheduleChange => write!(f, "ScheduleChange"),
        }
    }
}

impl ChangeType {
    #[must_use]
    pub const fn is_relevant(&self) -> bool {
        matches!(
            self,
            Self::ReachedFinalDestination
                | Self::ArrivedAtStation
                | Self::ExitedFirstStation
                | Self::ExitedStation
                | Self::PassedStationWithoutStopping
                | Self::ScheduleChange
        )
    }

    #[must_use]
    pub const fn is_arrival(&self) -> bool {
        matches!(self, Self::ArrivedAtStation | Self::ReachedFinalDestination)
    }
}

/// Type of train.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrainType {
    /// Metro train.
    #[default]
    Metro,

    /// Ex Metro train.
    Exmetro,

    /// Freight train.
    Freight,
}

/// Direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum Direction {
    /// Right.
    Right = 0,

    /// Left.
    Left = 1,

    /// Unspecified.
    Unspecified = -1,
}

/// Direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum StopType {
    /// Original, Passing (non-stop/skip), or Destination (no dwell time in
    /// timetable).
    Original = 4,

    /// Intermediate stop (there is a dwell time in the time table).
    Intermediate = 5,
}

#[cfg(test)]
mod tests {
    use super::R9kMessage;

    #[test]
    fn deserialization() {
        let xml = include_str!("../data/sample.xml");
        let message: R9kMessage = quick_xml::de::from_str(xml).expect("should deserialize");

        let update = message.train_update;
        assert_eq!(update.even_train_id, Some("1234".to_string()));
        assert!(!update.changes.is_empty(), "should have changes");
    }
}
