//! Compare manual dilax-adapter outputs with AI outputs and write a Markdown report.
#![cfg(not(miri))]

use anyhow::Result;
use std::path::Path;

#[allow(clippy::redundant_closure_for_method_calls)]
fn read_json(path: &std::path::Path) -> Result<serde_json::Value> {
    let bytes = std::fs::read(path)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)?;
    Ok(v)
}

#[allow(clippy::redundant_closure_for_method_calls)]
fn field_string(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn compare_fields(manual: &serde_json::Value, ai: &serde_json::Value) -> Vec<String> {
    let keys = ["trip_id", "stop_id", "start_date", "start_time"];
    let mut diffs = Vec::new();
    #[allow(clippy::explicit_iter_loop)]
    for k in keys.iter() {
        let m = field_string(manual, k);
        let a = field_string(ai, k);
        if m.as_deref() != a.as_deref() {
            #[allow(clippy::uninlined_format_args)]
            diffs.push(format!("- {}: manual={:?} ai={:?}", k, m, a));
        }
    }
    diffs
}

#[tokio::test]
async fn generate_comparison_report() -> Result<()> {
    let data_dir = Path::new("data");
    assert!(data_dir.is_dir(), "data directory missing");

    let mut rows: Vec<String> = vec![
        String::from("# Dilax Adapter vs AI Output Comparison\n"),
        String::from("\n"),
        String::from("- Scope: Compare enrichment fields (trip_id, stop_id, start_date, start_time)."),
        String::from("- Source: Manual outputs in `data/<stem>-out.json` vs AI outputs in `data/ai-<stem>-out.json`."),
        String::from("\n"),
    ];

    for entry in std::fs::read_dir(data_dir)? {
        let path = entry?.path();
        #[allow(clippy::manual_let_else)]
        let file_name = match path.file_name().and_then(|n| n.to_str()) { Some(n) => n, None => continue };
        if !file_name.ends_with("-out.json") || file_name.starts_with("ai-") { continue; }
        let stem_full = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let stem = stem_full.strip_suffix("-out").unwrap_or(stem_full);

        // manual output path
        let manual_path = path.clone();
        // AI output path
        #[allow(clippy::uninlined_format_args)]
        let ai_name = format!("ai-{}-out.json", stem);
        let ai_path = data_dir.join(ai_name);

        if !ai_path.exists() {
            #[allow(clippy::uninlined_format_args)]
            rows.push(format!("## {}\n- Status: AI output missing ({})\n", stem, ai_path.display()));
            continue;
        }

        let manual_json = read_json(&manual_path)?;
        let ai_json = read_json(&ai_path)?;
        let diffs = compare_fields(&manual_json, &ai_json);

        #[allow(clippy::uninlined_format_args)]
        rows.push(format!("## {}", stem));
        #[allow(clippy::uninlined_format_args)]
        rows.push(format!("- Manual: `{}`", manual_path.display()));
        #[allow(clippy::uninlined_format_args)]
        rows.push(format!("- AI: `{}`", ai_path.display()));
        if diffs.is_empty() {
            rows.push(String::from("- Result: ✅ Match on all enrichment fields"));
        } else {
            rows.push(String::from("- Result: ❌ Differences found"));
            rows.extend(diffs);
        }
        rows.push(String::from("\n"));
    }

    let report = rows.join("\n");
    let report_path = Path::new("DIFF-REPORT.md");
    std::fs::write(report_path, report)?;
    println!("Comparison report written to {}", report_path.display());

    Ok(())
}
