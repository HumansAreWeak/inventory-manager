use crate::commands::{ColumnType, DBUser, SchemaDeclaration};
use anyhow::{bail, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use rusqlite::params;
use std::path::Path;

pub trait InvManDBPool {
    fn get_config(&self) -> AppConfig;
    fn user_register(&self, username: &str, password: &str) -> Result<String>;
    fn user_auth(&self, username: &str, password: &str, user: &mut DBUser) -> Result<String>;

    fn schema_alter(&mut self, config: &mut AppConfig, decl: SchemaDeclaration) -> Result<String>;
}

#[derive(Default)]
pub struct AppConfig {
    pub allow_registration: bool,
    pub schema_declaration: Vec<SchemaDeclaration>,
}

pub struct InvManSqlite {
    db: rusqlite::Connection,
}

struct Count {
    count: u32,
}

struct IdPassword {
    id: u32,
    password: String,
}

struct Config {
    name: String,
    value: String,
}

impl InvManSqlite {
    pub fn new() -> InvManSqlite {
        let file = Path::new("./storage");
        let file_exists = file.exists();
        let conn = rusqlite::Connection::open(file.to_str().unwrap()).unwrap();

        if !file_exists {
            // Create all the tables
            conn.execute(include_str!("./sql/sqlite/create_users_table.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/create_roles_table.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/create_config_table.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/create_articles_table.sql"), ())
                .unwrap();

            // Inserting default values into the database
            conn.execute(include_str!("./sql/sqlite/insert_default_config.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/insert_default_roles.sql"), ())
                .unwrap();

            // Creating all necessary triggers
            conn.execute(include_str!("./sql/sqlite/create_users_trigger.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/create_config_trigger.sql"), ())
                .unwrap();
            conn.execute(include_str!("./sql/sqlite/create_roles_trigger.sql"), ())
                .unwrap();
        }

        return InvManSqlite { db: conn };
    }

    fn user_count(&self) -> Result<u32> {
        let mut stmt = self
            .db
            .prepare("SELECT COUNT(*) AS count FROM invman_users WHERE deleted_at IS NULL")?;
        let count_iter = stmt.query_map([], |row| Ok(Count { count: row.get(0)? }))?;

        for count in count_iter {
            return Ok(count?.count);
        }

        return Ok(0);
    }

    fn is_username_unique(&self, username: &str) -> Result<bool> {
        let mut stmt = self
            .db
            .prepare("SELECT COUNT(*) AS count FROM invman_users WHERE username=?1")?;
        let mut rows = stmt.query(params![username])?;
        let mut counter = 0;
        while let Some(row) = rows.next()? {
            counter = row.get(0)?;
        }
        return Ok(counter == 0);
    }

    fn make_row_statement(&self, decl: &SchemaDeclaration) -> String {
        let mut query = format!("{}", decl.name);

        match decl.column_type {
            ColumnType::BOOL => query.push_str(" VARCHAR(5)"),
            ColumnType::INT => query.push_str(" INTEGER"),
            ColumnType::REAL => query.push_str(" REAL"),
            ColumnType::TEXT => query.push_str(" TEXT"),
            ColumnType::VARCHAR => {
                query.push_str(" VARCHAR(");
                query.push_str(decl.max_length.to_string().as_str());
                query.push(')');
            }
        };

        if !decl.nullable {
            query.push_str(" NOT NULL");
        }

        if decl.default != "NULL" {
            let string;
            let default = match decl.default.as_str() {
                "CURRENT_TIMESTAMP" => "(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW'))",
                s => match decl.column_type {
                    ColumnType::TEXT | ColumnType::VARCHAR => {
                        string = format!("'{}'", s);
                        &string
                    }
                    _ => s,
                },
            };
            let default = format!(" DEFAULT {}", default);
            query.push_str(default.as_str());
        }

        if decl.unique {
            query.push_str(" UNIQUE");
        }

        return query;
    }

    fn make_temp_articles_table(&self, declarations: &Vec<SchemaDeclaration>) -> String {
        let mut query = if declarations.is_empty() {
            return String::from(
                r#"
CREATE TABLE invman_temp_articles(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL
);
"#,
            );
        } else {
            String::from(
                r#"
CREATE TABLE invman_temp_articles(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL,
"#,
            )
        };

        let count = declarations.iter().count();
        let mut i = 0;
        declarations
            .iter()
            .map(|e| self.make_row_statement(e))
            .for_each(|e| {
                i += 1;
                query.push_str(e.as_str());
                if i != count {
                    query.push(',');
                }
            });

        if !declarations.is_empty() {
            query.push_str(");");
        }

        return query;
    }

    fn make_copy_columns(&self, declarations: &Vec<SchemaDeclaration>) -> String {
        let mut cols = String::from("id,created_at,updated_at,deleted_at");
        declarations.iter().for_each(|e| {
            cols.push(',');
            cols.push_str(&e.name);
        });
        return cols;
    }
}

impl Default for InvManSqlite {
    fn default() -> Self {
        Self::new()
    }
}

impl InvManDBPool for InvManSqlite {
    fn get_config(&self) -> AppConfig {
        let mut stmt = self
            .db
            .prepare("SELECT name, value FROM invman_config")
            .unwrap();
        let config_iter = stmt
            .query_map([], |row| {
                Ok(Config {
                    name: row.get(0).unwrap(),
                    value: row.get(1).unwrap(),
                })
            })
            .unwrap();
        let mut app_config = AppConfig::default();
        for config in config_iter {
            let config = config.unwrap();
            match config.name.as_str() {
                "allow_registration" => {
                    app_config.allow_registration = config.value == "true";
                }
                "schema_declaration" => {
                    app_config.schema_declaration =
                        serde_json::from_str(config.value.as_str()).unwrap();
                }
                _ => continue,
            }
        }
        return app_config;
    }

    fn user_register(&self, username: &str, password: &str) -> Result<String> {
        if !self.is_username_unique(username)? {
            bail!("Username already taken");
        }
        let role_id = if self.user_count()? == 0 { 1 } else { 2 };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        self.db.execute(
            "INSERT INTO invman_users (username, role_id, password) VALUES (?1, ?2, ?3)",
            (username, role_id, password_hash),
        )?;

        Ok("Successfully registered new user".into())
    }

    fn user_auth(&self, username: &str, password: &str, user: &mut DBUser) -> Result<String> {
        let mut stmt = self.db.prepare(
            "SELECT id, password FROM invman_users WHERE username=?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query(params![username])?;
        let mut fetched_user = IdPassword {
            id: 0,
            password: "".into(),
        };
        while let Some(row) = rows.next()? {
            fetched_user = IdPassword {
                id: row.get(0)?,
                password: row.get(1)?,
            };
        }
        if fetched_user.id == 0 || fetched_user.password.is_empty() {
            bail!("Either username or password is incorrect");
        }
        let parsed_hash = PasswordHash::new(&fetched_user.password)?;
        if !Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            bail!("Either username or password is incorrect");
        }

        // Store the ID of the fetched user for usage in other areas of the program
        user.id = fetched_user.id;
        return Ok("User was authorized".into());
    }

    fn schema_alter(&mut self, config: &mut AppConfig, decl: SchemaDeclaration) -> Result<String> {
        let copy_table = format!(
            "INSERT INTO invman_temp_articles({cols}) SELECT {cols} FROM invman_articles",
            cols = self.make_copy_columns(&config.schema_declaration)
        );
        let db_decl = config
            .schema_declaration
            .iter()
            .position(|d| d.is_equal(&decl));

        if db_decl.is_some() {
            let mut schema_declaration = config.schema_declaration.clone();
            schema_declaration.remove(db_decl.unwrap());
            config.schema_declaration = schema_declaration;
        }
        config.schema_declaration.push(decl);
        let create_articles_table = self.make_temp_articles_table(&config.schema_declaration);

        let tx = self.db.transaction()?;
        tx.execute(&create_articles_table, ())?;
        tx.execute(&copy_table, ())?;
        tx.execute("DROP TABLE invman_articles", ())?;
        tx.execute(
            "ALTER TABLE invman_temp_articles RENAME TO invman_articles",
            (),
        )?;
        tx.execute(r#"
CREATE TRIGGER update_articles_updated_at AFTER UPDATE ON invman_articles
       BEGIN
            UPDATE invman_articles SET updated_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=new.id;
       END;
"#, ())?;
        tx.execute(
            "UPDATE invman_config SET value=?1 WHERE name='schema_declaration'",
            [serde_json::to_string(&config.schema_declaration)?],
        )?;
        tx.commit()?;

        Ok("".into())
    }
}
