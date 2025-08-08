//! Wallet caching module

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{models::*, Wallet};

/// Wallet cache for fast access
pub struct WalletCache {
    wallets: Arc<RwLock<HashMap<Uuid, Wallet>>>,
    balances: Arc<RwLock<HashMap<String, Balance>>>, // address -> balance
}

impl WalletCache {
    /// Creates new cache
    pub fn new() -> Self {
        Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets wallet from cache
    pub async fn get_wallet(&self, id: &Uuid) -> Option<Wallet> {
        let wallets = self.wallets.read().await;
        wallets.get(id).cloned()
    }

    /// Puts wallet in cache
    pub async fn put_wallet(&self, wallet: Wallet) {
        let mut wallets = self.wallets.write().await;
        wallets.insert(wallet.id, wallet);
    }

    /// Removes wallet from cache
    pub async fn remove_wallet(&self, id: &Uuid) {
        let mut wallets = self.wallets.write().await;
        wallets.remove(id);
    }

    /// Gets balance from cache
    pub async fn get_balance(&self, address: &str) -> Option<Balance> {
        let balances = self.balances.read().await;
        balances.get(address).cloned()
    }

    /// Puts balance in cache
    pub async fn put_balance(&self, address: String, balance: Balance) {
        let mut balances = self.balances.write().await;
        balances.insert(address, balance);
    }

    /// Clears all cache
    pub async fn clear(&self) {
        let mut wallets = self.wallets.write().await;
        let mut balances = self.balances.write().await;
        wallets.clear();
        balances.clear();
    }

    /// Gets cache statistics
    pub async fn stats(&self) -> CacheStats {
        let wallets = self.wallets.read().await;
        let balances = self.balances.read().await;

        CacheStats {
            wallet_count: wallets.len(),
            balance_count: balances.len(),
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub wallet_count: usize,
    pub balance_count: usize,
}
