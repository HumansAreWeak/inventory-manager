use clap::{Args, Parser, Subcommand};
mod commands;
mod database;

#[derive(Parser)]
#[command(name = "invmanager")]
#[command(bin_name = "invmanager")]
#[command(author, version, about, long_about = None)]
enum InventoryManagerCli {
    #[command(subcommand)]
    /// Manage user account's in your system
    User(commands::UserCommands),
}

fn main() {
    use commands::UserCommands;
    let cli = InventoryManagerCli::parse();
    let db = database::DBSQLite::new();

    match cli {
        InventoryManagerCli::User(args) => match args {
            UserCommands::Register(args) => args.register(&db),
        },
    }
}
