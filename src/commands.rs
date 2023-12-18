use crate::database::{AppConfig, InvManDBPool};
use anyhow::{bail, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

pub struct CommandParams<'a> {
    pub db: &'a mut dyn InvManDBPool,
    pub config: &'a mut AppConfig,
    pub auth: Option<String>,
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
pub enum SchemeCommands {
    Alter(SchemeAlterArgs),
}

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
pub struct SchemeAlterArgs {
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

impl SchemaDeclaration {
    fn new(args: &SchemeAlterArgs) -> Result<SchemaDeclaration> {
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

        return Ok(decl);
    }

    pub fn is_equal(&self, other: &SchemaDeclaration) -> bool {
        return self.name == other.name;
    }
}

fn auth_valid(auth: Option<String>, db: &dyn InvManDBPool, user: &mut DBUser) -> Result<bool> {
    let auth = auth.unwrap_or("".into());
    if auth.is_empty() {
        bail!("User authentication failure (No auth token was provided)");
    }

    return match auth.split_once(":") {
        Some(s) => match db.user_auth(s.0, s.1, user) {
            Ok(_) => Ok(true),
            Err(e) => bail!("User authentication failure ({})", e.to_string()),
        },
        None => bail!("User authentication failure (Failed to split the token)"),
    };
}

impl UserArgs {
    pub fn register(&self, param: &CommandParams) -> String {
        if !param.config.allow_registration {
            return "User registration failed (Registration is disabled by inventory administrator)"
                .into();
        }

        return match param
            .db
            .user_register(self.name.as_str(), self.password.as_str())
        {
            Ok(s) => s,
            Err(e) => format!("User registration failed ({})", e.to_string()),
        };
    }
}

impl UserEditArgs {
    pub fn edit(&self, param: &CommandParams) -> String {
        let mut user = DBUser::new();
        return match auth_valid(param.auth.clone(), param.db, &mut user) {
            Ok(_) => "Authentication succedded".into(),
            Err(e) => e.to_string(),
        };
    }
}

impl SchemeAlterArgs {
    pub fn alter(&self, param: &mut CommandParams) -> String {
        let mut user = DBUser::new();
        return match auth_valid(param.auth.clone(), param.db, &mut user) {
            Ok(_) => match SchemaDeclaration::new(self) {
                Ok(decl) => match param.db.schema_alter(param.config, decl) {
                    Ok(_) => "Schema has been altered".into(),
                    Err(e) => e.to_string(),
                },
                Err(e) => e.to_string(),
            },
            Err(e) => e.to_string(),
        };
    }
}
