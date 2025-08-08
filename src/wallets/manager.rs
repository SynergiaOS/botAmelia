//! Wallet Manager - Core wallet operations

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    models::*, Chain, CreateWalletRequest, Pagination, UpdateWalletRequest, Wallet, WalletFilters,
    WalletListResponse, WalletStatus, WalletType,
};

/// Wallet Manager handles all wallet operations
pub struct WalletManager {
    db: Arc<SqlitePool>,
    cache: Arc<RwLock<HashMap<Uuid, Wallet>>>,
    sync_intervals: HashMap<Chain, i64>, // minutes
}

impl WalletManager {
    /// Creates new wallet manager
    pub async fn new(db: Arc<SqlitePool>) -> Result<Self> {
        let manager = Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
            sync_intervals: Self::default_sync_intervals(),
        };

        // Initialize database tables
        manager.init_tables().await?;

        // Load wallets into cache
        manager.load_cache().await?;

        info!("WalletManager initialized");
        Ok(manager)
    }

    /// Default sync intervals for each chain (in minutes)
    fn default_sync_intervals() -> HashMap<Chain, i64> {
        let mut intervals = HashMap::new();
        intervals.insert(Chain::Ethereum, 1); // 1 minute
        intervals.insert(Chain::BinanceSmartChain, 1);
        intervals.insert(Chain::Polygon, 1);
        intervals.insert(Chain::Bitcoin, 2); // 2 minutes
        intervals
    }

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        // Wallets table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wallets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                chain TEXT NOT NULL,
                status TEXT NOT NULL,
                xpub TEXT,
                tags TEXT, -- JSON array
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_sync TEXT,
                metadata TEXT -- JSON object
            )
            "#,
        )
        .execute(&*self.db)
        .await
        .context("Failed to create wallets table")?;

        // Addresses table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS addresses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT NOT NULL,
                address TEXT NOT NULL,
                chain TEXT NOT NULL,
                label TEXT,
                derivation_path TEXT,
                balance_native TEXT,
                balance_tokens TEXT, -- JSON object
                balance_updated_at TEXT,
                balance_block_number INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES wallets (id) ON DELETE CASCADE,
                UNIQUE(wallet_id, address)
            )
            "#,
        )
        .execute(&*self.db)
        .await
        .context("Failed to create addresses table")?;

        // Sync stats table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT NOT NULL,
                last_sync TEXT NOT NULL,
                sync_duration_ms INTEGER NOT NULL,
                addresses_synced INTEGER NOT NULL,
                transactions_found INTEGER NOT NULL,
                errors TEXT, -- JSON array
                next_sync_at TEXT,
                FOREIGN KEY (wallet_id) REFERENCES wallets (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&*self.db)
        .await
        .context("Failed to create sync_stats table")?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_wallets_chain ON wallets(chain)")
            .execute(&*self.db)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_wallets_status ON wallets(status)")
            .execute(&*self.db)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_addresses_wallet ON addresses(wallet_id)")
            .execute(&*self.db)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses(address)")
            .execute(&*self.db)
            .await?;

        info!("Database tables initialized");
        Ok(())
    }

    /// Load wallets into cache
    async fn load_cache(&self) -> Result<()> {
        let wallets = self.list_wallets_from_db(None, None).await?;
        let mut cache = self.cache.write().await;

        for wallet in wallets {
            cache.insert(wallet.id, wallet);
        }

        info!("Loaded {} wallets into cache", cache.len());
        Ok(())
    }

    /// Creates a new wallet
    pub async fn create_wallet(&self, request: CreateWalletRequest) -> Result<Wallet> {
        let wallet = request.into_wallet()?;

        // Save to database
        self.save_wallet_to_db(&wallet).await?;

        // Add to cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(wallet.id, wallet.clone());
        }

        info!("Created wallet: {} ({})", wallet.name, wallet.id);
        Ok(wallet)
    }

    /// Gets a wallet by ID
    pub async fn get_wallet(&self, id: Uuid) -> Result<Option<Wallet>> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(wallet) = cache.get(&id) {
                return Ok(Some(wallet.clone()));
            }
        }

        // Load from database
        let wallet = self.load_wallet_from_db(id).await?;

        // Update cache if found
        if let Some(ref wallet) = wallet {
            let mut cache = self.cache.write().await;
            cache.insert(id, wallet.clone());
        }

        Ok(wallet)
    }

    /// Updates a wallet
    pub async fn update_wallet(&self, id: Uuid, request: UpdateWalletRequest) -> Result<Wallet> {
        let mut wallet = self
            .get_wallet(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Wallet not found"))?;

        // Apply updates
        if let Some(name) = request.name {
            wallet.name = name;
        }
        if let Some(status) = request.status {
            wallet.status = status;
        }
        if let Some(tags) = request.tags {
            wallet.tags = tags;
        }
        if let Some(metadata) = request.metadata {
            wallet.metadata = metadata;
        }

        wallet.updated_at = chrono::Utc::now();

        // Validate
        wallet.validate()?;

        // Save to database
        self.save_wallet_to_db(&wallet).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id, wallet.clone());
        }

        info!("Updated wallet: {} ({})", wallet.name, wallet.id);
        Ok(wallet)
    }

    /// Deletes a wallet
    pub async fn delete_wallet(&self, id: Uuid) -> Result<()> {
        // Delete from database
        sqlx::query("DELETE FROM wallets WHERE id = ?")
            .bind(id.to_string())
            .execute(&*self.db)
            .await
            .context("Failed to delete wallet from database")?;

        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.remove(&id);
        }

        info!("Deleted wallet: {}", id);
        Ok(())
    }

    /// Lists wallets with filters and pagination
    pub async fn list_wallets(
        &self,
        filters: Option<WalletFilters>,
        pagination: Option<Pagination>,
    ) -> Result<WalletListResponse> {
        let pagination = pagination.unwrap_or_default();
        let page = pagination.page.unwrap_or(1);
        let limit = pagination.limit.unwrap_or(50);

        // For now, load from cache and filter in memory
        // In production, this should be done in the database
        let cache = self.cache.read().await;
        let mut wallets: Vec<Wallet> = cache.values().cloned().collect();

        // Apply filters
        if let Some(filters) = filters {
            wallets = self.apply_filters(wallets, filters).await?;
        }

        let total = wallets.len() as u64;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

        // Apply pagination
        let start = ((page - 1) * limit) as usize;
        let end = (start + limit as usize).min(wallets.len());
        wallets = wallets[start..end].to_vec();

        Ok(WalletListResponse {
            wallets,
            total,
            page,
            limit,
            total_pages,
        })
    }

    /// Apply filters to wallet list
    async fn apply_filters(
        &self,
        mut wallets: Vec<Wallet>,
        filters: WalletFilters,
    ) -> Result<Vec<Wallet>> {
        if let Some(chain) = filters.chain {
            wallets.retain(|w| w.chain == chain);
        }

        if let Some(wallet_type) = filters.wallet_type {
            wallets.retain(|w| w.wallet_type == wallet_type);
        }

        if let Some(status) = filters.status {
            wallets.retain(|w| w.status == status);
        }

        if let Some(tags) = filters.tags {
            wallets.retain(|w| tags.iter().any(|tag| w.tags.contains(tag)));
        }

        if let Some(name_contains) = filters.name_contains {
            let name_lower = name_contains.to_lowercase();
            wallets.retain(|w| w.name.to_lowercase().contains(&name_lower));
        }

        if let Some(needs_sync) = filters.needs_sync {
            if needs_sync {
                let sync_threshold = 60; // 60 minutes
                wallets.retain(|w| w.needs_sync(sync_threshold));
            }
        }

        Ok(wallets)
    }

    /// Gets wallets that need syncing
    pub async fn get_wallets_needing_sync(&self) -> Result<Vec<Wallet>> {
        let cache = self.cache.read().await;
        let mut wallets = Vec::new();

        for wallet in cache.values() {
            if wallet.status != WalletStatus::Active {
                continue;
            }

            let threshold = self.sync_intervals.get(&wallet.chain).unwrap_or(&60);
            if wallet.needs_sync(*threshold) {
                wallets.push(wallet.clone());
            }
        }

        Ok(wallets)
    }

    /// Save wallet to database
    async fn save_wallet_to_db(&self, wallet: &Wallet) -> Result<()> {
        let tags_json = serde_json::to_string(&wallet.tags)?;
        let metadata_json = serde_json::to_string(&wallet.metadata)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO wallets
            (id, name, wallet_type, chain, status, xpub, tags, created_at, updated_at, last_sync, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(wallet.id.to_string())
        .bind(&wallet.name)
        .bind(serde_json::to_string(&wallet.wallet_type)?)
        .bind(serde_json::to_string(&wallet.chain)?)
        .bind(serde_json::to_string(&wallet.status)?)
        .bind(&wallet.xpub)
        .bind(tags_json)
        .bind(wallet.created_at.to_rfc3339())
        .bind(wallet.updated_at.to_rfc3339())
        .bind(wallet.last_sync.map(|dt| dt.to_rfc3339()))
        .bind(metadata_json)
        .execute(&*self.db)
        .await
        .context("Failed to save wallet to database")?;

        // Save addresses
        for address in &wallet.addresses {
            self.save_address_to_db(&wallet.id, address).await?;
        }

        Ok(())
    }

    /// Save address to database
    async fn save_address_to_db(&self, wallet_id: &Uuid, address: &Address) -> Result<()> {
        let balance_tokens = address
            .balance
            .as_ref()
            .map(|b| serde_json::to_string(&b.tokens))
            .transpose()?
            .unwrap_or_else(|| "{}".to_string());

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO addresses
            (wallet_id, address, chain, label, derivation_path, balance_native, balance_tokens,
             balance_updated_at, balance_block_number, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(wallet_id.to_string())
        .bind(&address.address)
        .bind(serde_json::to_string(&address.chain)?)
        .bind(&address.label)
        .bind(&address.derivation_path)
        .bind(address.balance.as_ref().map(|b| &b.native))
        .bind(balance_tokens)
        .bind(address.balance.as_ref().map(|b| b.updated_at.to_rfc3339()))
        .bind(
            address
                .balance
                .as_ref()
                .and_then(|b| b.block_number)
                .map(|n| n as i64),
        )
        .bind(address.created_at.to_rfc3339())
        .bind(address.updated_at.to_rfc3339())
        .execute(&*self.db)
        .await
        .context("Failed to save address to database")?;

        Ok(())
    }

    /// Load wallet from database
    async fn load_wallet_from_db(&self, id: Uuid) -> Result<Option<Wallet>> {
        let row = sqlx::query(
            r#"SELECT id, name, wallet_type, chain, status, xpub, tags, created_at, updated_at, last_sync, metadata
               FROM wallets WHERE id = ?"#,
        )
        .bind(id.to_string())
        .fetch_optional(&*self.db)
        .await
        .context("Failed to load wallet from database")?;

        let Some(row) = row else {
            return Ok(None);
        };

        let wallet_type: WalletType = serde_json::from_str(&row.get::<String, _>("wallet_type"))
            .context("Invalid wallet_type in DB")?;
        let chain: Chain =
            serde_json::from_str(&row.get::<String, _>("chain")).context("Invalid chain in DB")?;
        let status: WalletStatus = serde_json::from_str(&row.get::<String, _>("status"))
            .context("Invalid status in DB")?;
        let tags: Vec<String> = serde_json::from_str(
            &row.get::<Option<String>, _>("tags")
                .unwrap_or_else(|| "[]".into()),
        )
        .context("Invalid tags JSON in DB")?;
        let metadata: HashMap<String, String> = serde_json::from_str(
            &row.get::<Option<String>, _>("metadata")
                .unwrap_or_else(|| "{}".into()),
        )
        .context("Invalid metadata JSON in DB")?;

        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))?;
        let last_sync = row.get::<Option<String>, _>("last_sync").and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        let id_str: String = row.get("id");
        let name: String = row.get("name");
        let xpub: Option<String> = row.get("xpub");

        // Load addresses
        let addresses = self.load_addresses_for_wallet(&id_str).await?;

        let wallet = Wallet {
            id,
            name,
            wallet_type,
            chain,
            status,
            addresses,
            xpub,
            tags,
            created_at,
            updated_at,
            last_sync,
            metadata,
        };

        Ok(Some(wallet))
    }

    /// List wallets from database
    async fn list_wallets_from_db(
        &self,
        _filters: Option<WalletFilters>,
        _pagination: Option<Pagination>,
    ) -> Result<Vec<Wallet>> {
        let rows = sqlx::query(
            r#"SELECT id, name, wallet_type, chain, status, xpub, tags, created_at, updated_at, last_sync, metadata FROM wallets"#
        )
        .fetch_all(&*self.db)
        .await
        .context("Failed to list wallets from database")?;

        let mut wallets = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let id = Uuid::parse_str(&id_str)?;
            let wallet_type: WalletType =
                serde_json::from_str(&row.get::<String, _>("wallet_type"))?;
            let chain: Chain = serde_json::from_str(&row.get::<String, _>("chain"))?;
            let status: WalletStatus = serde_json::from_str(&row.get::<String, _>("status"))?;
            let tags: Vec<String> = serde_json::from_str(
                &row.get::<Option<String>, _>("tags")
                    .unwrap_or_else(|| "[]".into()),
            )?;
            let metadata: HashMap<String, String> = serde_json::from_str(
                &row.get::<Option<String>, _>("metadata")
                    .unwrap_or_else(|| "{}".into()),
            )?;
            let created_at_str: String = row.get("created_at");
            let updated_at_str: String = row.get("updated_at");
            let created_at =
                chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);
            let updated_at =
                chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&chrono::Utc);
            let last_sync = row.get::<Option<String>, _>("last_sync").and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });
            let name: String = row.get("name");
            let xpub: Option<String> = row.get("xpub");
            let addresses = self.load_addresses_for_wallet(&id_str).await?;

            wallets.push(Wallet {
                id,
                name,
                wallet_type,
                chain,
                status,
                addresses,
                xpub,
                tags,
                created_at,
                updated_at,
                last_sync,
                metadata,
            });
        }

        Ok(wallets)
    }

    /// Load addresses for a wallet
    async fn load_addresses_for_wallet(&self, wallet_id: &str) -> Result<Vec<Address>> {
        let rows = sqlx::query(
            r#"SELECT address, chain, label, derivation_path, balance_native, balance_tokens, balance_updated_at, balance_block_number, created_at, updated_at
               FROM addresses WHERE wallet_id = ?"#,
        )
        .bind(wallet_id)
        .fetch_all(&*self.db)
        .await
        .context("Failed to load addresses for wallet")?;

        let mut addresses = Vec::new();
        for row in rows {
            let chain_str: String = row.get("chain");
            let chain: Chain = serde_json::from_str(&chain_str)?;
            let address_str: String = row.get("address");
            let mut address = Address::new(address_str, &chain)?;
            address.label = row.get("label");
            address.derivation_path = row.get("derivation_path");

            // Balance
            if let Some(native) = row.get::<Option<String>, _>("balance_native") {
                let tokens: HashMap<String, TokenBalance> = serde_json::from_str(
                    &row.get::<Option<String>, _>("balance_tokens")
                        .unwrap_or_else(|| "{}".into()),
                )?;
                let updated_at = row
                    .get::<Option<String>, _>("balance_updated_at")
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    });
                let block_number = row
                    .get::<Option<i64>, _>("balance_block_number")
                    .map(|n| n as u64);
                let mut bal = Balance::new(native);
                bal.tokens = tokens;
                bal.updated_at = updated_at.unwrap_or_else(chrono::Utc::now);
                bal.block_number = block_number;
                address.balance = Some(bal);
            }

            let created_at_str: String = row.get("created_at");
            let updated_at_str: String = row.get("updated_at");
            address.created_at =
                chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);
            address.updated_at =
                chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&chrono::Utc);

            addresses.push(address);
        }

        Ok(addresses)
    }
    /// Save sync stats to database
    pub async fn save_sync_stats(&self, stats: &SyncStats) -> Result<()> {
        let errors_json = serde_json::to_string(&stats.errors)?;

        sqlx::query(
            r#"INSERT OR REPLACE INTO sync_stats
               (wallet_id, last_sync, sync_duration_ms, addresses_synced, transactions_found, errors, next_sync_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(stats.wallet_id.to_string())
        .bind(stats.last_sync.to_rfc3339())
        .bind(stats.sync_duration_ms as i64)
        .bind(stats.addresses_synced as i64)
        .bind(stats.transactions_found as i64)
        .bind(errors_json)
        .bind(stats.next_sync_at.map(|dt| dt.to_rfc3339()))
        .execute(&*self.db)
        .await
        .context("Failed to save sync stats to database")?;

        Ok(())
    }

    /// Get sync stats for a wallet
    pub async fn get_sync_stats(&self, wallet_id: Uuid) -> Result<Option<SyncStats>> {
        let row = sqlx::query(
            r#"SELECT wallet_id, last_sync, sync_duration_ms, addresses_synced, transactions_found, errors, next_sync_at
               FROM sync_stats WHERE wallet_id = ? ORDER BY last_sync DESC LIMIT 1"#,
        )
        .bind(wallet_id.to_string())
        .fetch_optional(&*self.db)
        .await
        .context("Failed to get sync stats from database")?;

        let Some(row) = row else {
            return Ok(None);
        };

        let last_sync_str: String = row.get("last_sync");
        let last_sync =
            chrono::DateTime::parse_from_rfc3339(&last_sync_str)?.with_timezone(&chrono::Utc);
        let errors: Vec<String> = serde_json::from_str(
            &row.get::<Option<String>, _>("errors")
                .unwrap_or_else(|| "[]".into()),
        )?;
        let next_sync_at = row.get::<Option<String>, _>("next_sync_at").and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        let stats = SyncStats {
            wallet_id,
            last_sync,
            sync_duration_ms: row.get::<i64, _>("sync_duration_ms") as u64,
            addresses_synced: row.get::<i64, _>("addresses_synced") as u32,
            transactions_found: row.get::<i64, _>("transactions_found") as u32,
            errors,
            next_sync_at,
        };

        Ok(Some(stats))
    }
}
