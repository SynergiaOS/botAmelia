use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Moduł bezpieczeństwa z integracją Sentry
// pub mod encryption;
// pub mod wallet;
// pub mod auth;

// pub use encryption::*;
// pub use wallet::WalletManager;
// pub use auth::*;

/// Poziom bezpieczeństwa
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Event bezpieczeństwa
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Identyfikator eventu
    pub id: String,
    
    /// Typ eventu
    pub event_type: SecurityEventType,
    
    /// Poziom bezpieczeństwa
    pub level: SecurityLevel,
    
    /// Opis eventu
    pub description: String,
    
    /// Adres IP (jeśli dotyczy)
    pub ip_address: Option<String>,
    
    /// User agent (jeśli dotyczy)
    pub user_agent: Option<String>,
    
    /// Dodatkowe metadane
    pub metadata: serde_json::Value,
    
    /// Czas wystąpienia
    pub timestamp: i64,
}

/// Typ eventu bezpieczeństwa
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    /// Nieautoryzowany dostęp
    UnauthorizedAccess,
    
    /// Podejrzana transakcja
    SuspiciousTransaction,
    
    /// Anomalia w tradingu
    TradingAnomaly,
    
    /// Próba włamania
    IntrusionAttempt,
    
    /// Wyciek danych
    DataLeak,
    
    /// Błąd kryptograficzny
    CryptographicError,
    
    /// Naruszenie integralności
    IntegrityViolation,
}

/// Monitor bezpieczeństwa
pub struct SecurityMonitor {
    /// Czy monitoring jest włączony
    enabled: bool,
    
    /// Historia eventów
    events: Vec<SecurityEvent>,
    
    /// Maksymalna liczba eventów w historii
    max_events: usize,
}

impl SecurityMonitor {
    /// Tworzy nowy monitor bezpieczeństwa
    pub fn new(enabled: bool, max_events: usize) -> Self {
        Self {
            enabled,
            events: Vec::new(),
            max_events,
        }
    }
    
    /// Rejestruje event bezpieczeństwa
    pub fn log_event(&mut self, event: SecurityEvent) {
        if !self.enabled {
            return;
        }
        
        // Logowanie do Sentry jeśli poziom jest odpowiednio wysoki
        if matches!(event.level, SecurityLevel::High | SecurityLevel::Critical) {
            self.send_to_sentry(&event);
        }
        
        // Dodanie do historii
        self.events.push(event.clone());
        
        // Ograniczenie rozmiaru historii
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
        
        tracing::warn!(
            "Security event: {:?} - {} (Level: {:?})",
            event.event_type,
            event.description,
            event.level
        );
    }
    
    /// Wysyła event do Sentry
    fn send_to_sentry(&self, event: &SecurityEvent) {
        sentry::configure_scope(|scope| {
            scope.set_tag("security_event", format!("{:?}", event.event_type));
            scope.set_tag("security_level", format!("{:?}", event.level));
            scope.set_tag("event_id", &event.id);
            
            if let Some(ref ip) = event.ip_address {
                scope.set_tag("ip_address", ip);
            }
            
            scope.set_extra("metadata", event.metadata.clone().into());
        });
        
        let sentry_level = match event.level {
            SecurityLevel::Critical => sentry::Level::Fatal,
            SecurityLevel::High => sentry::Level::Error,
            SecurityLevel::Medium => sentry::Level::Warning,
            SecurityLevel::Low => sentry::Level::Info,
        };
        
        sentry::capture_message(&event.description, sentry_level);
    }
    
    /// Tworzy event nieautoryzowanego dostępu
    pub fn unauthorized_access(&mut self, description: String, ip: Option<String>) {
        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::UnauthorizedAccess,
            level: SecurityLevel::High,
            description,
            ip_address: ip,
            user_agent: None,
            metadata: serde_json::Value::Null,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.log_event(event);
    }
    
    /// Tworzy event podejrzanej transakcji
    pub fn suspicious_transaction(&mut self, description: String, metadata: serde_json::Value) {
        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::SuspiciousTransaction,
            level: SecurityLevel::High,
            description,
            ip_address: None,
            user_agent: None,
            metadata,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.log_event(event);
    }
    
    /// Tworzy event anomalii w tradingu
    pub fn trading_anomaly(&mut self, description: String, metadata: serde_json::Value) {
        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::TradingAnomaly,
            level: SecurityLevel::Medium,
            description,
            ip_address: None,
            user_agent: None,
            metadata,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.log_event(event);
    }
    
    /// Zwraca ostatnie eventy
    pub fn recent_events(&self, count: usize) -> Vec<&SecurityEvent> {
        self.events.iter().rev().take(count).collect()
    }
    
    /// Zwraca eventy według typu
    pub fn events_by_type(&self, event_type: SecurityEventType) -> Vec<&SecurityEvent> {
        self.events.iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }
    
    /// Zwraca eventy według poziomu
    pub fn events_by_level(&self, level: SecurityLevel) -> Vec<&SecurityEvent> {
        self.events.iter()
            .filter(|e| e.level == level)
            .collect()
    }
    
    /// Sprawdza czy są eventy krytyczne w ostatnim czasie
    pub fn has_critical_events(&self, minutes: i64) -> bool {
        let cutoff = chrono::Utc::now().timestamp() - (minutes * 60);
        
        self.events.iter().any(|e| {
            e.level == SecurityLevel::Critical && e.timestamp > cutoff
        })
    }
}

/// Walidator bezpieczeństwa
pub struct SecurityValidator;

impl SecurityValidator {
    /// Waliduje adres IP
    pub fn validate_ip(ip: &str) -> bool {
        ip.parse::<std::net::IpAddr>().is_ok()
    }
    
    /// Sprawdza czy IP jest z dozwolonych zakresów
    pub fn is_ip_allowed(ip: &str, allowed_ranges: &[String]) -> bool {
        // Implementacja sprawdzania zakresów IP
        // Na razie prosta implementacja
        allowed_ranges.iter().any(|range| ip.starts_with(range))
    }
    
    /// Waliduje token autoryzacji
    pub fn validate_auth_token(token: &str) -> bool {
        // Podstawowa walidacja tokenu
        !token.is_empty() && token.len() >= 32 && token.chars().all(|c| c.is_ascii_alphanumeric())
    }
    
    /// Sprawdza siłę hasła
    pub fn check_password_strength(password: &str) -> PasswordStrength {
        let mut score = 0;
        
        if password.len() >= 8 {
            score += 1;
        }
        if password.len() >= 12 {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_lowercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_uppercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 1;
        }
        if password.chars().any(|c| !c.is_ascii_alphanumeric()) {
            score += 1;
        }
        
        match score {
            0..=2 => PasswordStrength::Weak,
            3..=4 => PasswordStrength::Medium,
            5..=6 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }
}

/// Siła hasła
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

/// Utility funkcje bezpieczeństwa
pub mod utils {
    use super::*;
    
    /// Generuje bezpieczny token
    pub fn generate_secure_token(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Hashuje dane z solą
    pub fn hash_with_salt(data: &str, salt: &str) -> Result<String> {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::SaltString;

        let argon2 = Argon2::default();
        let salt = SaltString::new(salt)
            .map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;
        let password_hash = argon2.hash_password(data.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Hash failed: {}", e))?;

        Ok(password_hash.to_string())
    }
    
    /// Weryfikuje hash
    pub fn verify_hash(data: &str, hash: &str) -> Result<bool> {
        use argon2::{Argon2, PasswordVerifier};
        use argon2::password_hash::PasswordHash;

        let argon2 = Argon2::default();
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Invalid hash: {}", e))?;

        Ok(argon2.verify_password(data.as_bytes(), &parsed_hash).is_ok())
    }
}
