-- SQLx migration: create addresses table (aligned with WalletManager schema)
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
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE,
    UNIQUE(wallet_id, address)
);

CREATE INDEX IF NOT EXISTS idx_addresses_wallet ON addresses (wallet_id);
CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses (address);

