use pretty_assertions::assert_eq;

mod provider_mock;
use provider_mock::MockProvider;

#[inline]
fn set_env(key: &str, val: &str) {
    unsafe {
        std::env::set_var(key, val);
    }
}

// Build a minimal XML payload allowing custom movement fields.
fn xml_change(
    change_type: i32, station: i32, created_date: &str, actual_arrival: i64, actual_departure: i64,
    has_arrived: bool, has_departed: bool,
) -> String {
    format!(
        r#"<ActualizarDatosTren><trenPar>EMU-TRIP</trenPar><trenImpar></trenImpar><fechaCreacion>{}</fechaCreacion><numeroRegistro>REG</numeroRegistro><operadorComercial>METRO</operadorComercial><codigoOperadorComercial>MT</codigoOperadorComercial><trenCompleto>YES</trenCompleto><origenActualizaTren>SYSTEM</origenActualizaTren><pasoTren><tipoCambio>{}</tipoCambio><estacion>{}</estacion><idPaso>entry</idPaso><horaEntrada>{}</horaEntrada><horaEntradaReal>{}</horaEntradaReal><haEntrado>{}</haEntrado><retrasoEntrada>0</retrasoEntrada><horaSalida>{}</horaSalida><horaSalidaReal>{}</horaSalidaReal><haSalido>{}</haSalido><retrasoSalida>0</retrasoSalida><horaInicioDetencion>0</horaInicioDetencion><duracionDetencion>0</duracionDetencion><viaEntradaMallas>P1</viaEntradaMallas><viaCirculacionMallas>L1</viaCirculacionMallas><sentido>0</sentido><tipoParada>4</tipoParada><paridad>even</paridad></pasoTren></ActualizarDatosTren>"#,
        created_date,
        change_type,
        station,
        // legacy semantics: horaEntrada/horaSalida seconds from midnight (approx)
        actual_arrival,
        actual_arrival,
        if has_arrived { "true" } else { "false" },
        actual_departure,
        actual_departure,
        if has_departed { "true" } else { "false" },
    )
}

// Departure scenario (changeType=1) two-tap publish parity with arrival test.
#[tokio::test]
async fn departure_two_tap() {
    set_common_env();
    set_env("R9K_TWO_TAP_DELAY_MS", "10");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");
    let provider = MockProvider::new();
    // changeType 1 -> departure; has_departed true, has_arrived false.
    let xml = xml_change(1, 0, "02/08/2025", 3600, 3700, false, true);
    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let published = provider.take_published();
    assert_eq!(published.len(), 4, "two vehicles x two taps for departure");
}

// Station excluded from filter list should yield no events.
#[tokio::test]
async fn filtered_out_station_no_events() {
    set_env("STATIONS", "0,19"); // exclude station 40
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "5");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");
    let provider = MockProvider::new();
    // Station 40 maps to stop_code 134 but not in filter list.
    let xml = xml_change(3, 40, "02/08/2025", 3600, 3700, true, true);
    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    assert!(provider.take_published().is_empty(), "no events for filtered-out station");
}

// Envelope without trainUpdate should produce invalid_format error (legacy: no publish).
#[tokio::test]
async fn envelope_without_train_update() {
    set_common_env();
    let provider = MockProvider::new();
    let xml = "<CCO xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" stream=\"abc\" sequence=\"1\" xsi:type=\"CCO\"></CCO>";
    let err = r9k_adapter_ai_5::process(xml, &provider).await.unwrap_err();
    assert_eq!(err.code(), "invalid_format");
}

// Early (too negative) delay should produce wrong_time (deterministic via fixed now).
#[tokio::test]
async fn early_time_error() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "60");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-30");
    // Ensure validation active
    set_env("R9K_SKIP_DELAY_VALIDATION", "0");
    // Fix 'now' to a stable timestamp earlier than created date 2099
    set_env("R9K_FIXED_NOW_TIMESTAMP", "2025-08-02T00:00:00Z");
    let provider = MockProvider::new();
    // Far future date (2099) with now=2025 means event_time is way in future, so delay will be large negative (< -30s)
    let xml = xml_change(3, 0, "02/08/2099", 10, 10, true, true);
    let err = r9k_adapter_ai_5::process(&xml, &provider).await.unwrap_err();
    assert_eq!(err.code(), "wrong_time");
}

fn set_common_env() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
}
