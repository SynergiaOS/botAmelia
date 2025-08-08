#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(clippy::all)]

use anyhow::Result;
use cerberus::cache::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unit tests for cache functionality

#[test]
fn test_cache_entry_creation() {
    let entry = CacheEntry::new("test_value".to_string(), 300); // 5 minutes TTL

    assert_eq!(entry.value, "test_value");
    assert_eq!(entry.access_count, 0);
    assert!(!entry.is_expired());
    assert!(entry.ttl_seconds() > 0);
    assert!(entry.age_seconds() < 5);
}

#[test]
fn test_cache_entry_expiration() {
    let mut entry = CacheEntry::new("test_value".to_string(), 1); // 1 second TTL

    // Should not be expired immediately
    assert!(!entry.is_expired());

    // Simulate expiration by setting past timestamp
    entry.expires_at = chrono::Utc::now().timestamp() - 10;
    assert!(entry.is_expired());
}

#[test]
fn test_cache_entry_access_tracking() {
    let mut entry = CacheEntry::new("test_value".to_string(), 300);

    assert_eq!(entry.access_count, 0);

    entry.mark_accessed();
    assert_eq!(entry.access_count, 1);

    entry.mark_accessed();
    assert_eq!(entry.access_count, 2);
}

#[test]
fn test_cache_stats_default() {
    let stats = CacheStats::default();

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.entries, 0);
    assert_eq!(stats.size_bytes, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[test]
fn test_cache_stats_hit_tracking() {
    let mut stats = CacheStats::default();

    stats.record_hit();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.hit_rate, 1.0);

    stats.record_miss();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hit_rate, 0.5);

    stats.record_hit();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.hit_rate, 2.0 / 3.0);
}

#[test]
fn test_cache_stats_updates() {
    let mut stats = CacheStats::default();

    stats.update_entries(100);
    assert_eq!(stats.entries, 100);

    stats.update_size(1024);
    assert_eq!(stats.size_bytes, 1024);
}

#[tokio::test]
async fn test_cache_manager_creation() {
    let manager = CacheManager::new(true);
    let stats = manager.get_stats().await;

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_cache_manager_hit_recording() {
    let manager = CacheManager::new(true);

    manager.record_hit("test_cache").await;
    let stats = manager.get_stats().await;

    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.hit_rate, 1.0);
}

#[tokio::test]
async fn test_cache_manager_miss_recording() {
    let manager = CacheManager::new(true);

    manager.record_miss("test_cache").await;
    let stats = manager.get_stats().await;

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hit_rate, 0.0);
}

#[tokio::test]
async fn test_cache_manager_disabled_monitoring() {
    let manager = CacheManager::new(false);

    // Should not record anything when monitoring is disabled
    manager.record_hit("test_cache").await;
    manager.record_miss("test_cache").await;

    let stats = manager.get_stats().await;
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_cache_manager_performance_check() -> Result<()> {
    let manager = CacheManager::new(true);

    // Add some hits and misses
    for _ in 0..10 {
        manager.record_hit("test_cache").await;
    }
    for _ in 0..90 {
        manager.record_miss("test_cache").await;
    }

    // Performance check should not fail (just logs warnings)
    manager.check_performance().await?;

    Ok(())
}

#[test]
fn test_cache_utils_estimate_size() {
    let test_string = "Hello, World!";
    let size = utils::estimate_size(&test_string);

    // Size should be reasonable (at least the string length)
    assert!(size >= test_string.len());
}

#[test]
fn test_cache_utils_generate_key() {
    let key1 = utils::generate_cache_key("prefix", "data1");
    let key2 = utils::generate_cache_key("prefix", "data2");
    let key3 = utils::generate_cache_key("prefix", "data1"); // Same as key1

    assert_ne!(key1, key2); // Different data should produce different keys
    assert_eq!(key1, key3); // Same data should produce same key
    assert!(key1.starts_with("prefix:"));
}

#[test]
fn test_cache_utils_key_validation() {
    assert!(utils::is_valid_key("valid_key"));
    assert!(utils::is_valid_key("prefix:hash123"));
    assert!(utils::is_valid_key("a")); // Single character

    assert!(!utils::is_valid_key("")); // Empty
    assert!(!utils::is_valid_key("key with spaces")); // Non-ASCII spaces
    assert!(!utils::is_valid_key(&"x".repeat(300))); // Too long
}

#[test]
fn test_cache_utils_cleanup_expired() {
    let mut cache_map: HashMap<String, CacheEntry<String>> = HashMap::new();

    // Add fresh entry
    cache_map.insert(
        "fresh".to_string(),
        CacheEntry::new("value1".to_string(), 300),
    );

    // Add expired entry
    let mut expired_entry = CacheEntry::new("value2".to_string(), 300);
    expired_entry.expires_at = chrono::Utc::now().timestamp() - 10; // Expired
    cache_map.insert("expired".to_string(), expired_entry);

    assert_eq!(cache_map.len(), 2);

    utils::cleanup_expired(&mut cache_map);

    assert_eq!(cache_map.len(), 1);
    assert!(cache_map.contains_key("fresh"));
    assert!(!cache_map.contains_key("expired"));
}

// Mock implementation of Cache trait for testing
struct MockCache {
    data: HashMap<String, CacheEntry<String>>,
    stats: CacheStats,
}

impl MockCache {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            stats: CacheStats::default(),
        }
    }
}

impl Cache<String, String> for MockCache {
    fn get(&self, key: &String) -> Option<String> {
        if let Some(entry) = self.data.get(key) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }
        None
    }

    fn put(&mut self, key: String, value: String, ttl_seconds: u64) {
        let entry = CacheEntry::new(value, ttl_seconds);
        self.data.insert(key, entry);
        self.stats.update_entries(self.data.len() as u64);
    }

    fn remove(&mut self, key: &String) -> Option<String> {
        if let Some(entry) = self.data.remove(key) {
            self.stats.update_entries(self.data.len() as u64);
            Some(entry.value)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.stats.update_entries(0);
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn stats(&self) -> CacheStats {
        self.stats.clone()
    }
}

#[test]
fn test_mock_cache_basic_operations() {
    let mut cache = MockCache::new();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);

    // Test put and get
    cache.put("key1".to_string(), "value1".to_string(), 300);
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());

    let value = cache.get(&"key1".to_string());
    assert_eq!(value, Some("value1".to_string()));

    // Test get non-existent key
    let missing = cache.get(&"missing".to_string());
    assert_eq!(missing, None);
}

#[test]
fn test_mock_cache_remove() {
    let mut cache = MockCache::new();

    cache.put("key1".to_string(), "value1".to_string(), 300);
    cache.put("key2".to_string(), "value2".to_string(), 300);
    assert_eq!(cache.len(), 2);

    let removed = cache.remove(&"key1".to_string());
    assert_eq!(removed, Some("value1".to_string()));
    assert_eq!(cache.len(), 1);

    let missing = cache.remove(&"missing".to_string());
    assert_eq!(missing, None);
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_mock_cache_clear() {
    let mut cache = MockCache::new();

    cache.put("key1".to_string(), "value1".to_string(), 300);
    cache.put("key2".to_string(), "value2".to_string(), 300);
    assert_eq!(cache.len(), 2);

    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_mock_cache_expiration() {
    let mut cache = MockCache::new();

    // Add entry with very short TTL
    cache.put("short_lived".to_string(), "value".to_string(), 0);

    // Should return None for expired entry
    let value = cache.get(&"short_lived".to_string());
    assert_eq!(value, None);
}

#[test]
fn test_mock_cache_stats() {
    let mut cache = MockCache::new();
    let initial_stats = cache.stats();

    assert_eq!(initial_stats.entries, 0);

    cache.put("key1".to_string(), "value1".to_string(), 300);
    let updated_stats = cache.stats();

    assert_eq!(updated_stats.entries, 1);
}

#[test]
fn test_cache_entry_ttl_calculation() {
    let entry = CacheEntry::new("test".to_string(), 300);
    let ttl = entry.ttl_seconds();

    // TTL should be close to 300 seconds (allowing for small timing differences)
    assert!(ttl >= 299);
    assert!(ttl <= 300);
}

#[test]
fn test_cache_entry_age_calculation() {
    let entry = CacheEntry::new("test".to_string(), 300);
    let age = entry.age_seconds();

    // Age should be very small for newly created entry
    assert!(age >= 0);
    assert!(age < 5);
}

#[test]
fn test_cache_stats_hit_rate_edge_cases() {
    let mut stats = CacheStats::default();

    // No operations - hit rate should be 0
    assert_eq!(stats.hit_rate, 0.0);

    // Only hits
    stats.record_hit();
    stats.record_hit();
    assert_eq!(stats.hit_rate, 1.0);

    // Only misses
    let mut stats2 = CacheStats::default();
    stats2.record_miss();
    stats2.record_miss();
    assert_eq!(stats2.hit_rate, 0.0);
}

#[test]
fn test_cache_key_generation_consistency() {
    let key1 = utils::generate_cache_key("test", "same_data");
    let key2 = utils::generate_cache_key("test", "same_data");
    let key3 = utils::generate_cache_key("test", "different_data");

    assert_eq!(key1, key2); // Same input should produce same key
    assert_ne!(key1, key3); // Different input should produce different key
}

#[test]
fn test_cache_entry_serialization() -> Result<()> {
    let entry = CacheEntry::new("test_value".to_string(), 300);

    // Test JSON serialization
    let json = serde_json::to_string(&entry)?;
    assert!(!json.is_empty());

    // Test deserialization
    let deserialized: CacheEntry<String> = serde_json::from_str(&json)?;
    assert_eq!(entry.value, deserialized.value);

    Ok(())
}
