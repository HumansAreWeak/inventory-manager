/**
 * This file is part of invman.
 *
 * invman - Manage your inventory easily, declaratively, without the headache.
 * Copyright (C) 2023  Maik Steiger <m.steiger@csurielektronics.com>
 *
 * invman is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * invman is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with invman. If not, see <https://www.gnu.org/licenses/>.
 */
use super::{
    AppConfig, DBOpNo, EventActionNo, IdEntry, InvManDBPool, InvManToSql, KeyValueTypeEntry,
    SchemaActionNo, SchemaCollection,
};
use super::{Config, Count};
use crate::commands::{ColumnType, DBUser, InventoryListProps, SchemaDeclaration};
use crate::database::{IdPassword, KeyValueCollection};
use crate::utils::InvManSerialization;
use anyhow::{bail, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use rusqlite::params;
use rusqlite::types::Type;
use rusqlite::{params_from_iter, Connection, Row};
use std::path::Path;

pub struct InvManSqlite {
    db: Connection,
}

trait InvManTypedKeyValue {
    fn to_typed_key_value(&self, declarations: &SchemaCollection) -> Result<KeyValueCollection>;
}

impl InvManTypedKeyValue for Row<'_> {
    fn to_typed_key_value(&self, declarations: &SchemaCollection) -> Result<KeyValueCollection> {
        let items = self
            .as_ref()
            .column_names()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let key = e.to_owned();
                let val_ref = self.get_ref(i)?;
                match key {
                    "id" => {
                        let value = val_ref.as_i64()?.to_string();
                        Ok(KeyValueTypeEntry {
                            column_type: ColumnType::INT,
                            key: key.to_string(),
                            value: Some(value),
                        })
                    }
                    "created_at" | "updated_at" | "deleted_at" => {
                        let value = val_ref.as_str_or_null()?;
                        Ok(KeyValueTypeEntry {
                            column_type: ColumnType::TEXT,
                            key: key.to_string(),
                            value: match value {
                                None => None,
                                Some(val) => Some(val.to_string()),
                            },
                        })
                    }
                    _ => {
                        let decl = declarations.collection.iter().find(|e| e.name == key);
                        if decl.is_none() {
                            bail!("Declaration was not found for given key '{}'", key);
                        }
                        let decl = decl.unwrap();

                        fn string_or_none<T>(e: Option<T>) -> Option<String>
                        where
                            T: ToString,
                        {
                            if let Some(val) = e {
                                Some(val.to_string())
                            } else {
                                None
                            }
                        }

                        let value = match val_ref.data_type() {
                            Type::Blob => {
                                if let Some(val) = val_ref.as_blob_or_null()? {
                                    Some(std::str::from_utf8(val).unwrap().to_string())
                                } else {
                                    None
                                }
                            }
                            Type::Integer => string_or_none(val_ref.as_i64_or_null()?),
                            Type::Real => string_or_none(val_ref.as_f64_or_null()?),
                            Type::Text => string_or_none(val_ref.as_str_or_null()?),
                            Type::Null => None,
                        };
                        Ok(KeyValueTypeEntry {
                            column_type: decl.column_type,
                            key: key.to_string(),
                            value,
                        })
                    }
                }
            })
            .into_iter()
            .collect::<Result<Vec<KeyValueTypeEntry>>>();
        Ok(KeyValueCollection { collection: items? })
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

    fn make_temp_inventory_table(&self, declarations: &SchemaCollection) -> String {
        let mut query = if declarations.collection.is_empty() {
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

        let count = declarations.collection.iter().count();
        let mut i = 0;
        declarations
            .collection
            .iter()
            .map(|e| self.make_row_statement(e))
            .for_each(|e| {
                i += 1;
                query.push_str(e.as_str());
                if i != count {
                    query.push(',');
                }
            });

        if !declarations.collection.is_empty() {
            query.push_str(");");
        }

        return query;
    }

    fn alter_inventory_table(
        &mut self,
        new_schema: &SchemaCollection,
        old_schema: &SchemaCollection,
        action_no: &SchemaActionNo,
        user: &DBUser,
    ) -> Result<String> {
        let old_schema_str = serde_json::to_string(&old_schema.collection)?;
        let new_schema_str = serde_json::to_string(&new_schema.collection)?;
        let create_inventory_table = self.make_temp_inventory_table(&new_schema);
        let copy_table = format!(
            "INSERT INTO invman_temp_inventory({cols}) SELECT {cols} FROM invman_inventory",
            cols = match action_no {
                SchemaActionNo::Alter => old_schema.sql_names(),
                SchemaActionNo::Remove => new_schema.sql_names(),
            }
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
                        SchemaCollection::new(serde_json::from_str(config.value.as_str()).unwrap());
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
        if let Some(idx) = config.inventory_schema_declaration.contains(&decl) {
            let mut schema_declaration = config.inventory_schema_declaration.collection.clone();
            schema_declaration.remove(idx);
            config.inventory_schema_declaration.collection = schema_declaration;
        }
        config.inventory_schema_declaration.collection.push(decl);
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
            .collection
            .iter()
            .position(|e| e.name == name);
        if !id.is_some() {
            bail!("The name attribute provided did not match any schema column definition");
        }
        let id = id.unwrap();
        config.inventory_schema_declaration.collection.remove(id);
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
        params: &KeyValueCollection,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String> {
        let values = params.sql_values();
        let sql = format!(
            "INSERT INTO invman_inventory ({}) VALUES ({})",
            params.sql_names(),
            vec!["?"; values.iter().count()].join(",")
        );
        let select_item_sql = format!(
            "SELECT id,created_at,updated_at,deleted_at,{} FROM invman_inventory WHERE id=?1",
            config.inventory_schema_declaration.sql_names(),
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
                Ok(row
                    .to_typed_key_value(&config.inventory_schema_declaration)
                    .with_context(|| {
                        format!("Failed to convert row into typed key value representation")
                    }))
            })??
            .to_json();
        tx.execute(
            "INSERT INTO invman_inventory_tx (dispatcher, schema_id, inventory_id, action_no, from_val, to_val) VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![user.id, latest_schema.id, latest_item.id, DBOpNo::Add as u32, json]
        )?;
        tx.execute("INSERT INTO invman_event_tx (action_no, dispatcher, target) VALUES (?1, ?2, (LAST_INSERT_ROWID()))", params![EventActionNo::InventoryAdd as u32, user.id])?;
        tx.commit()?;
        return Ok("Entity was successfully added to inventory".into());
    }

    fn inventory_list(
        &self,
        props: &InventoryListProps,
        config: &AppConfig,
    ) -> Result<Vec<KeyValueCollection>> {
        let mut sql = format!(
            "SELECT id,created_at,updated_at,deleted_at,{} FROM invman_inventory",
            config.inventory_schema_declaration.sql_names()
        );
        match props.raw {
            Some(raw) => {
                sql.push(' ');
                sql.push_str(raw);
            }
            None => {
                if props.limit > 0 {
                    sql.push_str(" LIMIT ");
                    sql.push_str(props.limit.to_string().as_str());
                }
            }
        }
        let mut stmt = self.db.prepare(&sql)?;
        let entries = stmt.query_map(params_from_iter(props.params), |row| {
            Ok(row
                .to_typed_key_value(&config.inventory_schema_declaration)
                .with_context(|| {
                    format!("Failed to convert SQLite result into JSON representation")
                })
                .unwrap())
        })?;
        return Ok(entries.map(|e| e.unwrap()).collect());
    }

    fn inventory_edit(
        &mut self,
        identifier: &String,
        params: &KeyValueCollection,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String> {
        let sql = format!(
            "SELECT {} FROM invman_inventory WHERE id=?1",
            config.inventory_schema_declaration.sql_names(),
        );
        let update_sql = format!(
            "UPDATE invman_inventory SET {} WHERE id=?1",
            params.sql_prepare_update_fields(1)
        );
        let mut sql_params = params.sql_values();
        let mut values = vec![Some(identifier.clone())];
        values.append(&mut sql_params);
        let tx = self.db.transaction()?;
        let before_item = tx.query_row(sql.as_str(), params![identifier], |row| {
            Ok(row
                .to_typed_key_value(&config.inventory_schema_declaration)
                .unwrap())
        })?;
        tx.execute(&update_sql, params_from_iter(values.iter()))?;
        let after_item = tx.query_row(sql.as_str(), params![identifier], |row| {
            Ok(row
                .to_typed_key_value(&config.inventory_schema_declaration)
                .unwrap())
        })?;
        let latest_schema = tx.query_row(
            "SELECT MAX(id) FROM invman_inventory_schema_tx",
            (),
            |row| Ok(IdEntry { id: row.get(0)? }),
        )?;
        tx.execute(
            "INSERT INTO invman_inventory_tx (dispatcher, schema_id, inventory_id, action_no, from_val, to_val) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user.id, latest_schema.id, before_item.get_id()?, DBOpNo::Edit as u32, before_item.to_json(), after_item.to_json()]
        )?;
        tx.execute("INSERT INTO invman_event_tx (action_no, dispatcher, target) VALUES (?1, ?2, (LAST_INSERT_ROWID()))", params![EventActionNo::InventoryEdit as u32, user.id])?;
        tx.commit()?;
        Ok("Entity was successfully edited".into())
    }

    fn inventory_remove(
        &mut self,
        identifier: &String,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String> {
        let sql = format!(
            "SELECT {} FROM invman_inventory WHERE id=?1",
            config.inventory_schema_declaration.sql_names(),
        );
        let tx = self.db.transaction()?;
        let before_item = tx.query_row(sql.as_str(), params![identifier], |row| {
            Ok(row
                .to_typed_key_value(&config.inventory_schema_declaration)
                .unwrap())
        })?;
        tx.execute(
            "UPDATE invman_inventory SET deleted_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=?1 AND deleted_at IS NULL",
            params![identifier],
        )?;
        let after_item = tx.query_row(sql.as_str(), params![identifier], |row| {
            Ok(row
                .to_typed_key_value(&config.inventory_schema_declaration)
                .unwrap())
        })?;
        let latest_schema = tx.query_row(
            "SELECT MAX(id) FROM invman_inventory_schema_tx",
            (),
            |row| Ok(IdEntry { id: row.get(0)? }),
        )?;
        tx.execute(
            "INSERT INTO invman_inventory_tx (dispatcher, schema_id, inventory_id, action_no, from_val, to_val) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user.id, latest_schema.id, before_item.get_id()?, DBOpNo::Delete as u32, before_item.to_json(), after_item.to_json()]
        )?;
        tx.execute("INSERT INTO invman_event_tx (action_no, dispatcher, target) VALUES (?1, ?2, (LAST_INSERT_ROWID()))", params![EventActionNo::InventoryRemove as u32, user.id])?;
        tx.commit()?;
        Ok("Entity was successfully removed".into())
    }
}
