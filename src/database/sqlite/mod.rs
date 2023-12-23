use super::{
    AppConfig, DBOpNo, EventActionNo, IdEntry, InvManDBPool, KeyValueEntry, SchemaActionNo,
};
use super::{Config, Count};
use crate::commands::{ColumnType, DBUser, InventoryListProps, SchemaDeclaration};
use crate::database::{IdPassword, JsonEntry};
use crate::utils::InvManDbHelper;
use anyhow::{bail, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use rusqlite::types::Type;
use rusqlite::{params, types::Value};
use rusqlite::{Connection, Row};
use std::path::Path;

pub struct InvManSqlite {
    db: Connection,
}

impl JsonEntry {
    fn new(row: &Row<'_>) -> Result<JsonEntry> {
        Ok(JsonEntry {
            json: row
                .as_ref()
                .column_names()
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let key = e.to_owned().into();
                    let val_ref = row.get_ref(i)?;
                    let value = match val_ref.data_type() {
                        Type::Blob => {
                            if let Some(val) = val_ref.as_blob_or_null()? {
                                std::str::from_utf8(val).unwrap().to_string()
                            } else {
                                "null".into()
                            }
                        }
                        Type::Integer => {
                            if let Some(val) = val_ref.as_i64_or_null()? {
                                val.to_string()
                            } else {
                                "null".into()
                            }
                        }
                        Type::Null => "null".into(),
                        Type::Real => {
                            if let Some(val) = val_ref.as_f64_or_null()? {
                                val.to_string()
                            } else {
                                "null".into()
                            }
                        }
                        Type::Text => {
                            if let Some(val) = val_ref.as_str_or_null()? {
                                format!("\"{}\"", val.to_string())
                            } else {
                                "null".into()
                            }
                        }
                    };
                    Ok(KeyValueEntry { key, value })
                })
                .into_iter()
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl InvManSqlite {
    pub fn new() -> Result<InvManSqlite> {
        let file = Path::new("./storage");
        let file_exists = file.exists();
        let mut conn = InvManSqlite {
            db: Connection::open(file.to_str().unwrap_or(""))?,
        };

        if !file_exists {
            conn.create_inital_setup()?;
        }

        return Ok(conn);
    }

    fn create_inital_setup(&mut self) -> Result<()> {
        let tx = self.db.transaction().unwrap();
        let exec = |content: &str| tx.execute(content, ());
        // Create all the tables
        exec(include_str!("./sql/v0001/create_users_table.sql"))?;
        exec(include_str!("./sql/v0001/create_roles_table.sql"))?;
        exec(include_str!("./sql/v0001/create_config_table.sql"))?;
        exec(include_str!("./sql/v0001/create_inventory_table.sql"))?;
        exec(include_str!("./sql/v0001/create_inventory_tx_table.sql"))?;
        exec(include_str!(
            "./sql/v0001/create_inventory_schema_tx_table.sql"
        ))?;
        exec(include_str!("./sql/v0001/create_event_tx_table.sql"))?;

        // Inserting default values into the database
        exec(include_str!("./sql/v0001/insert_default_config.sql"))?;
        exec(include_str!("./sql/v0001/insert_default_roles.sql"))?;

        // Creating all necessary triggers
        exec(include_str!(
            "./sql/v0001/after_user_registration_trigger.sql"
        ))?;
        exec(include_str!("./sql/v0001/create_users_trigger.sql"))?;
        exec(include_str!("./sql/v0001/create_config_trigger.sql"))?;
        exec(include_str!("./sql/v0001/create_roles_trigger.sql"))?;
        exec(include_str!("./sql/v0001/create_inventory_trigger.sql"))?;

        tx.commit()?;
        Ok(())
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

    fn make_temp_inventory_table(&self, declarations: &Vec<SchemaDeclaration>) -> String {
        let mut query = if declarations.is_empty() {
            return String::from(
                r#"
CREATE TABLE invman_temp_inventory(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    deleted_at TEXT DEFAULT NULL
);"#,
            );
        } else {
            String::from(
                r#"
CREATE TABLE invman_temp_inventory(
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

    fn alter_inventory_table(
        &mut self,
        new_schema: &Vec<SchemaDeclaration>,
        old_schema: &Vec<SchemaDeclaration>,
        action_no: &SchemaActionNo,
        user: &DBUser,
    ) -> Result<String> {
        let old_schema_str = serde_json::to_string(old_schema)?;
        let new_schema_str = serde_json::to_string(new_schema)?;
        let create_inventory_table = self.make_temp_inventory_table(new_schema);
        let copy_table = format!(
            "INSERT INTO invman_temp_inventory({cols}) SELECT {cols} FROM invman_inventory",
            cols = self.make_copy_columns(match action_no {
                SchemaActionNo::Alter => old_schema,
                SchemaActionNo::Remove => new_schema,
            })
        );

        let tx = self.db.transaction()?;
        let exec = |sql: &str| tx.execute(sql, ());
        exec(&create_inventory_table)?;
        exec(&copy_table)?;
        exec("DROP TABLE invman_inventory")?;
        exec("ALTER TABLE invman_temp_inventory RENAME TO invman_inventory")?;
        exec(include_str!("./sql/v0001/create_inventory_trigger.sql"))?;
        tx.execute(
            "INSERT INTO invman_inventory_schema_tx (dispatcher, action_no, from_val, to_val) VALUES (?1, ?2, ?3, ?4)",
            params![user.id, *action_no as u32, old_schema_str, new_schema_str],
        )?;
        tx.execute(
            "UPDATE invman_config SET value=?1 WHERE name='inventory_schema_declaration'",
            [new_schema_str],
        )?;
        tx.commit()?;
        return Ok("Altered invman_inventory table".into());
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
                "inventory_schema_declaration" => {
                    app_config.inventory_schema_declaration =
                        serde_json::from_str(config.value.as_str()).unwrap();
                }
                _ => continue,
            }
        }
        return app_config;
    }

    fn user_register(&mut self, username: &str, password: &str) -> Result<String> {
        if !self.is_username_unique(username)? {
            bail!("Username already taken");
        }
        let role_id = if self.user_count()? == 0 { 1 } else { 2 };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        let tx = self.db.transaction()?;
        tx.execute(
            "INSERT INTO invman_users (username, role_id, password) VALUES (?1, ?2, ?3)",
            (username, role_id, password_hash),
        )?;
        tx.commit()?;

        Ok("Successfully registered new user".into())
    }

    fn user_auth(&self, username: &str, password: &str, user: &mut DBUser) -> Result<()> {
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
        return Ok(());
    }

    fn schema_alter(
        &mut self,
        config: &mut AppConfig,
        decl: SchemaDeclaration,
        user: &DBUser,
    ) -> Result<String> {
        let old_schema = config.inventory_schema_declaration.clone();
        if let Some(match_decl) = config
            .inventory_schema_declaration
            .iter()
            .position(|d| d.is_equal(&decl))
        {
            let mut schema_declaration = config.inventory_schema_declaration.clone();
            schema_declaration.remove(match_decl);
            config.inventory_schema_declaration = schema_declaration;
        }
        config.inventory_schema_declaration.push(decl);
        self.alter_inventory_table(
            &config.inventory_schema_declaration,
            &old_schema,
            &SchemaActionNo::Alter,
            user,
        )?;
        Ok("Altered schema".into())
    }

    fn schema_remove(
        &mut self,
        config: &mut AppConfig,
        name: &str,
        user: &DBUser,
    ) -> Result<String> {
        let old_schema = config.inventory_schema_declaration.clone();
        let id = config
            .inventory_schema_declaration
            .iter()
            .position(|e| e.name == name);
        if !id.is_some() {
            bail!("The name attribute provided did not match any schema column definition");
        }
        let id = id.unwrap();
        config.inventory_schema_declaration.remove(id);
        self.alter_inventory_table(
            &config.inventory_schema_declaration,
            &old_schema,
            &SchemaActionNo::Remove,
            user,
        )?;
        Ok("Removed schema column".into())
    }

    fn inventory_add(
        &mut self,
        params: &Vec<(String, String)>,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String> {
        let mut names: Vec<String> = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        params.iter().for_each(|e| {
            names.push(e.0.clone());
            values.push(e.1.clone().into());
        });
        let names = names.join(",");

        let sql = format!(
            "INSERT INTO invman_inventory ({}) VALUES ({})",
            names,
            vec!["?"; params.iter().count()].join(",")
        );
        let names = config
            .inventory_schema_declaration
            .iter()
            .map(|e| e.name.clone())
            .collect::<Vec<String>>()
            .join(",");
        let select_item_sql = format!(
            "SELECT id,created_at,updated_at,deleted_at,{} FROM invman_inventory WHERE id=?1",
            names
        );
        let tx = self.db.transaction()?;
        let latest_schema = tx.query_row(
            "SELECT MAX(id) FROM invman_inventory_schema_tx",
            (),
            |row| Ok(IdEntry { id: row.get(0)? }),
        )?;
        tx.execute(&sql, rusqlite::params_from_iter(values))?;
        let latest_item = tx.query_row("SELECT (LAST_INSERT_ROWID())", (), |row| {
            Ok(IdEntry { id: row.get(0)? })
        })?;
        let json = tx
            .query_row(&select_item_sql, params![latest_item.id], |row| {
                Ok(JsonEntry::new(row))
            })??
            .to_json();
        tx.execute("INSERT INTO invman_inventory_tx (dispatcher, schema_id, inventory_id, action_no, from_val, to_val) VALUES (?1, ?2, ?3, ?4, NULL, ?5)", params![user.id, latest_schema.id, latest_item.id, DBOpNo::Add as u32, json])?;
        tx.execute("INSERT INTO invman_event_tx (action_no, dispatcher, target) VALUES (?1, ?2, (LAST_INSERT_ROWID()))", params![EventActionNo::InventoryAdd as u32, user.id])?;
        tx.commit()?;
        return Ok("Entity was successfully added to inventory".into());
    }

    fn inventory_list(
        &self,
        props: &InventoryListProps,
        config: &AppConfig,
    ) -> Result<Vec<JsonEntry>> {
        let mut sql = format!(
            "SELECT id,created_at,updated_at,deleted_at,{} FROM invman_inventory ORDER BY id DESC",
            config.inventory_schema_declaration.to_sql_names()
        );
        if props.limit > 0 {
            sql.push_str(" LIMIT ");
            sql.push_str(props.limit.to_string().as_str());
        }
        let mut stmt = self.db.prepare(&sql)?;
        let entries = stmt.query_map((), |row| {
            Ok(JsonEntry::new(row)
                .expect("Could not convert row to its Key-Value pair representation"))
        })?;
        return Ok(entries.map(|e| e.unwrap()).collect());
    }
}
