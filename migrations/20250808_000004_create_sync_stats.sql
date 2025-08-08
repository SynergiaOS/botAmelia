-- SQLx migration: create sync_stats table (aligned with WalletManager schema)
CREATE TABLE IF NOT EXISTS sync_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    last_sync TEXT NOT NULL,
    sync_duration_ms INTEGER NOT NULL,
    addresses_synced INTEGER NOT NULL,
    transactions_found INTEGER NOT NULL,
    errors TEXT, -- JSON array
    next_sync_at TEXT,
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sync_stats_wallet ON sync_stats (wallet_id);
CREATE INDEX IF NOT EXISTS idx_sync_stats_last_sync ON sync_stats (last_sync);

