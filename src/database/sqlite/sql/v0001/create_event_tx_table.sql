CREATE TABLE invman_event_tx(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_no INTEGER NOT NULL,
    dispatcher INTEGER NOT NULL,
    target INTEGER DEFAULT NULL,
    reason TEXT DEFAULT NULL,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    FOREIGN KEY(dispatcher) REFERENCES invman_users(id)
);
