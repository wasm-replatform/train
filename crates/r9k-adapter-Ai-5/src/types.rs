use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum MovementType {
    Arrival,
    Departure,
    Prediction,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ChangeType {
    ExitedFirstStation = 1,
    ReachedFinalDestination = 2,
    ArrivedAtStation = 3,
    ExitedStation = 4,
    PassedStationWithoutStopping = 5,
    DetainedInPark = 6,
    DetainedAtStation = 7,
    StationNoLongerPartOfTheRun = 8,
    PlatformChange = 9,
    ExitLineChange = 10,
    ScheduleChange = 11,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Change {
    #[serde(rename = "tipoCambio")]
    pub change_type: i32,
    #[serde(rename = "estacion")]
    pub station: i32,
    #[serde(rename = "idPaso")]
    pub entry_id: Option<String>,
    #[serde(rename = "horaEntrada")]
    pub arrival_time: i64,
    #[serde(rename = "horaEntradaReal")]
    pub actual_arrival_time: i64,
    #[serde(rename = "haEntrado")]
    pub has_arrived: bool,
    #[serde(rename = "retrasoEntrada")]
    pub arrival_delay: i64,
    #[serde(rename = "horaSalida")]
    pub departure_time: i64,
    #[serde(rename = "horaSalidaReal")]
    pub actual_departure_time: i64,
    #[serde(rename = "haSalido")]
    pub has_departed: bool,
    #[serde(rename = "retrasoSalida")]
    pub departure_delay: i64,
    #[serde(rename = "horaInicioDetencion")]
    pub detention_time: i64,
    #[serde(rename = "duracionDetencion")]
    pub detention_duration: i64,
    #[serde(rename = "viaEntradaMallas")]
    pub platform: Option<String>,
    #[serde(rename = "viaCirculacionMallas")]
    pub exit_line: Option<String>,
    #[serde(rename = "sentido")]
    pub train_direction: i32,
    #[serde(rename = "tipoParada")]
    pub stop_type: i32,
    #[serde(rename = "paridad")]
    pub parity: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrainUpdate {
    #[serde(rename = "trenPar")]
    pub even_train_id: Option<String>,
    #[serde(rename = "trenImpar")]
    pub odd_train_id: Option<String>,
    #[serde(rename = "fechaCreacion")]
    pub created_date: Option<String>,
    #[serde(rename = "numeroRegistro")]
    pub registration_number: Option<String>,
    #[serde(rename = "operadorComercial")]
    pub train_type: Option<String>,
    #[serde(rename = "codigoOperadorComercial")]
    pub train_type_code: Option<String>,
    #[serde(rename = "trenCompleto")]
    pub full_train: Option<String>,
    #[serde(rename = "origenActualizaTren")]
    pub train_update_source: Option<String>,
    #[serde(rename = "pasoTren")]
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct R9kMessage {
    #[serde(rename = "ActualizarDatosTren")]
    pub train_update: Option<TrainUpdate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmarTrakEvent {
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "receivedAt")]
    pub received_at: String,
    #[serde(rename = "messageData")]
    pub message_data: MessageData,
    #[serde(rename = "eventData")]
    pub event_data: EventData,
    #[serde(rename = "remoteData")]
    pub remote_data: RemoteData,
    #[serde(rename = "locationData")]
    pub location_data: LocationData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageData {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventData;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteData {
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(rename = "remoteName", skip_serializing_if = "Option::is_none")]
    pub remote_name: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocationData {
    pub gps_accuracy: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub speed: i32,
}

impl SmarTrakEvent {
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub fn new(
        label: &str, lat: f64, lon: f64, received_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            event_type: "Location".to_string(),
            received_at: received_at.to_rfc3339(),
            message_data: MessageData { timestamp: now.to_rfc3339() },
            event_data: EventData {},
            remote_data: RemoteData {
                external_id: Some(label.replace(' ', "")),
                remote_name: None,
            },
            location_data: LocationData {
                gps_accuracy: 0,
                latitude: lat,
                longitude: lon,
                speed: 0,
            },
        }
    }
}
