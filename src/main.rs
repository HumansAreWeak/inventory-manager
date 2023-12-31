use crate::{
    commands::{CommandContext, InventorySchemaCommands},
    database::{InvManConnection, InvManDBPool},
};
use clap::{Parser, Subcommand, ValueEnum};
use commands::InventoryCommands;

mod commands;
mod database;
mod utils;

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
    output: Option<OutputType>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputType {
    Plain,
}

#[derive(Subcommand)]
enum InventoryManagerCliSub {
    #[command(subcommand)]
    /// Manage user account's in your system
    User(commands::UserCommands),

    #[command(subcommand)]
    /// Read and modify config
    Config(commands::ConfigCommands),

    #[command(subcommand)]
    /// Manage your articles
    Inventory(InventoryCommands),
}

fn main() {
    use commands::UserCommands;
    use InventoryManagerCliSub::{Config, Inventory, User};

    let cli = InventoryManagerCli::parse();
    let mut conn = InvManConnection::sqlite().unwrap();
    let pool: &mut dyn InvManDBPool = &mut conn;
    let mut config = pool.get_config();
    let mut ctx = CommandContext {
        db: pool,
        auth: cli.auth,
        config: &mut config,
        output: cli.output.unwrap_or(OutputType::Plain),
    };

    let response = match &cli.command {
        User(args) => match args {
            UserCommands::Register(args) => args.register(&mut ctx),
            UserCommands::Edit(args) => args.edit(&ctx),
        },
        Config(args) => match args {
            _ => Ok("not a command".into()),
        },
        Inventory(args) => match args {
            InventoryCommands::Add(args) => args.add(&mut ctx),
            InventoryCommands::List(args) => args.list(&ctx),
            InventoryCommands::Edit(args) => args.edit(&mut ctx),
            InventoryCommands::Remove(args) => args.remove(&mut ctx),
            InventoryCommands::Schema(args) => match args {
                InventorySchemaCommands::Alter(args) => args.alter(&mut ctx),
                InventorySchemaCommands::List(args) => args.schema_list(&mut ctx),
                InventorySchemaCommands::Remove(args) => args.remove(&mut ctx),
            },
        },
    };

    match response {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e.to_string()),
    }
}
