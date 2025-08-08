-- SQLx migration: create transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    hash TEXT NOT NULL,
    chain TEXT NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT,
    value TEXT NOT NULL,
    gas_used TEXT,
    gas_price TEXT,
    block_number INTEGER,
    block_hash TEXT,
    transaction_index INTEGER,
    status TEXT NOT NULL,
    timestamp DATETIME,
    confirmations INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tx_wallet ON transactions (wallet_id);
CREATE INDEX IF NOT EXISTS idx_tx_hash ON transactions (hash);
CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions (block_number);

