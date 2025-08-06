use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Moduł cache z integracją Sentry
// pub mod lru_cache;
// pub mod decision_cache;

// pub use lru_cache::LruCache;
// pub use decision_cache::DecisionCache;

/// Wpis w cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// Wartość
    pub value: T,
    
    /// Czas utworzenia
    pub created_at: i64,
    
    /// Czas wygaśnięcia
    pub expires_at: i64,
    
    /// Liczba odczytów
    pub access_count: u64,
    
    /// Ostatni dostęp
    pub last_accessed: i64,
}

/// Statystyki cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Liczba trafień
    pub hits: u64,
    
    /// Liczba chybień
    pub misses: u64,
    
    /// Liczba wpisów
    pub entries: u64,
    
    /// Rozmiar cache (w bajtach)
    pub size_bytes: u64,
    
    /// Maksymalny rozmiar
    pub max_size: u64,
    
    /// Współczynnik trafień
    pub hit_rate: f64,
    
    /// Ostatnia aktualizacja
    pub last_updated: i64,
}

/// Trait dla cache
pub trait Cache<K, V>: Send + Sync {
    /// Pobiera wartość z cache
    fn get(&self, key: &K) -> Option<V>;
    
    /// Wstawia wartość do cache
    fn put(&mut self, key: K, value: V, ttl_seconds: u64);
    
    /// Usuwa wartość z cache
    fn remove(&mut self, key: &K) -> Option<V>;
    
    /// Czyści cache
    fn clear(&mut self);
    
    /// Zwraca rozmiar cache
    fn len(&self) -> usize;
    
    /// Sprawdza czy cache jest pusty
    fn is_empty(&self) -> bool;
    
    /// Zwraca statystyki
    fn stats(&self) -> CacheStats;
}

impl<T> CacheEntry<T> {
    /// Tworzy nowy wpis w cache
    pub fn new(value: T, ttl_seconds: u64) -> Self {
        let now = chrono::Utc::now().timestamp();
        
        Self {
            value,
            created_at: now,
            expires_at: now + ttl_seconds as i64,
            access_count: 0,
            last_accessed: now,
        }
    }
    
    /// Sprawdza czy wpis wygasł
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }
    
    /// Oznacza wpis jako odczytany
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = chrono::Utc::now().timestamp();
    }
    
    /// Zwraca wiek wpisu w sekundach
    pub fn age_seconds(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.created_at
    }
    
    /// Zwraca czas do wygaśnięcia w sekundach
    pub fn ttl_seconds(&self) -> i64 {
        self.expires_at - chrono::Utc::now().timestamp()
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            size_bytes: 0,
            max_size: 0,
            hit_rate: 0.0,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
}

impl CacheStats {
    /// Aktualizuje statystyki po trafieniu
    pub fn record_hit(&mut self) {
        self.hits += 1;
        self.update_hit_rate();
    }
    
    /// Aktualizuje statystyki po chybieniu
    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.update_hit_rate();
    }
    
    /// Aktualizuje współczynnik trafień
    fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hit_rate = self.hits as f64 / total as f64;
        }
        self.last_updated = chrono::Utc::now().timestamp();
    }
    
    /// Aktualizuje liczbę wpisów
    pub fn update_entries(&mut self, count: u64) {
        self.entries = count;
        self.last_updated = chrono::Utc::now().timestamp();
    }
    
    /// Aktualizuje rozmiar cache
    pub fn update_size(&mut self, size_bytes: u64) {
        self.size_bytes = size_bytes;
        self.last_updated = chrono::Utc::now().timestamp();
    }
}

/// Manager cache z monitorowaniem
pub struct CacheManager {
    /// Statystyki globalne
    stats: Arc<RwLock<CacheStats>>,
    
    /// Czy włączyć monitoring
    monitoring_enabled: bool,
}

impl CacheManager {
    /// Tworzy nowy manager cache
    pub fn new(monitoring_enabled: bool) -> Self {
        Self {
            stats: Arc::new(RwLock::new(CacheStats::default())),
            monitoring_enabled,
        }
    }
    
    /// Rejestruje trafienie w cache
    pub async fn record_hit(&self, cache_name: &str) {
        if self.monitoring_enabled {
            let mut stats = self.stats.write().await;
            stats.record_hit();
            
            // Metryka dla Sentry
            sentry::add_breadcrumb(sentry::Breadcrumb {
                message: Some(format!("Cache hit: {}", cache_name)),
                category: Some("cache".into()),
                level: sentry::Level::Debug,
                ..Default::default()
            });
        }
    }
    
    /// Rejestruje chybienie w cache
    pub async fn record_miss(&self, cache_name: &str) {
        if self.monitoring_enabled {
            let mut stats = self.stats.write().await;
            stats.record_miss();
            
            // Metryka dla Sentry
            sentry::add_breadcrumb(sentry::Breadcrumb {
                message: Some(format!("Cache miss: {}", cache_name)),
                category: Some("cache".into()),
                level: sentry::Level::Debug,
                ..Default::default()
            });
        }
    }
    
    /// Zwraca statystyki cache
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
    
    /// Sprawdza wydajność cache i wysyła alerty jeśli potrzeba
    pub async fn check_performance(&self) -> Result<()> {
        if !self.monitoring_enabled {
            return Ok(());
        }
        
        let stats = self.stats.read().await;
        
        // Alert jeśli współczynnik trafień jest niski
        if stats.hit_rate < 0.5 && (stats.hits + stats.misses) > 100 {
            sentry::capture_message(
                &format!("Low cache hit rate: {:.2}%", stats.hit_rate * 100.0),
                sentry::Level::Warning,
            );
        }
        
        // Alert jeśli cache jest przepełniony
        if stats.size_bytes > 0 && stats.max_size > 0 {
            let utilization = stats.size_bytes as f64 / stats.max_size as f64;
            if utilization > 0.9 {
                sentry::capture_message(
                    &format!("High cache utilization: {:.1}%", utilization * 100.0),
                    sentry::Level::Warning,
                );
            }
        }
        
        Ok(())
    }
}

/// Utility funkcje dla cache
pub mod utils {
    use super::*;
    
    /// Oblicza rozmiar obiektu w bajtach (przybliżony)
    pub fn estimate_size<T>(value: &T) -> usize {
        std::mem::size_of_val(value)
    }
    
    /// Generuje klucz cache z hashem
    pub fn generate_cache_key(prefix: &str, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{}:{:x}", prefix, hash)
    }
    
    /// Sprawdza czy klucz jest prawidłowy
    pub fn is_valid_key(key: &str) -> bool {
        !key.is_empty() && key.len() <= 250 && key.chars().all(|c| c.is_ascii())
    }
    
    /// Czyści wygasłe wpisy z mapy
    pub fn cleanup_expired<K, V>(map: &mut std::collections::HashMap<K, CacheEntry<V>>) 
    where 
        K: Clone + std::hash::Hash + Eq,
    {
        let expired_keys: Vec<K> = map
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            map.remove(&key);
        }
    }
}
