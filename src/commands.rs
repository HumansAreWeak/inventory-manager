use core::fmt;

use crate::{
    database::{AppConfig, InvManDBPool},
    utils::{InvManSerialization, SchemaDeclarationVerify},
    OutputType,
};
use anyhow::{bail, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::{de::IntoDeserializer, Deserialize, Serialize};

pub struct CommandContext<'a> {
    pub db: &'a mut dyn InvManDBPool,
    pub config: &'a mut AppConfig,
    pub auth: Option<String>,
    pub output: OutputType,
}

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// Register a new user
    Register(UserArgs),
    Edit(UserEditArgs),
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize, Serialize,
)]
pub enum ColumnType {
    #[default]
    TEXT,
    VARCHAR,
    INT,
    REAL,
    BOOL,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {}

#[derive(Subcommand, Debug)]
pub enum InventoryCommands {
    /// Add an entity to your inventory
    Add(InventoryAddArgs),

    /// List all entities stored in your inventory
    List(InventoryListArgs),

    #[command(subcommand)]
    /// Change the schema in which your entities are stored
    Schema(InventorySchemaCommands),
}

#[derive(Subcommand, Debug)]
pub enum InventorySchemaCommands {
    /// Add or edit a schema column
    Alter(SchemaAlterArgs),

    /// Remove a schema column
    Remove(SchemaRemoveArgs),

    /// List your schema columns
    List(NoParams),
}

#[derive(Args, Debug)]
pub struct InventoryListArgs {
    #[arg(short, long)]
    /// Limit the amount of entities to be returned
    limit: Option<i32>,

    #[arg(short, long)]
    /// How the returned rows should be sorted
    sort: Vec<String>,

    #[arg(short, long)]
    /// How the returned rows should be sorted
    condition: Vec<String>,
}

pub struct InventoryListProps {
    pub limit: i32,
}

impl InventoryListArgs {
    pub fn list(&self, context: &CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        let props = InventoryListProps {
            limit: self.limit.unwrap_or(-1),
        };
        let data = context.db.inventory_list(&props, &context.config)?;
        return Ok(data.to_json());
    }
}

#[derive(Args, Debug)]
pub struct InventoryAddArgs {
    /// Enter your parameters according to your specified schema in a name=value way
    params: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SchemaRemoveArgs {
    /// Name of the schema column
    name: String,
}

#[derive(Args, Debug)]
pub struct NoParams;

#[derive(Args, Debug)]
pub struct UserArgs {
    /// Name of the user
    name: String,
    /// Password of the user
    password: String,
}

#[derive(Args, Debug)]
pub struct UserEditArgs {
    /// Options to change into in option1=value1 option2=value2 syntax
    options: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SchemaAlterArgs {
    #[arg(short, long)]
    /// Specifies the name as tag for your column. Only following values are allowed [a-z\-\_] (Letters from a-z (lowercase), - (dash) and _ (underscore))
    name: String,

    #[arg(long)]
    /// Explicitly defines its display name for printing (Default: Parsed name value)
    display_name: Option<String>,

    #[arg(short, long)]
    /// If set to true then one and only one kind of its value can be found in the system (Default: false)
    unique: bool,

    #[arg(short, long)]
    /// Specifies the maximum length of this parameter (only applies to strings) (Default: 0)
    max_length: Option<u32>,

    #[arg(long)]
    /// Specifies the minimum length of this parameter (only applies to strings) (Default: 0)
    min_length: Option<u32>,

    #[arg(long)]
    /// Specifies the maximum value of this parameter (only applies to INT and REAL) (Default: 0)
    max: Option<u32>,

    #[arg(long)]
    /// Specifies the minimum value of this parameter (only applies to INT and REAL) (Default: 0)
    min: Option<u32>,

    #[arg(long)]
    /// Allows for value NULL to be inserted if no value is provided and no default is specified (Default: false)
    nullable: Option<bool>,

    #[arg(short, long, value_enum)]
    /// Define the type of data stored in the column, choose between
    ///     - TEXT for texts of arbritrary length that cannot be searched
    ///     - VARCHAR that must have max_length specified for texts that can be searched
    ///     - INT for whole numbers
    ///     - REAL for real numbers
    ///     - BOOL for boolean value, i.e. only values of true and false
    column_type: ColumnType,

    #[arg(short, long)]
    /// The default value that will be used if no value is provided (Default: NULL)
    ///     TIPS:
    ///     - Use CURRENT_TIMESTAMP to automatically use the current Datetime as value
    default: Option<String>,

    #[arg(long)]
    /// Hint for external applications of how to display this column (Default: Empty String)
    hint: Option<String>,

    #[arg(long)]
    /// For external applications as additional layout information (Default: Empty String)
    layout: Option<String>,
}

pub struct DBUser {
    pub id: u32,
}

impl DBUser {
    fn new() -> DBUser {
        return DBUser { id: 0 };
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
    fn new(args: &SchemaAlterArgs) -> Result<SchemaDeclaration> {
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

fn auth_valid(context: &CommandContext, user: &mut DBUser) -> Result<bool> {
    let auth = context.auth.clone().unwrap_or("".into());
    if auth.is_empty() {
        bail!("User authentication failure (No auth token was provided)");
    }

    return match auth.split_once(":") {
        Some(s) => match context.db.user_auth(s.0, s.1, user) {
            Ok(_) => Ok(true),
            Err(e) => bail!("User authentication failure ({})", e.to_string()),
        },
        None => bail!("User authentication failure (Failed to split the token)"),
    };
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

impl UserEditArgs {
    pub fn edit(&self, context: &CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        return Ok("".into());
    }
}

impl SchemaAlterArgs {
    pub fn alter(&self, context: &mut CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        let decl = SchemaDeclaration::new(self)?;
        return context.db.schema_alter(context.config, decl, &mut user);
    }
}

impl NoParams {
    pub fn schema_list(&self, context: &CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        return Ok(context.config.inventory_schema_declaration.to_json());
    }
}

impl SchemaRemoveArgs {
    pub fn remove(&self, context: &mut CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        return context
            .db
            .schema_remove(&mut context.config, self.name.as_str(), &user);
    }
}

impl InventoryAddArgs {
    pub fn add(&self, context: &mut CommandContext) -> Result<String> {
        let mut user = DBUser::new();
        auth_valid(context, &mut user)?;
        let all_exist = self.params.iter().all(|e| match e.split_once("=") {
            Some((name, _)) => context
                .config
                .inventory_schema_declaration
                .iter()
                .any(|e1| e1.name == name),
            None => false,
        });
        if !all_exist {
            bail!("One of the entered parameter did not match the schema");
        }
        let params: Vec<Result<(String, String)>> = self
            .params
            .iter()
            .map(|e| e.check_against_declaration(&context.config.inventory_schema_declaration))
            .collect();
        let errors = params.iter().filter(|e| e.is_err());
        if errors.clone().count() > 0 {
            let return_message: Vec<String> = errors
                .map(|e| {
                    e.as_ref()
                        .expect_err("Is Error flag is true but element is not of error type")
                        .to_string()
                })
                .collect();
            bail!("{}", return_message.join("\n"));
        }
        let params: Vec<(String, String)> = params
            .iter()
            .map(|e| e.as_ref().unwrap().to_owned())
            .collect();
        return context.db.inventory_add(&params, &context.config, &user);
    }
}
