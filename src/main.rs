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
use clap::{Args, Parser, Subcommand, ValueEnum};
use invman::{
    common::args::{
        ColumnType, CommandContext, InventoryAddArgs, InventoryEditArgs, InventoryListArgs,
        InventoryRemoveArgs, InventorySchemaAlterArgs, InventorySchemaListArgs,
        InventorySchemaRemoveArgs, OutputType, UserArgs, UserEditArgs,
    },
    database::{InvManConnection, InvManDBPool},
};

#[derive(Parser)]
#[command(name = "invman")]
#[command(bin_name = "invman")]
#[command(author, version, about, long_about = None)]
struct InventoryManagerCli {
    #[command(subcommand)]
    /// Manage user account's in your system
    command: InventoryManagerCliSub,

    /// Username:Password syntax used for secured access
    #[arg(short, long)]
    auth: Option<String>,

    #[arg(short, long, value_enum)]
    output: Option<OutputTypeCli>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, ValueEnum, Ord)]
pub enum ColumnTypeCli {
    #[default]
    TEXT,
    VARCHAR,
    INT,
    REAL,
    BOOL,
}

impl ColumnTypeCli {
    fn to_lib(self) -> ColumnType {
        return match self {
            ColumnTypeCli::BOOL => ColumnType::BOOL,
            ColumnTypeCli::INT => ColumnType::INT,
            ColumnTypeCli::REAL => ColumnType::REAL,
            ColumnTypeCli::TEXT => ColumnType::TEXT,
            ColumnTypeCli::VARCHAR => ColumnType::VARCHAR,
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputTypeCli {
    Plain,
    Json,
}

impl OutputTypeCli {
    fn to_lib(&self) -> OutputType {
        return match self {
            OutputTypeCli::Json => OutputType::Json,
            OutputTypeCli::Plain => OutputType::Plain,
        };
    }
}

#[derive(Args, Debug)]
pub struct InventoryRemoveCliArgs {
    #[arg(short, long)]
    /// The identifier used to target a specific entity
    identifier: String,
}

impl InventoryRemoveCliArgs {
    fn to_lib(&self) -> InventoryRemoveArgs {
        return InventoryRemoveArgs {
            identifier: self.identifier.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct InventoryEditCliArgs {
    #[arg(short, long)]
    /// The identifier used to target a specific entity
    identifier: String,

    #[arg(short, long)]
    /// Enter your parameters according to your specified schema in a name=value way
    set: Vec<String>,
}

impl InventoryEditCliArgs {
    fn to_lib(&self) -> InventoryEditArgs {
        return InventoryEditArgs {
            identifier: self.identifier.clone(),
            set: self.set.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct InventoryAddCliArgs {
    /// Enter your parameters according to your specified schema in a name=value way
    params: Vec<String>,
}

impl InventoryAddCliArgs {
    fn to_lib(&self) -> InventoryAddArgs {
        return InventoryAddArgs {
            params: self.params.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct InventorySchemaRemoveCliArgs {
    /// Name of the schema column
    name: String,
}

impl InventorySchemaRemoveCliArgs {
    fn to_lib(&self) -> InventorySchemaRemoveArgs {
        return InventorySchemaRemoveArgs {
            name: self.name.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct UserRegisterCliArgs {
    /// Name of the user
    name: String,
    /// Password of the user
    password: String,
}

impl UserRegisterCliArgs {
    fn to_lib(&self) -> UserArgs {
        return UserArgs {
            name: self.name.clone(),
            password: self.password.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct UserEditCliArgs {
    /// Options to change into in option1=value1 option2=value2 syntax
    options: Vec<String>,
}

impl UserEditCliArgs {
    fn to_lib(&self) -> UserEditArgs {
        return UserEditArgs {
            options: self.options.clone(),
        };
    }
}

#[derive(Args, Debug)]
pub struct InventorySchemaAlterCliArgs {
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
    column_type: ColumnTypeCli,

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

impl InventorySchemaAlterCliArgs {
    fn to_lib(&self) -> InventorySchemaAlterArgs {
        return InventorySchemaAlterArgs {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            unique: self.unique,
            max_length: self.max_length,
            min_length: self.min_length,
            max: self.max,
            min: self.min,
            nullable: self.nullable,
            column_type: self.column_type.to_lib(),
            default: self.default.clone(),
            hint: self.hint.clone(),
            layout: self.layout.clone(),
        };
    }
}

#[derive(Subcommand, Debug)]
pub enum InventoryCommands {
    /// Add an entity to your inventory
    Add(InventoryAddCliArgs),

    /// List all entities stored in your inventory
    List(InventoryListCliArgs),

    #[command(subcommand)]
    /// Change the schema in which your entities are stored
    Schema(InventorySchemaCommands),

    /// Edit an existing entity in your inventory
    Edit(InventoryEditCliArgs),

    /// Remove an entity from your inventory
    Remove(InventoryRemoveCliArgs),
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {}

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// Register a new user
    Register(UserRegisterCliArgs),
    Edit(UserEditCliArgs),
}

#[derive(Args, Debug)]
pub struct InventoryListCliArgs {
    #[arg(short, long)]
    /// Limit the amount of entities to be returned
    limit: Option<i32>,

    #[arg(short, long)]
    /// How the returned rows should be sorted
    sort: Vec<String>,

    #[arg(short, long)]
    /// Executes the query directly onto the database. BEWARE that parameters must be passed seperatly with --params flags, otherwise your system will be vulnerable to SQL injection attacks
    raw: Option<String>,

    #[arg(short, long)]
    /// Parameters that are passed with the raw SQL string
    params: Vec<String>,

    #[arg(short, long)]
    /// How the returned rows should be sorted
    condition: Vec<String>,
}

impl InventoryListCliArgs {
    fn to_lib(&self) -> InventoryListArgs {
        return InventoryListArgs {
            limit: self.limit,
            sort: self.sort.clone(),
            raw: self.raw.clone(),
            params: self.params.clone(),
            condition: self.condition.clone(),
        };
    }
}

#[derive(Args, Debug)]
struct InventorySchemaListCliArgs;

impl InventorySchemaListCliArgs {
    fn to_lib(&self) -> InventorySchemaListArgs {
        return InventorySchemaListArgs;
    }
}

#[derive(Subcommand, Debug)]
pub enum InventorySchemaCommands {
    /// Add or edit a schema column
    Alter(InventorySchemaAlterCliArgs),

    /// Remove a schema column
    Remove(InventorySchemaRemoveCliArgs),

    /// List your schema columns
    List(InventorySchemaListCliArgs),
}

#[derive(Subcommand)]
enum InventoryManagerCliSub {
    #[command(subcommand)]
    /// Manage user account's in your system
    User(UserCommands),

    #[command(subcommand)]
    /// Read and modify config
    Config(ConfigCommands),

    #[command(subcommand)]
    /// Manage your articles
    Inventory(InventoryCommands),
}

fn main() {
    use InventoryManagerCliSub::{Config, Inventory, User};

    let cli = InventoryManagerCli::parse();
    let mut conn = InvManConnection::sqlite().unwrap();
    let pool: &mut dyn InvManDBPool = &mut conn;
    let mut config = pool.get_config();
    let mut ctx = CommandContext {
        db: pool,
        auth: cli.auth,
        config: &mut config,
        output: cli.output.unwrap_or(OutputTypeCli::Json).to_lib(),
    };

    let response = match &cli.command {
        User(args) => match args {
            UserCommands::Register(args) => args.to_lib().register(&mut ctx),
            UserCommands::Edit(args) => args.to_lib().edit(&ctx),
        },
        Config(args) => match args {
            _ => Ok("not a command".into()),
        },
        Inventory(args) => match args {
            InventoryCommands::Add(args) => args.to_lib().add(&mut ctx),
            InventoryCommands::List(args) => args.to_lib().list(&ctx),
            InventoryCommands::Edit(args) => args.to_lib().edit(&mut ctx),
            InventoryCommands::Remove(args) => args.to_lib().remove(&mut ctx),
            InventoryCommands::Schema(args) => match args {
                InventorySchemaCommands::Alter(args) => args.to_lib().alter(&mut ctx),
                InventorySchemaCommands::List(args) => args.to_lib().schema_list(&mut ctx),
                InventorySchemaCommands::Remove(args) => args.to_lib().remove(&mut ctx),
            },
        },
    };

    match response {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e.to_string()),
    }
}
