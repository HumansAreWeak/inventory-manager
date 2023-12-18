CREATE TABLE invman_roles(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(1024) NOT NULL UNIQUE,
    display_name VARCHAR(1024),
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL
);
