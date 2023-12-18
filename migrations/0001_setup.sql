CREATE TABLE IF NOT EXISTS invman_users(
    id INT PRIMARY KEY AUTOINCREMENT,
    username VARCHAR(1024) NOT NULL UNIQUE,
    display_name TEXT DEFAULT NULL,
    role_id INT NOT NULL,
    password TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT DEFAULT
);

CREATE TABLE IF NOT EXISTS invman_sessions(
    id INT PRIMARY KEY AUTOINCREMENT,
    token VARCHAR(1024) NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    valid_until TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS invman_roles(
    id INT PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(1024) NOT NULL UNIQUE,
    display_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS invman_config(
    id INT PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(256) NOT NULL UNIQUE,
    value TEXT
);

CREATE TABLE IF NOT EXISTS invman_article(
    id INT PRIMARY KEY AUTOINCREMENT
);
