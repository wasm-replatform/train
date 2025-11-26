//! Dummy data enrichment test for Dilax adapter.
#![cfg(not(miri))]

mod provider;

use anyhow::Result;
use dilax_adapter::{DilaxMessage, EnrichedEvent};
use provider::MockProvider;

/// For every Dilax input json in `data/` writes enriched output to `<name>-out.json`.
#[tokio::test]
async fn generate_enriched_outputs() -> Result<()> {
    let data_dir = std::path::Path::new("data");
    assert!(data_dir.is_dir(), "data directory missing");

    for entry in std::fs::read_dir(data_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json")
            && !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("-out.json"))
            && !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains("datacontract"))
        {
            let bytes = std::fs::read(&path)?;
            let message: DilaxMessage = match serde_json::from_slice(&bytes) {
                Ok(m) => m,
                Err(e) => {
                    println!("skipping non-Dilax schema file {path:?}: {e}");
                    continue;
                }
            };            

            let provider = MockProvider::new();
            dilax_adapter::handlers::processor::process(message, &provider).await?;
            let events: Vec<EnrichedEvent> = provider.events();
            assert_eq!(events.len(), 1, "expected one enriched publish for {path:?}");
            let enriched = &events[0];

            assert!(enriched.trip_id.is_some(), "trip id should be set for {path:?}");
            assert!(enriched.stop_id.is_some(), "stop id should be set for {path:?}");
            assert!(enriched.start_date.is_some(), "start date should be set for {path:?}");
            assert!(enriched.start_time.is_some(), "start time should be set for {path:?}");

            // compare with existing expected output file
            let stem = path.file_stem().and_then(|s| s.to_str()).expect("file stem");
            let out_name = format!("{stem}-out.json");
            let out_path = data_dir.join(out_name);
            let expected = std::fs::read(&out_path).expect("expected output file exists");
            let expected_json: serde_json::Value = serde_json::from_slice(&expected)?;
            let actual_json: serde_json::Value = serde_json::to_value(enriched)?;
            assert_eq!(expected_json, actual_json, "mismatch for {out_path:?}");
        }
    }

    Ok(())
}
