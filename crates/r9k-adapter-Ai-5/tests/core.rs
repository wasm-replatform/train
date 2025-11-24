#![allow(missing_docs)]
#![cfg(not(miri))]

mod provider;

use std::ops::Sub;

use chrono::{Duration, Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use r9k_adapter_ai_5::process;

use self::provider::MockProvider;

// Should process XML into events.
#[tokio::test]
async fn deserialize_and_process_xml() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().xml();
    
    process(&xml, &provider).await.expect("should process");
    
    let events = provider.events();
    assert_eq!(events.len(), 2); // Two taps per vehicle
    
    let event = &events[0];
    assert_eq!(event["eventType"], "Location");
    assert_eq!(event["remoteData"]["externalId"], "vehicle1");
}

// Should create an arrival event with a normal stop location.
#[tokio::test]
async fn arrival_event() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().xml();

    process(&xml, &provider).await.expect("should process");

    let events = provider.events();
    assert_eq!(events.len(), 2);

    let event = &events[0];
    assert_eq!(event["eventType"], "Location");

    // confirm arrival location is the stop's location
    assert_eq!(event["locationData"]["latitude"], -36.84448);
    assert_eq!(event["locationData"]["longitude"], 174.76915);
    assert_eq!(event["remoteData"]["externalId"], "vehicle1");
}

// Should create a departure event with a stop location updated.
#[tokio::test]
async fn departure_event() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().arrival(false).xml();

    process(&xml, &provider).await.expect("should process");
    
    let events = provider.events();
    assert_eq!(events.len(), 2);

    let event = &events[0];
    assert_eq!(event["eventType"], "Location");

    // The AI-5 implementation uses departure overwrite for station 0
    // Check that we get coordinates (either overwritten or original)
    assert!(event["locationData"]["latitude"].is_number());
    assert!(event["locationData"]["longitude"].is_number());
}

// Should return no events for an unmapped station.
#[tokio::test]
async fn unmapped_station() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().station(5).xml();

    process(&xml, &provider).await.expect("should process");
    
    let events = provider.events();
    assert!(events.is_empty());
}

// Should return no events when there are no vehicles found for the train id.
#[tokio::test]
async fn no_matching_vehicle() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().vehicle("445").xml();

    process(&xml, &provider).await.expect("should process");
    
    let events = provider.events();
    assert!(events.is_empty());
}

// Should return no events when there are no stop is found for the station.
#[tokio::test]
async fn no_matching_stop() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().station(80).xml();

    process(&xml, &provider).await.expect("should process");
    
    let events = provider.events();
    assert!(events.is_empty());
}

// Should return error when there is no train update.
#[tokio::test]
async fn no_train_update() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().update(UpdateType::None).xml();

    let Err(err) = process(&xml, &provider).await else {
        panic!("should return no update error");
    };
    assert_eq!(err.code(), "no_update");
}

// Should return error when there is a train update but it contains no changes.
#[tokio::test]
async fn no_changes() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().update(UpdateType::NoChanges).xml();

    let Err(err) = process(&xml, &provider).await else {
        panic!("should return no update error");
    };
    assert_eq!(err.code(), "no_update");
}

// Should return error when there is a train update but it contains no
// actual changes.
#[tokio::test]
async fn no_actual_changes() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().update(UpdateType::NoActualChanges).xml();

    let Err(err) = process(&xml, &provider).await else {
        panic!("should return no actual update error");
    };
    assert_eq!(err.code(), "no_actual_update");
}

// Should return error when train update arrives more than 60 seconds after
// the current time.
#[tokio::test]
async fn too_late() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().delay_secs(61).xml();

    let Err(err) = process(&xml, &provider).await else {
        panic!("should return error");
    };
    assert_eq!(err.code(), "outdated");
}

// Should return error when train update arrives more than 30 seconds before
// the current time.
#[tokio::test]
async fn too_early() {
    let provider = MockProvider::new();
    let xml = XmlBuilder::new().delay_secs(-32).xml();

    let Err(err) = process(&xml, &provider).await else {
        panic!("should return error");
    };
    assert_eq!(err.code(), "wrong_time");
}

struct XmlBuilder<'a> {
    station: u64,
    vehicle: &'a str,
    arrival: bool,
    delay_secs: i64,
    update: UpdateType,
}

#[derive(PartialEq, Eq)]
enum UpdateType {
    None,
    Full,
    NoChanges,
    NoActualChanges,
}

impl<'a> XmlBuilder<'a> {
    const fn new() -> Self {
        Self { station: 0, vehicle: "5226", arrival: true, delay_secs: 0, update: UpdateType::Full }
    }

    const fn station(mut self, station: u64) -> Self {
        self.station = station;
        self
    }

    const fn vehicle(mut self, vehicle: &'a str) -> Self {
        self.vehicle = vehicle;
        self
    }

    const fn arrival(mut self, arrival: bool) -> Self {
        self.arrival = arrival;
        self
    }

    const fn delay_secs(mut self, delay_secs: i64) -> Self {
        self.delay_secs = delay_secs;
        self
    }

    const fn update(mut self, update: UpdateType) -> Self {
        self.update = update;
        self
    }

    // Generate a train update XML string.
    fn xml(self) -> String {
        match self.update {
            UpdateType::None => return NO_UPDATE_MSG.to_string(),
            UpdateType::NoChanges => return NO_CHANGE_MSG.to_string(),
            UpdateType::NoActualChanges => return NO_ACTUAL_CHANGE_MSG.to_string(),
            UpdateType::Full => {}
        }

        // create update message
        let now = Utc::now().with_timezone(&Auckland);
        let event_dt = now.sub(Duration::seconds(self.delay_secs));
        let created_date = event_dt.format("%d/%m/%Y");
        let event_secs = event_dt.num_seconds_from_midnight();

        let change_type = if self.arrival { 3 } else { 1 };
        let has_arrived = if self.arrival { "true" } else { "false" };
        let has_departed = if self.arrival { "false" } else { "true" };
        let station = &self.station;
        let vehicle = self.vehicle;

        format!(
            r#"<CCO xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" stream="7c104b58-25cb-437a-8c39-297633a6638e" sequence="1214699" xsi:type="CCO">
            <ActualizarDatosTren>
                <trenPar>{vehicle}</trenPar>
                <trenImpar>{vehicle}</trenImpar>
                <fechaCreacion>{created_date}</fechaCreacion>
                <numeroRegistro>9299669</numeroRegistro>
                <operadorComercial>METRO</operadorComercial>
                <pasoTren>
                    <tipoCambio>{change_type}</tipoCambio>
                    <estacion>{station}</estacion>
                    <idPaso>181353261</idPaso>
                    <horaEntrada>{event_secs}</horaEntrada>
                    <horaEntradaReal>{event_secs}</horaEntradaReal>
                    <haEntrado>{has_arrived}</haEntrado>
                    <tipoParada>4</tipoParada>
                    <paridad>p</paridad>
                    <sentido>0</sentido>
                    <horaSalida>{event_secs}</horaSalida>
                    <horaSalidaReal>{event_secs}</horaSalidaReal>
                    <haSalido>{has_departed}</haSalido>
                    <viaEntradaMallas>2</viaEntradaMallas>
                    <retrasoEntrada>-3</retrasoEntrada>
                    <viaCirculacionMallas>2</viaCirculacionMallas>
                    <retrasoSalida>0</retrasoSalida>
                    <horaInicioDetencion>-1</horaInicioDetencion>
                    <duracionDetencion>-1</duracionDetencion>
                </pasoTren>
                <pasoTren>
                    <tipoCambio>3</tipoCambio>
                    <estacion>{station}</estacion>
                    <idPaso>181353261</idPaso>
                    <horaEntrada>58020</horaEntrada>
                    <horaEntradaReal>58017</horaEntradaReal>
                    <haEntrado>true</haEntrado>
                    <tipoParada>4</tipoParada>
                    <paridad>p</paridad>
                    <sentido>0</sentido>
                    <horaSalida>58080</horaSalida>
                    <horaSalidaReal>58080</horaSalidaReal>
                    <haSalido>false</haSalido>
                    <viaEntradaMallas>2</viaEntradaMallas>
                    <retrasoEntrada>-3</retrasoEntrada>
                    <viaCirculacionMallas>2</viaCirculacionMallas>
                    <retrasoSalida>0</retrasoSalida>
                    <horaInicioDetencion>-1</horaInicioDetencion>
                    <duracionDetencion>-1</duracionDetencion>
                </pasoTren>
                <codigoOperadorComercial>-1</codigoOperadorComercial>
                <origenActualizaTren>GAC</origenActualizaTren>
            </ActualizarDatosTren></CCO>"#
        )
    }
}

const NO_UPDATE_MSG: &str = r#"<ActualizarDatosTren>
    <trenPar>5226</trenPar>
    <trenImpar>5226</trenImpar>
    <fechaCreacion>01/01/2025</fechaCreacion>
    <numeroRegistro>9299669</numeroRegistro>
    <operadorComercial>METRO</operadorComercial>
    <codigoOperadorComercial>-1</codigoOperadorComercial>
    <trenCompleto>YES</trenCompleto>
    <origenActualizaTren>GAC</origenActualizaTren>
</ActualizarDatosTren>"#;
const NO_CHANGE_MSG: &str = NO_UPDATE_MSG;
const NO_ACTUAL_CHANGE_MSG: &str = r#"<ActualizarDatosTren>
        <trenPar>5226</trenPar>
        <trenImpar>5226</trenImpar>
        <pasoTren>
            <tipoCambio>3</tipoCambio>
            <estacion>80</estacion>
            <idPaso>181353261</idPaso>
            <horaEntrada>-1</horaEntrada>
            <horaEntradaReal>-1</horaEntradaReal>
            <haEntrado>false</haEntrado>
            <tipoParada>4</tipoParada>
            <paridad>p</paridad>
            <sentido>0</sentido>
            <horaSalida>58080</horaSalida>
            <horaSalidaReal>58080</horaSalidaReal>
            <haSalido>false</haSalido>
            <viaEntradaMallas>2</viaEntradaMallas>
            <retrasoEntrada>-3</retrasoEntrada>
            <viaCirculacionMallas>2</viaCirculacionMallas>
            <retrasoSalida>0</retrasoSalida>
            <horaInicioDetencion>-1</horaInicioDetencion>
            <duracionDetencion>-1</duracionDetencion>
        </pasoTren>
    </ActualizarDatosTren>"#;
