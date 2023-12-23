CREATE TABLE invman_inventory_tx(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dispatcher INTEGER NOT NULL,
    schema_id INTEGER NOT NULL,
    inventory_id INTEGER NOT NULL,
    action_no INTEGER NOT NULL,
    from_val TEXT DEFAULT NULL,
    to_val TEXT DEFAULT NULL,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    FOREIGN KEY(dispatcher) REFERENCES invman_users(id),
    FOREIGN KEY(schema_id) REFERENCES invman_inventory_schema_tx(id)
);
