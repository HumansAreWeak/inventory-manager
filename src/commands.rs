use clap::{Args, Subcommand};

use crate::database::InvManActions;

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// Register a new user
    Register(UserArgs),
}

#[derive(Args, Debug)]
pub struct UserArgs {
    /// Name of the user
    name: String,
    /// Password of the user
    password: String,

    /// Login the user after registration
    #[arg(short, long)]
    login: bool,
}

impl UserArgs {
    pub fn register(&self, db: &dyn InvManActions) -> Option<String> {
        db.user_register(self.name, self.password);
        return match self.login {
            true => Some(self.login(db)),
            false => None,
        };
    }

    pub fn login(&self, db: &dyn InvManActions) -> String {
        return db.user_login(self.name, self.password);
    }
}
