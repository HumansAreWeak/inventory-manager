use anyhow::{anyhow, bail, Result};
use core::fmt;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        AppConfig, DBUser, InvManDBPool, KeyValueCollection, KeyValueTypeEntry, SchemaCollection,
    },
    utils::InvManSerialization,
};

pub trait InvManNotationHelper {
    fn to_typed_key_value_entry(
        &self,
        declarations: &SchemaCollection,
    ) -> Result<KeyValueTypeEntry>;
}

pub trait InvManNotationHelperVec {
    fn to_key_value_collection(
        &self,
        declarations: &SchemaCollection,
    ) -> Result<KeyValueCollection>;
}

impl InvManNotationHelperVec for Vec<String> {
    fn to_key_value_collection(
        &self,
        declarations: &SchemaCollection,
    ) -> Result<KeyValueCollection> {
        return Ok(KeyValueCollection {
            collection: self
                .iter()
                .map(|e| e.to_typed_key_value_entry(declarations))
                .into_iter()
                .collect::<Result<Vec<_>>>()?,
        });
    }
}

impl InvManNotationHelper for String {
    fn to_typed_key_value_entry(
        &self,
        declarations: &SchemaCollection,
    ) -> Result<KeyValueTypeEntry> {
        return match self.split_once("=") {
            None => Err(anyhow!("Could not split parsed parameter")),
            Some(val) => {
                if let Some(decl) = declarations.collection.iter().find(|e| e.name == val.0) {
                    Ok(KeyValueTypeEntry::new(
                        val.0.to_string(),
                        Some(val.1.to_string()),
                        decl.column_type,
                    ))
                } else {
                    Err(anyhow!("Could not find '{}' in table schema", val.0))
                }
            }
        };
    }
}

impl InvManSerialization for Vec<KeyValueCollection> {
    fn to_json(&self) -> String {
        let mut jsons = self
            .iter()
            .map(|e| e.to_json())
            .collect::<Vec<String>>()
            .join(",");
        jsons.insert(0, '[');
        jsons.push(']');
        return jsons;
    }
}

pub struct CommandContext<'a> {
    pub db: &'a mut dyn InvManDBPool,
    pub config: &'a mut AppConfig,
    pub auth: Option<String>,
    pub output: OutputType,
}

impl<'a> CommandContext<'a> {
    fn authenticate(&self) -> Result<DBUser> {
        let auth = self.auth.clone().unwrap_or("".into());
        if auth.is_empty() {
            bail!("User authentication failure (No auth token was provided)");
        }
        let mut user = DBUser::default();

        return match auth.split_once(":") {
            Some(s) => match self.db.user_auth(s.0, s.1, &mut user) {
                Ok(_) => Ok(user),
                Err(e) => bail!("User authentication failure ({})", e.to_string()),
            },
            None => bail!("User authentication failure (Failed to split the token)"),
        };
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum ColumnType {
    #[default]
    TEXT,
    VARCHAR,
    INT,
    REAL,
    BOOL,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputType {
    Plain,
    Json,
}

pub struct InventoryRemoveArgs {
    pub identifier: String,
}

impl InventoryRemoveArgs {
    pub fn remove(&self, ctx: &mut CommandContext) -> Result<String> {
        let user = ctx.authenticate()?;
        ctx.db
            .inventory_remove(&self.identifier, &ctx.config, &user)
    }
}

pub struct InventoryEditArgs {
    pub identifier: String,
    pub set: Vec<String>,
}

impl InventoryEditArgs {
    pub fn edit(&self, ctx: &mut CommandContext) -> Result<String> {
        let user = ctx.authenticate()?;
        ctx.db.inventory_edit(
            &self.identifier,
            &self
                .set
                .to_key_value_collection(&ctx.config.inventory_schema_declaration)?,
            &ctx.config,
            &user,
        )
    }
}

pub struct InventoryListArgs {
    pub limit: Option<i32>,
    pub sort: Vec<String>,
    pub raw: Option<String>,
    pub params: Vec<String>,
    pub condition: Vec<String>,
}

pub struct InventoryListProps<'a> {
    pub limit: i32,
    pub raw: &'a Option<String>,
    pub params: &'a Vec<String>,
}

impl InventoryListArgs {
    pub fn list(&self, ctx: &CommandContext) -> Result<String> {
        let _ = ctx.authenticate()?;
        let props = InventoryListProps {
            limit: self.limit.unwrap_or(-1),
            raw: &self.raw,
            params: &self.params,
        };
        let data = ctx.db.inventory_list(&props, &ctx.config)?;
        return Ok(data.to_json());
    }
}

pub struct InventorySchemaListArgs;

impl InventorySchemaListArgs {
    pub fn schema_list(&self, ctx: &CommandContext) -> Result<String> {
        let user = ctx.authenticate()?;
        if !user.can_read_table("config") {
            bail!("Cannot read the config table");
        }
        return Ok(ctx.config.inventory_schema_declaration.to_json());
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SchemaDeclaration {
    pub name: String,
    pub display_name: String,
    pub unique: bool,
    pub max_length: u32,
    pub min_length: u32,
    pub max: u32,
    pub min: u32,
    pub nullable: bool,
    pub column_type: ColumnType,
    pub default: String,
    pub hint: String,
    pub layout: String,
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColumnType::BOOL => write!(f, "bool"),
            ColumnType::INT => write!(f, "int"),
            ColumnType::REAL => write!(f, "real"),
            ColumnType::TEXT => write!(f, "text"),
            ColumnType::VARCHAR => write!(f, "varchar"),
        }
    }
}

impl SchemaDeclaration {
    fn new(args: &InventorySchemaAlterArgs) -> Result<SchemaDeclaration> {
        let name = args.name.clone();
        let default = args.default.clone();
        let hint = args.hint.clone();
        let layout = args.layout.clone();
        let display_name = match args.display_name.clone() {
            Some(name) => name,
            None => {
                let name = name.replace("-", " ").replace("_", " ");
                let mut chars = name.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first
                        .to_uppercase()
                        .chain(chars.map(|c| c.to_ascii_lowercase()))
                        .collect(),
                }
            }
        };

        let decl = SchemaDeclaration {
            name,
            display_name,
            unique: args.unique,
            max_length: args.max_length.unwrap_or(0),
            min_length: args.min_length.unwrap_or(0),
            max: args.max.unwrap_or(0),
            min: args.min.unwrap_or(0),
            nullable: args.nullable.unwrap_or(false),
            column_type: args.column_type,
            default: default.unwrap_or("NULL".into()),
            hint: hint.unwrap_or("".into()),
            layout: layout.unwrap_or("".into()),
        };

        if decl.min_length > decl.max_length {
            bail!("Schema min-length parameter cannot be larger than max-length!");
        }

        if decl.min > decl.max {
            bail!("Schema min parameter cannot be larger than max!");
        }

        if decl.column_type == ColumnType::VARCHAR && decl.max_length == 0 {
            bail!("Schema cannot have column type varchar with max-length being 0!");
        }

        if decl.default != "NULL" {
            if decl.max_length > 0 && decl.default.len() > usize::try_from(decl.max_length)? {
                bail!("Schema default value cannot be longer than max-length!");
            }
            if decl.min_length > 0 && decl.default.len() < usize::try_from(decl.min_length)? {
                bail!("Schema default value cannot be shorter than min-length!");
            }
        }

        return Ok(decl);
    }

    pub fn is_equal(&self, other: &SchemaDeclaration) -> bool {
        return self.name == other.name;
    }

    pub fn to_json(&self) -> String {
        return format!("{{\"name\":\"{}\",\"display_name\":\"{}\",\"unique\":{},\"max_length\":{},\"min_length\":{},\"max\":{},\"min\":{},\"nullable\":{},\"column_type\":\"{}\",\"default\":\"{}\",\"hint\":\"{}\",\"layout\":\"{}\"}}",
                       self.name, self.display_name, self.unique, self.max_length, self.min_length, self.max, self.min, self.nullable, self.column_type, self.default, self.hint, self.layout);
    }
}

pub struct UserArgs {
    pub name: String,
    pub password: String,
}

impl UserArgs {
    pub fn register(&self, param: &mut CommandContext) -> Result<String> {
        if !param.config.allow_registration {
            bail!("User registration failed (Registration is disabled by inventory administrator)");
        }

        return match param
            .db
            .user_register(self.name.as_str(), self.password.as_str())
        {
            Ok(s) => Ok(s),
            Err(e) => bail!("User registration failed ({})", e.to_string()),
        };
    }
}

pub struct UserEditArgs {
    pub options: Vec<String>,
}

impl UserEditArgs {
    pub fn edit(&self, ctx: &CommandContext) -> Result<String> {
        let _user = ctx.authenticate()?;
        return Ok("".into());
    }
}

pub struct InventorySchemaAlterArgs {
    pub name: String,
    pub display_name: Option<String>,
    pub unique: bool,
    pub max_length: Option<u32>,
    pub min_length: Option<u32>,
    pub max: Option<u32>,
    pub min: Option<u32>,
    pub nullable: Option<bool>,
    pub column_type: ColumnType,
    pub default: Option<String>,
    pub hint: Option<String>,
    pub layout: Option<String>,
}

impl InventorySchemaAlterArgs {
    pub fn alter(&self, ctx: &mut CommandContext) -> Result<String> {
        let mut user = ctx.authenticate()?;
        if !user.can_write_table("config") {
            bail!("Cannot write to config table");
        }
        let decl = SchemaDeclaration::new(self)?;
        return ctx.db.schema_alter(ctx.config, decl, &mut user);
    }
}

pub struct InventorySchemaRemoveArgs {
    pub name: String,
}

impl InventorySchemaRemoveArgs {
    pub fn remove(&self, ctx: &mut CommandContext) -> Result<String> {
        let user = ctx.authenticate()?;
        return ctx
            .db
            .schema_remove(&mut ctx.config, self.name.as_str(), &user);
    }
}

pub struct InventoryAddArgs {
    pub params: Vec<String>,
}

impl InventoryAddArgs {
    pub fn add(&self, ctx: &mut CommandContext) -> Result<String> {
        let user = ctx.authenticate()?;
        let entries: KeyValueCollection = self
            .params
            .iter()
            .map(|e| e.to_typed_key_value_entry(&ctx.config.inventory_schema_declaration))
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .into();
        if !user.can_write_collection("inventory", &entries) {
            bail!("Cannot write arguments to inventory");
        }
        return ctx.db.inventory_add(&entries, &ctx.config, &user);
    }
}
