use std::fmt;
use std::str::FromStr;

use jiff::civil::Date;
use jiff::{Error, Timestamp, Zoned};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const TIME_ZONE: &str = "Pacific/Auckland";

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct R9kDate(Date);

impl R9kDate {
    #[must_use]
    pub const fn new(date: Date) -> Self {
        Self(date)
    }

    #[must_use]
    pub const fn inner(&self) -> &Date {
        &self.0
    }

    #[must_use]
    pub const fn date(year: i16, month: i8, day: i8) -> Self {
        Self(jiff::civil::date(year, month, day))
    }

    /// Inverse of `to_timestamp_secs`. Takes the given `timestamp`, transforms it to New Zealand
    /// time and splits it into the date and the seconds since midnight parts.
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a valid unix epoch seconds timestamp.
    #[must_use]
    pub fn from_timestamp_secs(timestamp: i64) -> (Self, i64) {
        let timestamp = Timestamp::from_second(timestamp).unwrap();
        let zoned = timestamp.in_tz(TIME_ZONE).unwrap();
        let date = zoned.date();
        let time = zoned.time();
        let hours: i64 = time.hour().into();
        let minutes: i64 = time.minute().into();
        let seconds: i64 = time.second().into();
        let seconds_since_midnight = (hours * 60 + minutes) * 60 + seconds;
        (Self(date), seconds_since_midnight)
    }

    /// Assume the time is midnight of the date in New Zealand time. Add `seconds_since_midnight`
    /// and return the unix epoch seconds of that.
    #[must_use]
    pub fn to_timestamp_secs(&self, seconds_since_midnight: i64) -> i64 {
        self.to_zoned().timestamp().as_second() + seconds_since_midnight
    }

    /// The resulting `Zoned` represents midnight of the date in New Zealand time.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn to_zoned(&self) -> Zoned {
        self.0.in_tz(TIME_ZONE).unwrap()
    }
}

impl FromStr for R9kDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let date = Date::strptime("%d/%m/%Y", s)?;
        Ok(Self(date))
    }
}

impl fmt::Display for R9kDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime("%d/%m/%Y"))
    }
}

impl Serialize for R9kDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{self}"))
    }
}

impl<'de> Deserialize<'de> for R9kDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let date = R9kDate::date(2025, 9, 19);
        let serialized = serde_json::to_string(&date).unwrap();
        assert_eq!(serialized, "\"19/09/2025\"");
    }

    #[test]
    fn test_deserialize() {
        let json = "\"19/09/2025\"";
        let date: R9kDate = serde_json::from_str(json).unwrap();
        assert_eq!(date, R9kDate::date(2025, 9, 19));
    }

    #[test]
    fn test_from_str() {
        let date = R9kDate::from_str("19/09/2025").unwrap();
        assert_eq!(date, R9kDate::date(2025, 9, 19));
    }

    #[test]
    fn test_display() {
        let date = R9kDate::date(2025, 9, 19);
        assert_eq!(date.to_string(), "19/09/2025");
    }

    #[test]
    fn test_to_timestamp() {
        let date = R9kDate::date(2025, 10, 7);
        assert_eq!(date.to_timestamp_secs(1), 1_759_748_401);
    }

    #[test]
    fn test_from_timestamp() {
        let timestamp = 1_759_748_401;
        let (date, seconds_since_midnight) = R9kDate::from_timestamp_secs(timestamp);
        assert_eq!(date, R9kDate::date(2025, 10, 7));
        assert_eq!(seconds_since_midnight, 1);
    }

    #[test]
    fn test_timestamp_round_trip() {
        let date = R9kDate::date(2025, 10, 7);
        let seconds = 3661; // 1 hour, 1 minute, 1 second
        let timestamp = date.to_timestamp_secs(seconds);
        let (recovered_date, recovered_seconds) = R9kDate::from_timestamp_secs(timestamp);
        assert_eq!(date, recovered_date);
        assert_eq!(seconds, recovered_seconds);
    }
}
