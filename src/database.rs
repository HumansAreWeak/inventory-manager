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
mod sqlite;

pub(crate) use self::sqlite::InvManSqlite;
use crate::{
    commands::{ColumnType, DBUser, InventoryListProps, SchemaDeclaration},
    utils::InvManSerialization,
};
use anyhow::{bail, Result};

#[derive(Debug, Copy, Clone)]
enum SchemaActionNo {
    Alter = 1,
    Remove = 2,
}

#[derive(Debug, Copy, Clone)]
enum DBOpNo {
    Add = 1,
    Edit = 2,
    Delete = 3,
}

#[derive(Debug, Copy, Clone)]
enum EventActionNo {
    UserRegister = 100,

    InventoryAdd = 200,
    InventoryEdit = 201,
    InventoryRemove = 202,
}

pub trait InvManDBPool {
    fn get_config(&self) -> AppConfig;
    fn user_register(&mut self, username: &str, password: &str) -> Result<String>;
    fn user_auth(&self, username: &str, password: &str, user: &mut DBUser) -> Result<()>;

    fn schema_alter(
        &mut self,
        config: &mut AppConfig,
        decl: SchemaDeclaration,
        user: &DBUser,
    ) -> Result<String>;
    fn schema_remove(
        &mut self,
        config: &mut AppConfig,
        name: &str,
        user: &DBUser,
    ) -> Result<String>;

    fn inventory_add(
        &mut self,
        params: &KeyValueCollection,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String>;

    fn inventory_list(
        &self,
        props: &InventoryListProps,
        config: &AppConfig,
    ) -> Result<Vec<KeyValueCollection>>;

    fn inventory_edit(
        &mut self,
        identifier: &String,
        params: &KeyValueCollection,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String>;

    fn inventory_remove(
        &mut self,
        identifier: &String,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String>;
}

pub struct InvManConnection;

impl InvManConnection {
    pub fn sqlite() -> Result<InvManSqlite> {
        return InvManSqlite::new();
    }
}

#[derive(Default, Clone)]
pub struct AppConfig {
    pub allow_registration: bool,
    pub inventory_schema_declaration: SchemaCollection,
}

#[derive(Debug)]
struct Count {
    count: u32,
}

#[derive(Debug)]
struct IdPassword {
    id: u32,
    password: String,
}

#[derive(Debug)]
struct IdEntry {
    id: u32,
}

#[derive(Debug)]
struct Config {
    name: String,
    value: String,
}

#[derive(Debug)]
pub struct KeyValueCollection {
    pub collection: Vec<KeyValueTypeEntry>,
}

#[derive(Default, Debug, Clone)]
pub struct SchemaCollection {
    pub collection: Vec<SchemaDeclaration>,
}

impl SchemaCollection {
    pub fn new(collection: Vec<SchemaDeclaration>) -> SchemaCollection {
        return SchemaCollection { collection };
    }

    pub fn sql_names(&self) -> String {
        return if self.collection.iter().count() == 0 {
            "id,created_at,updated_at,deleted_at".into()
        } else {
            format!(
                "id,created_at,updated_at,deleted_at,{}",
                self.collection
                    .iter()
                    .map(|e| e.name.clone())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        };
    }

    pub fn to_json(&self) -> String {
        let mut json = self
            .collection
            .iter()
            .map(|e| e.to_json())
            .collect::<Vec<String>>()
            .join(",");
        json.insert(0, '[');
        json.push(']');
        return json;
    }

    pub fn contains(&self, declaration: &SchemaDeclaration) -> Option<usize> {
        return self.collection.iter().position(|d| d.is_equal(declaration));
    }
}

pub trait InvManToSql {
    // Returns the SQL names as plain string that are fetched
    fn sql_names(&self) -> String;

    // Returns all the values in the same order that the names are printed
    fn sql_values(&self) -> Vec<Option<String>>;
}

impl KeyValueCollection {
    fn new(collection: Vec<KeyValueTypeEntry>) -> KeyValueCollection {
        return KeyValueCollection { collection };
    }

    pub fn sql_prepare_update_fields(&self, idx_offset: usize) -> String {
        return self
            .collection
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}=?{}", e.key, (i + idx_offset + 1)))
            .collect::<Vec<String>>()
            .join(",");
    }

    pub fn get_id(&self) -> Result<String> {
        if let Some(val) = self.collection.iter().find(|e| e.key == "id") {
            if let Some(val) = val.value.clone() {
                return Ok(val);
            }
            bail!("Entry with key 'id' is not available");
        } else {
            bail!("No entry with key 'id' found in collection");
        }
    }
}

impl InvManToSql for KeyValueCollection {
    fn sql_names(&self) -> String {
        return self
            .collection
            .iter()
            .map(|e| e.key.clone())
            .collect::<Vec<String>>()
            .join(",");
    }

    fn sql_values(&self) -> Vec<Option<String>> {
        return self.collection.iter().map(|e| e.value.clone()).collect();
    }
}

impl Into<KeyValueCollection> for Vec<KeyValueTypeEntry> {
    fn into(self) -> KeyValueCollection {
        return KeyValueCollection::new(self);
    }
}

#[derive(Debug)]
pub struct KeyValueTypeEntry {
    key: String,
    value: Option<String>,
    column_type: ColumnType,
}

impl KeyValueTypeEntry {
    pub fn new(key: String, value: Option<String>, column_type: ColumnType) -> KeyValueTypeEntry {
        return KeyValueTypeEntry {
            key,
            value,
            column_type,
        };
    }

    fn to_json_notation(&self) -> String {
        return format!(
            "\"{}\":{}",
            self.key,
            match self.value.clone() {
                None => "null".into(),
                Some(val) => match self.column_type {
                    ColumnType::TEXT | ColumnType::VARCHAR => format!("\"{}\"", val),
                    ColumnType::BOOL =>
                        if val == "true" || val == "1" {
                            "true".into()
                        } else {
                            "false".into()
                        },
                    _ => val,
                },
            }
        );
    }
}

impl InvManSerialization for KeyValueCollection {
    fn to_json(&self) -> String {
        let first_element = self
            .collection
            .first()
            .expect("The vector of row elements is empty");
        let mut json = format!("{{{}", first_element.to_json_notation());
        self.collection.iter().skip(1).for_each(|e| {
            json.push(',');
            json.push_str(e.to_json_notation().as_str());
        });
        json.push('}');
        return json;
    }
}
