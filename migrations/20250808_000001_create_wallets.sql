-- SQLx migration: create wallets table (aligned with WalletManager schema)
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
);

CREATE INDEX IF NOT EXISTS idx_wallets_chain ON wallets (chain);
CREATE INDEX IF NOT EXISTS idx_wallets_status ON wallets (status);
CREATE INDEX IF NOT EXISTS idx_wallets_name ON wallets (name);

