// #![cfg(not(miri))]

// use std::path::Path;
// use std::{cmp::Ordering, fs};

// use anyhow::{Result, bail};
// use credibil_api::Client;
// use r9k_position::StopInfo;
// use r9k_position::{ChangeType, Error, EventType, R9kMessage};
// use serde::Deserialize;

// /// One recording session of the Typescript adapter.
// #[derive(Deserialize, Clone)]
// #[allow(dead_code)]
// struct Wiretap {
//     input: String,
//     /// Unix epoch in milliseconds
//     now: Option<i64>,
//     /// Seconds since midnight
//     event_seconds: Option<i32>,
//     /// Unix epoch in seconds
//     event_date: Option<i32>,
//     message_delay: Option<i32>,
//     stop_info: Option<StopInfo>,
//     allocated_vehicles: Option<Vec<String>>,
//     error: Option<Error>,
//     not_relevant_type: Option<bool>,
//     not_relevant_station: Option<bool>,
//     publishing: Option<Vec<Publish>>,
// }

// /// An instance of a value published by Typescript.
// #[derive(Deserialize, Clone, Debug)]
// struct Publish {
//     key: String,
//     /// JSON encoded value.
//     value: String,
// }

// impl Source for Wiretap {
//     async fn fetch(&self, _owner: &str, key: &Key) -> Result<SourceData> {
//         match key {
//             Key::StopInfo(stop_code) => match &self.stop_info {
//                 None => bail!("no stop info in wiretap"),
//                 Some(stop_info) => {
//                     if stop_info.stop_code == *stop_code {
//                         Ok(SourceData::StopInfo(stop_info.clone()))
//                     } else {
//                         bail!("stop info in wiretap does not match requested stop code")
//                     }
//                 }
//             },
//             Key::BlockMgt(_train_id) => match &self.allocated_vehicles {
//                 None => bail!("no allocated vehicles in wiretap"),
//                 Some(allocated) => Ok(SourceData::BlockMgt(allocated.clone())),
//             },
//         }
//     }
// }

// impl Time for Wiretap {
//     #[allow(clippy::option_if_let_else)]
//     fn now(&self) -> Zoned {
//         match self.now {
//             Some(s) => Zoned::new(
//                 Timestamp::from_millisecond(s).unwrap(),
//                 TimeZone::get("Pacific/Auckland").unwrap(),
//             ),
//             None => panic!("Wiretap data is missing now field"),
//         }
//     }
// }

// impl Provider for Wiretap {}

// /// This test runs through a folder of files that recorded the input and output
// /// of the Typescript adapter.
// #[tokio::test]
// async fn wiretap() -> Result<()> {
//     let wiretap_dir = Path::new("data/wiretap");

//     assert!(wiretap_dir.exists(), "Wiretap data directory should exist");

//     for entry in fs::read_dir(wiretap_dir).expect("Should be able to read wiretap directory") {
//         let entry = entry.expect("Should be able to read directory entry");
//         let path = entry.path();
//         println!("Wiretap file: {path:?}");

//         let reader = fs::File::open(&path).expect("Should be able to open file");

//         // Parse the YAML content
//         match serde_yaml::from_reader::<_, Wiretap>(&reader) {
//             Ok(w) => run_wiretap(w).await?,
//             Err(e) => panic!("Failed to parse YAML in file {path:?}: {e}"),
//         }
//     }
//     Ok(())
// }

// async fn run_wiretap(wiretap: Wiretap) -> Result<()> {
//     let api = Client::new(wiretap.clone());
//     let request = R9kMessage::try_from(wiretap.input)?;
//     let response = match api.request(request).owner("owner").await {
//         Ok(r) => r,
//         Err(e) => {
//             assert_eq!(e, wiretap.error.unwrap());
//             return Ok(());
//         }
//     };
//     let events = response.body.smartrak_events;

//     // Expect to not emit events of typescript skipped an irrelevant station.
//     if wiretap.not_relevant_station.unwrap_or_default() {
//         assert!(events.is_empty());
//     }
//     // Expect to not emit events of typescript skipped an irrelevant update type.
//     if wiretap.not_relevant_type.unwrap_or_default() {
//         assert!(events.is_empty());
//     }

//     if events.is_empty() {
//         assert!(wiretap.publishing.is_none_or(|p| p.is_empty()));
//         return;
//     }

//     // There must be publishing events.
//     let publishing = wiretap.publishing.unwrap();

//     // If we did emit events, they must correspond to the first wave of publishing after
//     // 5 secs.
//     assert_eq!(events.len() * 2, publishing.len(), "Expecting 2 TS publish events per event");

//     publishing.into_iter().zip(events).for_each(|(publish, mut actual)| {
//         let expected: SmarTrakEvent = serde_json::from_str(&publish.value).unwrap();

//         // Kafka key is derived from the event.
//         assert_eq!(publish.key, expected.remote_data.external_id);

//         // Add 5 seconds to the actual message timestamp because typescript sleeps 5 seconds
//         // before publishing the first round.
//         let diff = expected.message_data.timestamp - (actual.message_data.timestamp + 5.seconds());

//         // Add 5 seconds of tolerance because Typescript waits for Api responses before
//         // publishing.
//         assert!(
//             diff.compare(3.seconds()).unwrap() == Ordering::Less,
//             "expected - actual has to be < 3secs, got: {diff}"
//         );

//         // Overwrite the timestamp with the expected one so that we get a clean diff if
//         // above assert passes.
//         actual.message_data.timestamp = expected.message_data.timestamp;

//         assert_eq!(expected, actual);

//         // Finally compare generated json. Compare ``Value`s instead of strings because of key
//         // ordering.
//         let json_actual = serde_json::to_value(&actual).unwrap();
//         let json_expected: serde_json::Value = serde_json::from_str(&publish.value).unwrap();
//         assert_eq!(json_expected, json_actual);
//     });

//     Ok(())
// }
