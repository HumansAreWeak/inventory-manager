CREATE TABLE invman_users(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username VARCHAR(1024) NOT NULL UNIQUE,
    display_name TEXT DEFAULT NULL,
    role_id INT NOT NULL,
    password TEXT NOT NULL,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL
);
