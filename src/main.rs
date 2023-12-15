use clap::Parser;

#[derive(Parser)]
#[command(name = "invmanager")]
#[command(bin_name = "invmanager")]
enum InventoryManagerCli {
    /// user account managment
    User(UserArgs),
}

#[derive(clap::Args)]
#[command(author, version, about, long_about = None)]
struct UserArgs {
    /// username of the user
    username: String,
    /// password of the user
    password: String,

    #[arg(long)]
    /// login an existing user and get its auth token
    login: bool,

    #[arg(long)]
    /// register a new user to the system
    register: bool,
}

fn main() {
    let InventoryManagerCli::User(args) = InventoryManagerCli::parse();
    println!("{:?}:{:?}", args.username, args.password);
}
