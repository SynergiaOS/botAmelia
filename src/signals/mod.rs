use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Moduł do przetwarzania sygnałów tradingowych z integracją Sentry
// pub mod processor;
// pub mod validator;
// pub mod sources;

// pub use processor::SignalProcessor;
// pub use validator::SignalValidator;

/// Poziom pewności sygnału
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Confidence {
    Low,      // 5x base leverage
    Medium,   // 10x base leverage  
    High,     // 20x base leverage
    Extreme,  // 30x base leverage
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::Low => write!(f, "low"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::High => write!(f, "high"),
            Confidence::Extreme => write!(f, "extreme"),
        }
    }
}

impl std::str::FromStr for Confidence {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Confidence::Low),
            "medium" => Ok(Confidence::Medium),
            "high" => Ok(Confidence::High),
            "extreme" => Ok(Confidence::Extreme),
            _ => Err(anyhow::anyhow!("Invalid confidence level: {}", s)),
        }
    }
}

/// Struktura sygnału tradingowego
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Identyfikator sygnału
    pub id: String,
    
    /// Token/symbol
    pub token: String,
    
    /// Źródło sygnału
    pub source: String,
    
    /// Poziom pewności
    pub confidence: Confidence,
    
    /// Cena w momencie sygnału
    pub price: f64,
    
    /// Wolumen
    pub volume: f64,
    
    /// Timestamp (Unix timestamp)
    pub timestamp: i64,
    
    /// Dodatkowe metadane
    pub metadata: serde_json::Value,
    
    /// Hash sygnału dla cache
    pub hash: Option<String>,
}

impl Signal {
    /// Tworzy nowy sygnał
    pub fn new(
        token: String,
        source: String,
        confidence: Confidence,
        price: f64,
        volume: f64,
        metadata: serde_json::Value,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        
        let mut signal = Self {
            id,
            token,
            source,
            confidence,
            price,
            volume,
            timestamp,
            metadata,
            hash: None,
        };
        
        signal.hash = Some(signal.calculate_hash());
        signal
    }
    
    /// Oblicza hash sygnału dla cache
    pub fn calculate_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.token.hash(&mut hasher);
        self.source.hash(&mut hasher);
        self.confidence.to_string().hash(&mut hasher);
        ((self.price * 1000.0) as u64).hash(&mut hasher); // Zaokrąglenie do 3 miejsc po przecinku
        ((self.volume * 1000.0) as u64).hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
    
    /// Waliduje sygnał
    pub fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!("Token cannot be empty"));
        }
        
        if self.source.is_empty() {
            return Err(anyhow::anyhow!("Source cannot be empty"));
        }
        
        if self.price <= 0.0 {
            return Err(anyhow::anyhow!("Price must be greater than 0"));
        }
        
        if self.volume < 0.0 {
            return Err(anyhow::anyhow!("Volume cannot be negative"));
        }
        
        if self.timestamp <= 0 {
            return Err(anyhow::anyhow!("Timestamp must be valid"));
        }
        
        // Sprawdzenie czy sygnał nie jest zbyt stary (maksymalnie 1 godzina)
        let now = chrono::Utc::now().timestamp();
        if now - self.timestamp > 3600 {
            return Err(anyhow::anyhow!("Signal is too old"));
        }
        
        Ok(())
    }
    
    /// Zwraca wiek sygnału w sekundach
    pub fn age_seconds(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.timestamp
    }
    
    /// Sprawdza czy sygnał jest świeży (młodszy niż podana liczba sekund)
    pub fn is_fresh(&self, max_age_seconds: i64) -> bool {
        self.age_seconds() <= max_age_seconds
    }

    /// Tworzy sygnał z określonym timestampem (dla testów)
    pub fn new_with_timestamp(
        token: String,
        source: String,
        confidence: Confidence,
        price: f64,
        volume: f64,
        metadata: serde_json::Value,
        timestamp: i64,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();

        let mut signal = Self {
            id,
            token,
            source,
            confidence,
            price,
            volume,
            timestamp,
            metadata,
            hash: None,
        };

        signal.hash = Some(signal.calculate_hash());
        signal
    }
}

/// Trait dla źródeł sygnałów
#[async_trait::async_trait]
pub trait SignalSource: Send + Sync {
    /// Pobiera sygnały ze źródła
    async fn get_signals(&self) -> Result<Vec<Signal>>;
    
    /// Zwraca nazwę źródła
    fn source_name(&self) -> &str;
    
    /// Sprawdza czy źródło jest połączone
    fn is_connected(&self) -> bool;
    
    /// Inicjalizuje połączenie ze źródłem
    async fn connect(&mut self) -> Result<()>;
    
    /// Zamyka połączenie ze źródłem
    async fn disconnect(&mut self) -> Result<()>;
}

/// Statystyki przetwarzania sygnałów
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalStats {
    /// Całkowita liczba przetworzonych sygnałów
    pub total_processed: u64,
    
    /// Liczba prawidłowych sygnałów
    pub valid_signals: u64,
    
    /// Liczba nieprawidłowych sygnałów
    pub invalid_signals: u64,
    
    /// Liczba sygnałów z cache
    pub cache_hits: u64,
    
    /// Liczba sygnałów bez cache
    pub cache_misses: u64,
    
    /// Średni czas przetwarzania (w ms)
    pub avg_processing_time: f64,
    
    /// Statystyki według źródeł
    pub by_source: HashMap<String, SourceStats>,
    
    /// Statystyki według poziomów pewności
    pub by_confidence: HashMap<String, u64>,
}

/// Statystyki dla konkretnego źródła
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStats {
    /// Liczba sygnałów z tego źródła
    pub count: u64,
    
    /// Liczba błędów
    pub errors: u64,
    
    /// Ostatni czas aktywności
    pub last_activity: i64,
    
    /// Czy źródło jest aktywne
    pub is_active: bool,
}

impl Default for SignalStats {
    fn default() -> Self {
        Self {
            total_processed: 0,
            valid_signals: 0,
            invalid_signals: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_processing_time: 0.0,
            by_source: HashMap::new(),
            by_confidence: HashMap::new(),
        }
    }
}

impl SignalStats {
    /// Aktualizuje statystyki po przetworzeniu sygnału
    pub fn update_for_signal(&mut self, signal: &Signal, processing_time_ms: f64, from_cache: bool) {
        self.total_processed += 1;
        self.valid_signals += 1;
        
        if from_cache {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
        
        // Aktualizacja średniego czasu przetwarzania
        self.avg_processing_time = (self.avg_processing_time * (self.total_processed - 1) as f64 + processing_time_ms) / self.total_processed as f64;
        
        // Aktualizacja statystyk według źródła
        let source_stats = self.by_source.entry(signal.source.clone()).or_insert_with(|| SourceStats {
            count: 0,
            errors: 0,
            last_activity: signal.timestamp,
            is_active: true,
        });
        source_stats.count += 1;
        source_stats.last_activity = signal.timestamp;
        
        // Aktualizacja statystyk według poziomu pewności
        *self.by_confidence.entry(signal.confidence.to_string()).or_insert(0) += 1;
    }
    
    /// Aktualizuje statystyki po błędzie
    pub fn update_for_error(&mut self, source: Option<&str>) {
        self.total_processed += 1;
        self.invalid_signals += 1;
        
        if let Some(source_name) = source {
            let source_stats = self.by_source.entry(source_name.to_string()).or_insert_with(|| SourceStats {
                count: 0,
                errors: 0,
                last_activity: chrono::Utc::now().timestamp(),
                is_active: false,
            });
            source_stats.errors += 1;
        }
    }
    
    /// Zwraca współczynnik sukcesu
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            return 0.0;
        }
        self.valid_signals as f64 / self.total_processed as f64
    }
    
    /// Zwraca współczynnik trafień cache
    pub fn cache_hit_rate(&self) -> f64 {
        let total_cache_operations = self.cache_hits + self.cache_misses;
        if total_cache_operations == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total_cache_operations as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::new(
            "BONK".to_string(),
            "pump_fun".to_string(),
            Confidence::High,
            0.000001,
            1000000.0,
            json!({"liquidity": 50000}),
        );

        assert_eq!(signal.token, "BONK");
        assert_eq!(signal.source, "pump_fun");
        assert_eq!(signal.confidence, Confidence::High);
        assert_eq!(signal.price, 0.000001);
        assert_eq!(signal.volume, 1000000.0);
        assert!(signal.hash.is_some());
        assert!(!signal.id.is_empty());
    }

    #[test]
    fn test_signal_validation_valid() {
        let signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        assert!(signal.validate().is_ok());
    }

    #[test]
    fn test_signal_validation_empty_token() {
        let signal = Signal::new(
            "".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        assert!(signal.validate().is_err());
    }

    #[test]
    fn test_signal_validation_empty_source() {
        let signal = Signal::new(
            "TEST".to_string(),
            "".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        assert!(signal.validate().is_err());
    }

    #[test]
    fn test_signal_validation_negative_price() {
        let mut signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );
        signal.price = -0.001;

        assert!(signal.validate().is_err());
    }

    #[test]
    fn test_signal_validation_negative_volume() {
        let mut signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );
        signal.volume = -1000.0;

        assert!(signal.validate().is_err());
    }

    #[test]
    fn test_signal_validation_old_timestamp() {
        let old_timestamp = chrono::Utc::now().timestamp() - 7200; // 2 hours ago
        let signal = Signal::new_with_timestamp(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
            old_timestamp,
        );

        assert!(signal.validate().is_err());
    }

    #[test]
    fn test_signal_freshness() {
        let signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        assert!(signal.is_fresh(300)); // Should be fresh (< 5 minutes)
        assert!(signal.age_seconds() < 5); // Should be very recent
    }

    #[test]
    fn test_signal_hash_consistency() {
        let signal1 = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        let signal2 = Signal::new(
            "DIFFERENT".to_string(), // Different token
            "test_source".to_string(),
            Confidence::Medium,
            0.001,
            1000.0,
            json!({}),
        );

        // Different signals should have different hashes
        assert_ne!(signal1.hash, signal2.hash);

        // Same signal should have consistent hash
        let hash1 = signal1.calculate_hash();
        let hash2 = signal1.calculate_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_confidence_display() {
        assert_eq!(Confidence::Low.to_string(), "low");
        assert_eq!(Confidence::Medium.to_string(), "medium");
        assert_eq!(Confidence::High.to_string(), "high");
        assert_eq!(Confidence::Extreme.to_string(), "extreme");
    }

    #[test]
    fn test_confidence_from_str() {
        assert_eq!("low".parse::<Confidence>().unwrap(), Confidence::Low);
        assert_eq!("medium".parse::<Confidence>().unwrap(), Confidence::Medium);
        assert_eq!("high".parse::<Confidence>().unwrap(), Confidence::High);
        assert_eq!("extreme".parse::<Confidence>().unwrap(), Confidence::Extreme);
        assert_eq!("HIGH".parse::<Confidence>().unwrap(), Confidence::High); // Case insensitive

        assert!("invalid".parse::<Confidence>().is_err());
    }

    #[test]
    fn test_signal_stats_default() {
        let stats = SignalStats::default();
        assert_eq!(stats.total_processed, 0);
        assert_eq!(stats.valid_signals, 0);
        assert_eq!(stats.invalid_signals, 0);
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_signal_stats_update() {
        let mut stats = SignalStats::default();
        let signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::High,
            0.001,
            1000.0,
            json!({}),
        );

        stats.update_for_signal(&signal, 5.0, false);

        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.valid_signals, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.avg_processing_time, 5.0);
        assert_eq!(stats.success_rate(), 1.0);
        assert!(stats.by_source.contains_key("test_source"));
        assert!(stats.by_confidence.contains_key("high"));
    }

    #[test]
    fn test_signal_stats_error_handling() {
        let mut stats = SignalStats::default();

        stats.update_for_error(Some("error_source"));

        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.invalid_signals, 1);
        assert_eq!(stats.success_rate(), 0.0);
        assert!(stats.by_source.contains_key("error_source"));
        assert_eq!(stats.by_source["error_source"].errors, 1);
    }

    #[test]
    fn test_signal_stats_cache_hit_rate() {
        let mut stats = SignalStats::default();
        let signal = Signal::new(
            "TEST".to_string(),
            "test_source".to_string(),
            Confidence::High,
            0.001,
            1000.0,
            json!({}),
        );

        // Add cache hit
        stats.update_for_signal(&signal, 1.0, true);
        assert_eq!(stats.cache_hit_rate(), 1.0);

        // Add cache miss
        stats.update_for_signal(&signal, 2.0, false);
        assert_eq!(stats.cache_hit_rate(), 0.5);
    }
}
