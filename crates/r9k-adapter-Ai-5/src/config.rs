use std::collections::HashMap;
use std::env;

/// Returns list of station ids (as strings) that are relevant for filtering.
pub fn filter_stations() -> Vec<String> {
    env::var("STATIONS")
        .unwrap_or_else(|_| "0,19,40".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect()
}

/// Maximum message delay threshold in seconds (default 60).
pub fn max_message_delay() -> i64 {
    env::var("MAX_MESSAGE_DELAY_IN_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(60)
}

/// Minimum message delay threshold in seconds (default -30).
pub fn min_message_delay() -> i64 {
    env::var("MIN_MESSAGE_DELAY_IN_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(-30)
}

/// Timezone (IANA) used for interpreting train update dates.
pub fn timezone() -> String {
    env::var("TIMEZONE").unwrap_or_else(|_| "Pacific/Auckland".to_string())
}

/// Map of station id -> stop code.
pub fn station_id_to_stop_code_map() -> &'static HashMap<i32, &'static str> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<i32, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        HashMap::from([
            (0, "133"),
            (2, "115"),
            (3, "102"),
            (4, "605"),
            (5, "unmapped"),
            (6, "244"),
            (7, "122"),
            (8, "104"),
            (9, "105"),
            (10, "129"),
            (11, "125"),
            (12, "128"),
            (13, "127"),
            (15, "unmapped"),
            (16, "101"),
            (17, "109"),
            (18, "108"),
            (19, "9218"),
            (20, "unmapped"),
            (21, "107"),
            (22, "97"),
            (23, "112"),
            (24, "114"),
            (26, "118"),
            (27, "119"),
            (28, "120"),
            (29, "123"),
            (30, "124"),
            (31, "121"),
            (32, "106"),
            (33, "98"),
            (34, "99"),
            (35, "100"),
            (36, "130"),
            (37, "103"),
            (38, "unmapped"),
            (39, "113"),
            (40, "134"),
            (41, "277"),
            (115, "126"),
            (202, "116"),
            (371, "117"),
            (2000, "unmapped"),
            (2001, "606"),
            (2002, "140"),
            (2004, "unmapped"),
            (2005, "unmapped"),
        ])
    })
}

/// Departure location overwrite mapping stop_code -> (lat, lon).
pub fn departure_location_overwrite() -> &'static HashMap<i32, (f64, f64)> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<i32, (f64, f64)>> = OnceLock::new();
    MAP.get_or_init(|| {
        HashMap::from([
            (133, (-36.84448, 174.76915)),
            (134, (-37.20299, 174.90990)),
            (9218, (-36.99412, 174.8770)),
        ])
    })
}
