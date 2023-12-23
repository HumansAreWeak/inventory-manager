mod sqlite;

pub(crate) use self::sqlite::InvManSqlite;
use crate::commands::{ColumnType, DBUser, InventoryListProps, SchemaDeclaration};
use anyhow::{bail, Result};

#[derive(Debug, Copy, Clone)]
enum SchemaActionNo {
    Alter = 1,
    Remove = 2,
}

#[derive(Debug, Copy, Clone)]
enum DBOpNo {
    Add = 1,
}

#[derive(Debug, Copy, Clone)]
enum EventActionNo {
    UserRegister = 100,

    InventoryAdd = 200,
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
        params: &Vec<(String, String)>,
        config: &AppConfig,
        user: &DBUser,
    ) -> Result<String>;

    fn inventory_list(
        &self,
        props: &InventoryListProps,
        config: &AppConfig,
    ) -> Result<Vec<JsonEntry>>;
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
    pub inventory_schema_declaration: Vec<SchemaDeclaration>,
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
pub struct JsonEntry {
    json: Vec<KeyValueEntry>,
}

#[derive(Debug)]
pub struct KeyValueEntry {
    key: String,
    value: String,
}

impl JsonEntry {
    pub fn to_json(&self) -> String {
        let first_element = self
            .json
            .first()
            .expect("The vector of row elements is empty");
        let mut json = format!("{{\"{}\":{}", first_element.key, first_element.value);
        self.json.iter().skip(1).for_each(|e| {
            json.push_str(",\"");
            json.push_str(&e.key);
            json.push_str("\":");
            json.push_str(&e.value);
        });
        json.push('}');
        return json;
    }
}
