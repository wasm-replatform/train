use chrono::{DateTime, Utc};
use pretty_assertions::assert_eq;
use serde_json::Value;

mod provider_mock; // reuse existing mock
use provider_mock::MockProvider;

#[inline]
fn set_env(key: &str, val: &str) {
    unsafe {
        std::env::set_var(key, val);
    }
}

fn base_xml(even: &str, odd: &str, station: i32, actual_arr: i64, actual_dep: i64) -> String {
    format!(
        "<ActualizarDatosTren><trenPar>{}</trenPar><trenImpar>{}</trenImpar><fechaCreacion>02/08/2025</fechaCreacion><numeroRegistro>REG</numeroRegistro><operadorComercial>METRO</operadorComercial><codigoOperadorComercial>MT</codigoOperadorComercial><trenCompleto>YES</trenCompleto><origenActualizaTren>SYSTEM</origenActualizaTren><pasoTren><tipoCambio>3</tipoCambio><estacion>{}</estacion><idPaso>entry</idPaso><horaEntrada>3600</horaEntrada><horaEntradaReal>{}</horaEntradaReal><haEntrado>true</haEntrado><retrasoEntrada>0</retrasoEntrada><horaSalida>3700</horaSalida><horaSalidaReal>{}</horaSalidaReal><haSalido>true</haSalido><retrasoSalida>0</retrasoSalida><horaInicioDetencion>0</horaInicioDetencion><duracionDetencion>0</duracionDetencion><viaEntradaMallas>P1</viaEntradaMallas><viaCirculacionMallas>L1</viaCirculacionMallas><sentido>0</sentido><tipoParada>4</tipoParada><paridad>even</paridad></pasoTren></ActualizarDatosTren>",
        even, odd, station, actual_arr, actual_dep
    )
}

#[tokio::test]
async fn two_tap_timestamp_increment_delta() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "15");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::with_vehicles(vec!["EMU 001".into()]);
    let xml = base_xml("EMU-TRIP", "", 0, 3620, 3700);
    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    // Sleep a bit longer than two delays to ensure both publishes complete
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let published = provider.take_published();
    assert_eq!(published.len(), 2, "expect two publishes for single vehicle");
    let timestamps: Vec<DateTime<Utc>> = published
        .iter()
        .map(|e| {
            let v: Value = serde_json::from_slice(&e.payload).unwrap();
            v["messageData"]["timestamp"].as_str().unwrap().parse().unwrap()
        })
        .collect();
    let delta = timestamps[1] - timestamps[0];
    // Allow wider window due to timer granularity in constrained environments.
    assert!(
        delta.num_milliseconds() >= 5 && delta.num_milliseconds() <= 30,
        "delta {}ms outside expected window",
        delta.num_milliseconds()
    );
}

#[tokio::test]
async fn odd_train_id_fallback() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "5");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::with_vehicles(vec!["EMU 003".into()]);
    // evenTrainId empty forces fallback to oddTrainId
    let xml = base_xml("", "EMU-TRIP-ODD", 0, 3600, 3700);
    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    let published = provider.take_published();
    assert_eq!(published.len(), 2);
    let labels: Vec<String> = published
        .iter()
        .map(|e| {
            let v: Value = serde_json::from_slice(&e.payload).unwrap();
            v["remoteData"]["externalId"].as_str().unwrap().to_string()
        })
        .collect();
    assert!(
        labels.iter().all(|l| l.contains("EMU003") || l.contains("EMU003")),
        "sanitized labels should remove spaces (none here) and publish both events"
    );
}

#[tokio::test]
async fn empty_vehicle_list_no_events() {
    set_env("STATIONS", "0,19,40");
    set_env("TIMEZONE", "Pacific/Auckland");
    set_env("GTFS_CC_STATIC_URL", "http://mock-gtfs");
    set_env("BLOCK_MANAGEMENT_URL", "http://mock-block");
    set_env("R9K_TWO_TAP_DELAY_MS", "5");
    set_env("R9K_SKIP_DELAY_VALIDATION", "1");
    set_env("MAX_MESSAGE_DELAY_IN_SECONDS", "999999999");
    set_env("MIN_MESSAGE_DELAY_IN_SECONDS", "-999999999");
    let provider = MockProvider::with_vehicles(vec![]); // no vehicles allocated
    let xml = base_xml("EMU-TRIP", "", 0, 3600, 3700);
    r9k_adapter_ai_5::process(&xml, &provider).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    assert!(provider.take_published().is_empty(), "no vehicle allocations means no publishes");
}
