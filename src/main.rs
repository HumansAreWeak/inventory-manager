use crate::{
    commands::{CommandParams, SchemeCommands},
    database::{InvManDBPool, InvManSqlite},
};
use clap::{Parser, Subcommand};

mod commands;
mod database;

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
}

#[derive(Subcommand)]
enum InventoryManagerCliSub {
    #[command(subcommand)]
    /// Manage user account's in your system
    User(commands::UserCommands),
    #[command(subcommand)]
    Config(commands::ConfigCommands),
    #[command(subcommand)]
    Scheme(commands::SchemeCommands),
}

fn main() {
    use commands::UserCommands;
    use InventoryManagerCliSub::{Config, Scheme, User};

    let cli = InventoryManagerCli::parse();
    let mut conn = InvManSqlite::new();
    let pool: &mut dyn InvManDBPool = &mut conn;
    let mut config = pool.get_config();
    let mut params = CommandParams {
        db: pool,
        auth: cli.auth,
        config: &mut config,
    };

    let response = match &cli.command {
        User(args) => match args {
            UserCommands::Register(args) => args.register(&params),
            UserCommands::Edit(args) => args.edit(&params),
        },
        Config(args) => match args {
            _ => "not a command".into(),
        },
        Scheme(args) => match args {
            SchemeCommands::Alter(args) => args.alter(&mut params),
            _ => "not a command".into(),
        },
    };
    println!("{}", response);
}
