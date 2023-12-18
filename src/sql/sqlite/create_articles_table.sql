CREATE TABLE invman_articles(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL
);
