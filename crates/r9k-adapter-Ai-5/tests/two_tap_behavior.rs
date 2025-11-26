use pretty_assertions::assert_eq;
use serde_json::Value;

mod provider_mock;
use provider_mock::MockProvider;

#[inline]
fn set_env(key: &str, val: &str) {
    unsafe {
        std::env::set_var(key, val);
    }
}

fn sample_xml(
    station: i32, actual_arrival: i64, actual_departure: i64, has_departed: bool,
) -> String {
    format!(
        r#"<ActualizarDatosTren><trenPar>EMU-TRIP</trenPar><trenImpar></trenImpar><fechaCreacion>02/08/2025</fechaCreacion><numeroRegistro>REG</numeroRegistro><operadorComercial>METRO</operadorComercial><codigoOperadorComercial>MT</codigoOperadorComercial><trenCompleto>YES</trenCompleto><origenActualizaTren>SYSTEM</origenActualizaTren><pasoTren><tipoCambio>3</tipoCambio><estacion>{}</estacion><idPaso>entry</idPaso><horaEntrada>3600</horaEntrada><horaEntradaReal>{}</horaEntradaReal><haEntrado>true</haEntrado><retrasoEntrada>0</retrasoEntrada><horaSalida>3700</horaSalida><horaSalidaReal>{}</horaSalidaReal><haSalido>{}</haSalido><retrasoSalida>0</retrasoSalida><horaInicioDetencion>0</horaInicioDetencion><duracionDetencion>0</duracionDetencion><viaEntradaMallas>P1</viaEntradaMallas><viaCirculacionMallas>L1</viaCirculacionMallas><sentido>0</sentido><tipoParada>4</tipoParada><paridad>even</paridad></pasoTren></ActualizarDatosTren>"#,
        station,
        actual_arrival,
        actual_departure,
        if has_departed { "true" } else { "false" }
    )
}

#[tokio::test]
async fn two_tap_publishes_twice_per_vehicle() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");

    let provider = MockProvider::new(); // Has 2 vehicles by default
    let xml = sample_xml(0, 3620, 3700, true);

    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let published = provider.take_published();
    assert_eq!(published.len(), 4, "Expected 2 vehicles × 2 taps = 4 events");
}

#[tokio::test]
async fn two_tap_events_have_correct_structure() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");

    let provider = MockProvider::with_vehicles(vec!["EMU 001".into()]);
    let xml = sample_xml(0, 3620, 3700, true);

    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let published = provider.take_published();
    assert_eq!(published.len(), 2, "Expected 2 events for single vehicle");

    // Verify both events have correct structure
    for event_msg in &published {
        let event: Value = serde_json::from_slice(&event_msg.payload).unwrap();

        assert_eq!(event["eventType"], "Location", "Event type must be Location");
        assert_eq!(event["remoteData"]["externalId"], "EMU001", "Vehicle label sanitized");
        assert_eq!(event["locationData"]["gps_accuracy"], 0, "GPS accuracy is 0");
        assert_eq!(event["locationData"]["speed"], 0, "Speed is 0");
        assert_eq!(event["locationData"]["latitude"], -36.84448, "Latitude from stop_code 133");
        assert_eq!(event["locationData"]["longitude"], 174.76915, "Longitude from stop_code 133");
        assert!(event["receivedAt"].is_string(), "receivedAt must be timestamp");
        assert!(event["messageData"]["timestamp"].is_string(), "timestamp must be present");
    }
}

#[tokio::test]
async fn two_tap_timestamps_increment() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "15");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");

    let provider = MockProvider::with_vehicles(vec!["EMU 001".into()]);
    let xml = sample_xml(0, 3620, 3700, true);

    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;

    let published = provider.take_published();
    assert_eq!(published.len(), 2);

    let ts1: Value = serde_json::from_slice(&published[0].payload).unwrap();
    let ts2: Value = serde_json::from_slice(&published[1].payload).unwrap();

    let time1 = ts1["messageData"]["timestamp"].as_str().unwrap();
    let time2 = ts2["messageData"]["timestamp"].as_str().unwrap();

    let dt1: chrono::DateTime<chrono::Utc> = time1.parse().unwrap();
    let dt2: chrono::DateTime<chrono::Utc> = time2.parse().unwrap();

    assert!(dt2 > dt1, "Second event timestamp must be after first");
    let delta = (dt2 - dt1).num_milliseconds();
    assert!(
        delta >= 10 && delta <= 40,
        "Timestamp delta {}ms should be within reasonable window",
        delta
    );
}

#[tokio::test]
async fn two_tap_both_events_for_each_vehicle() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");

    let provider = MockProvider::with_vehicles(vec!["EMU 001".into(), "EMU 002".into()]);
    let xml = sample_xml(0, 3620, 3700, true);

    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let published = provider.take_published();
    assert_eq!(published.len(), 4, "2 vehicles × 2 taps = 4 events");

    // Count events per vehicle
    let mut emu001_count = 0;
    let mut emu002_count = 0;

    for event_msg in &published {
        let event: Value = serde_json::from_slice(&event_msg.payload).unwrap();
        match event["remoteData"]["externalId"].as_str().unwrap() {
            "EMU001" => emu001_count += 1,
            "EMU002" => emu002_count += 1,
            id => panic!("Unexpected vehicle ID: {}", id),
        }
    }

    assert_eq!(emu001_count, 2, "EMU001 should have 2 events");
    assert_eq!(emu002_count, 2, "EMU002 should have 2 events");
}

#[tokio::test]
async fn two_tap_arrival_vs_departure_same_location() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");

    let provider = MockProvider::with_vehicles(vec!["EMU 001".into()]);

    // Test arrival (hasDeparted=false)
    let arrival_xml = sample_xml(0, 3620, 3700, false);
    r9k_adapter_ai_5::process(&arrival_xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let arrival_events = provider.take_published();
    assert_eq!(arrival_events.len(), 2);

    let arrival_event: Value = serde_json::from_slice(&arrival_events[0].payload).unwrap();
    let arrival_lat = arrival_event["locationData"]["latitude"].as_f64().unwrap();
    let arrival_lon = arrival_event["locationData"]["longitude"].as_f64().unwrap();

    // Arrival should use GTFS stop location (not overwrite)
    assert_eq!(arrival_lat, -36.84448, "Arrival uses GTFS stop lat");
    assert_eq!(arrival_lon, 174.76915, "Arrival uses GTFS stop lon");

    // Test departure (hasDeparted=true) - should use same location for station 0
    let departure_xml = sample_xml(0, 3620, 3700, true);
    r9k_adapter_ai_5::process(&departure_xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let departure_events = provider.take_published();
    assert_eq!(departure_events.len(), 2);

    let departure_event: Value = serde_json::from_slice(&departure_events[0].payload).unwrap();
    let departure_lat = departure_event["locationData"]["latitude"].as_f64().unwrap();
    let departure_lon = departure_event["locationData"]["longitude"].as_f64().unwrap();

    // Departure for station 0 (stop_code 133) uses departure overwrite which happens to be same coords
    assert_eq!(departure_lat, -36.84448, "Departure uses overwrite lat");
    assert_eq!(departure_lon, 174.76915, "Departure uses overwrite lon");
}
