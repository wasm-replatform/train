use rand::{Rng, SeedableRng, rngs::StdRng};

/// Deterministic pseudo-random train update XML generator for edge-case fuzzing.
/// Usage: call `generate_xml(seed)` inside a test to obtain a stable XML payload.
pub fn generate_xml(seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(seed);
    let station_ids = [0, 19, 40, 5, 133, 134, 9218];
    let station = station_ids[rng.gen_range(0..station_ids.len())];
    let change_type = [3, 4, 5, 1, 11][rng.gen_range(0..5)];
    let arrival = rng.gen_range(0..7200);
    let actual_arrival = if rng.gen_bool(0.9) { arrival + rng.gen_range(0..120) } else { -1 };
    let departure = arrival + rng.gen_range(0..600);
    let actual_departure = if rng.gen_bool(0.9) { departure + rng.gen_range(0..120) } else { -1 };
    let has_arrived = actual_arrival >= 0;
    let has_departed = actual_departure >= 0;
    let date = format!("{:02}/{:02}/2025", rng.gen_range(1..28), rng.gen_range(1..12));
    format!(
        r#"<ActualizarDatosTren><trenPar>EMU-TRIP</trenPar><trenImpar></trenImpar><fechaCreacion>{}</fechaCreacion><numeroRegistro>REG</numeroRegistro><operadorComercial>METRO</operadorComercial><codigoOperadorComercial>MT</codigoOperadorComercial><trenCompleto>YES</trenCompleto><origenActualizaTren>SYSTEM</origenActualizaTren><pasoTren><tipoCambio>{}</tipoCambio><estacion>{}</estacion><idPaso>entry</idPaso><horaEntrada>{}</horaEntrada><horaEntradaReal>{}</horaEntradaReal><haEntrado>{}</haEntrado><retrasoEntrada>0</retrasoEntrada><horaSalida>{}</horaSalida><horaSalidaReal>{}</horaSalidaReal><haSalido>{}</haSalido><retrasoSalida>0</retrasoSalida><horaInicioDetencion>0</horaInicioDetencion><duracionDetencion>0</duracionDetencion><viaEntradaMallas>P1</viaEntradaMallas><viaCirculacionMallas>L1</viaCirculacionMallas><sentido>0</sentido><tipoParada>4</tipoParada><paridad>even</paridad></pasoTren></ActualizarDatosTren>"#,
        date,
        change_type,
        station,
        arrival,
        actual_arrival,
        has_arrived,
        departure,
        actual_departure,
        has_departed
    )
}

#[cfg(test)]
mod tests {
    use super::generate_xml;
    #[test]
    fn deterministic_generation() {
        let a = generate_xml(42);
        let b = generate_xml(42);
        assert_eq!(a, b, "same seed yields identical XML");
    }
}
