CREATE TABLE invman_config(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(256) NOT NULL UNIQUE,
    value TEXT,
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW'))
);
