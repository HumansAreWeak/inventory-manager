use rusqlite::{Connection, Result};

pub trait InvManActions {
    fn user_register(&self, username: String, password: String) -> Result<String>;
    fn user_login(&self, username: String, password: String) -> Result<String>;
}

pub struct DBSQLite {
    db: Connection,
}

impl DBSQLite {
    pub fn new() -> DBSQLite {
        let conn = Connection::open("./storage").expect("Could not open storage file");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS invman_users(
            id INT PRIMARY KEY AUTO_INCREMENT,
            username VARCHAR(1024) NOT NULL UNIQUE,
            display_name TEXT,
            role_id INT NOT NULL,
            password TEXT NOT NULL,
            created_at INT NOT NULL,
            updated_at INT NOT NULL,
            deleted_at INT NOT NULL
        )",
            (),
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS invman_sessions(
            id INT PRIMARY KEY AUTO_INCREMENT,
            token VARCHAR(1024) NOT NULL UNIQUE,
            created_at INT NOT NULL,
            valid_until INT NOT NULL
            )",
            (),
        );

        return DBSQLite { db: conn };
    }
}

impl InvManActions for DBSQLite {
    fn user_register(&self, username: String, password: String) -> Result<String> {
        self.db.Ok(String::from(""))
    }

    fn user_login(&self, username: String, password: String) -> Result<String> {
        Ok(String::from(""))
    }
}
