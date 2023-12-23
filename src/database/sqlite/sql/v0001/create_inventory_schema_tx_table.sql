CREATE TABLE invman_inventory_schema_tx(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dispatcher INTEGER NOT NULL,
    action_no INTEGER NOT NULL,
    from_val TEXT NOT NULL,
    to_val TEXT NOT NULL,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    FOREIGN KEY(dispatcher) REFERENCES invman_users(id)
);
