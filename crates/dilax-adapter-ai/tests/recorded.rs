//! Dummy data enrichment test for Dilax adapter.
#![cfg(not(miri))]

mod provider;

use anyhow::{Context, Result};
use dilax_adapter_ai::{Config, DilaxMessage, ProviderWrapper};
use dilax_adapter_ai::logic::process_event;
use provider::MockProvider;

/// For every Dilax input json in `data/` writes enriched output to `<name>-out.json`.
#[tokio::test]
#[allow(clippy::redundant_closure_for_method_calls)]
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
            let config = Config::from_env().context("loading config in test")?;
            let wrapper = ProviderWrapper::new(&provider, &config);
            let maybe = process_event(&wrapper, message.into_event()).await?;
            if maybe.is_none() {
                continue;
            }
            let enriched_event = maybe.unwrap();

            assert!(enriched_event.trip_id.is_some(), "trip id should be set for {path:?}");
            assert!(enriched_event.stop_id.is_some(), "stop id should be set for {path:?}");
            assert!(enriched_event.start_date.is_some(), "start date should be set for {path:?}");
            assert!(enriched_event.start_time.is_some(), "start time should be set for {path:?}");

            // Write AI enriched output to prefixed file: ai-<stem>-out.json
            let stem = path.file_stem().and_then(|s| s.to_str()).expect("file stem");
            let out_name = format!("ai-{stem}-out.json");
            let out_path = data_dir.join(out_name);
            let payload = serde_json::to_vec_pretty(&enriched_event)?;
            std::fs::write(&out_path, payload)?;
        }
    }

    Ok(())
}
