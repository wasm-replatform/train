use std::process::{Command, Stdio};
use std::time::Duration;
use serde_json::json;
use pretty_assertions::assert_eq;
use tokio::time;

#[inline]
fn set_env(key: &str, val: &str) { unsafe { std::env::set_var(key, val); } }

mod provider_mock;
use provider_mock::MockProvider;
// use r9k_adapter_ai_5::process;

fn sample_xml(change_type: i32, station: i32, created_date: &str, arrival: i64, actual_arrival: i64, departed: bool, actual_departure: i64) -> String {
    format!(r#"<ActualizarDatosTren><trenPar>EMU-TRIP</trenPar><trenImpar></trenImpar><fechaCreacion>{}</fechaCreacion><numeroRegistro>REG</numeroRegistro><operadorComercial>METRO</operadorComercial><codigoOperadorComercial>MT</codigoOperadorComercial><trenCompleto>YES</trenCompleto><origenActualizaTren>SYSTEM</origenActualizaTren><pasoTren><tipoCambio>{}</tipoCambio><estacion>{}</estacion><idPaso>entry</idPaso><horaEntrada>{}</horaEntrada><horaEntradaReal>{}</horaEntradaReal><haEntrado>true</haEntrado><retrasoEntrada>0</retrasoEntrada><horaSalida>{}</horaSalida><horaSalidaReal>{}</horaSalidaReal><haSalido>{}</haSalido><retrasoSalida>0</retrasoSalida><horaInicioDetencion>0</horaInicioDetencion><duracionDetencion>0</duracionDetencion><viaEntradaMallas>P1</viaEntradaMallas><viaCirculacionMallas>L1</viaCirculacionMallas><sentido>0</sentido><tipoParada>4</tipoParada><paridad>even</paridad></pasoTren></ActualizarDatosTren>"#, created_date, change_type, station, arrival, actual_arrival, actual_departure, actual_departure, departed)
}

#[tokio::test]
#[ignore = "Requires Node.js runtime for legacy comparison"]
async fn parity_success_two_tap() {
    // Environment config (filter stations includes station 0)
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");

    let provider = MockProvider::new();
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION","1");
    let xml = sample_xml(3, 0, "02/08/2025", 3600, 3620, true, 3700);

    let proc = tokio::spawn({ let p = provider.clone(); async move { r9k_adapter_ai_5::process(&xml, &p).await.unwrap(); }});
    // Wait slightly longer than two short test delays
    time::sleep(Duration::from_millis(30)).await;
    proc.await.unwrap();

    let published = provider.take_published();
    assert_eq!(published.len(), 4, "expected two vehicles x two publications");

    // Normalize Rust events
    let rust_events: Vec<_> = published.iter().map(|e| {
        let v: serde_json::Value = serde_json::from_slice(&e.payload).unwrap();
        json!({
          "eventType": v["eventType"],
          "receivedAt": v["receivedAt"],
          "externalId": v["remoteData"]["externalId"],
          "latitude": v["locationData"]["latitude"],
          "longitude": v["locationData"]["longitude"],
          "timestamp": v["messageData"]["timestamp"],
        })
    }).collect();

    // Prepare legacy runner input replicating same train update semantics
    let legacy_input = json!({
        "trainUpdate": {
            "evenTrainId": "EMU-TRIP",
            "oddTrainId": "",
            "createdDate": "02/08/2025",
            "numeroRegistro": "REG",
            "operadorComercial": "METRO",
            "codigoOperadorComercial": "MT",
            "trenCompleto": "YES",
            "origenActualizaTren": "SYSTEM",
            "changes": [{
              "changeType": 3,
              "station": 0,
              "entryId": "entry",
              "arrivalTime": 3600,
              "actualArrivalTime": 3620,
              "hasArrived": true,
              "arrivalDelay": 0,
              "departureTime": 3700,
              "actualDepartureTime": 3700,
              "hasDeparted": true,
              "departureDelay": 0,
              "detentionTime": 0,
              "detentionDuration": 0,
              "platform": "P1",
              "exitLine": "L1",
              "trainDirection": 0,
              "stopType": 4,
              "parity": "even"
            }]
        },
        "vehicles": ["EMU 001","EMU 002"],
        "stops": [
          {"stop_code":"133","stop_lat":-36.84448,"stop_lon":174.76915},
          {"stop_code":"134","stop_lat":-37.20299,"stop_lon":174.90990},
          {"stop_code":"9218","stop_lat":-36.99412,"stop_lon":174.8770}
        ]
    });

    // Run legacy runner
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let legacy_path = format!("{manifest_dir}/tests/legacy_runner.js");
    let mut child = Command::new("node")
        .arg(&legacy_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn legacy runner");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(legacy_input.to_string().as_bytes()).unwrap();
    }
    let output = child.wait_with_output().expect("wait legacy");
    assert!(output.status.success(), "legacy runner failed: {:?}", output);
    let legacy_events: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(legacy_events.len(), rust_events.len(), "event count parity");

    // Field parity (allow timestamp drift <= 3s)
    for (r, l) in rust_events.iter().zip(legacy_events.iter()) {
        assert_eq!(r["eventType"], l["eventType"], "eventType parity");
        assert_eq!(r["externalId"], l["externalId"], "externalId parity");
        assert_eq!(r["latitude"], l["latitude"], "latitude parity");
        assert_eq!(r["longitude"], l["longitude"], "longitude parity");
    }
}

#[tokio::test]
async fn error_no_update() {
        set_env("R9K_SKIP_DELAY_VALIDATION","1");
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::new();
    let xml = "<ActualizarDatosTren><trenPar>EMU-TRIP</trenPar><pasoTren></pasoTren></ActualizarDatosTren>";
    let err = r9k_adapter_ai_5::process(xml, &provider).await.unwrap_err();
    assert_eq!(err.code(), "invalid_format");
}

#[tokio::test]
async fn error_no_actual_update() {
    set_env("R9K_SKIP_DELAY_VALIDATION","1"); // Skip time validation but NOT actual time check
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::new();
    let xml = sample_xml(3, 0, "02/08/2025", 3600, -1, true, -1); // actual times -1
    let err = r9k_adapter_ai_5::process(&xml, &provider).await.unwrap_err();
    assert_eq!(err.code(), "no_actual_update");
}

#[tokio::test]
async fn unmapped_station_filtered() {
        set_env("R9K_SKIP_DELAY_VALIDATION","1");
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::new();
    let xml = sample_xml(3, 5, "02/08/2025", 3600, 3620, true, 3700); // station 5 unmapped
    let _ = r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    assert!(provider.take_published().is_empty(), "no events for unmapped station");
}

#[tokio::test]
async fn outdated_error() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "60");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-30");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_SKIP_DELAY_VALIDATION","0");
    // Deterministic time: fix 'now' to 2025-08-02 so past date (2000) produces large positive delay (> 60s)
    set_env("R9K_FIXED_NOW_TIMESTAMP","2025-08-02T00:00:00Z");
    let provider = MockProvider::new();
    // Use far past date (year 2000) - delay will be ~25 years = way more than 60s max
    let xml = sample_xml(3, 0, "01/01/2000", 10, 10, true, 10);
    let err = r9k_adapter_ai_5::process(&xml, &provider).await.unwrap_err();
    assert_eq!(err.code(), "outdated");
}
