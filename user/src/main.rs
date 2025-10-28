mod cli;
mod config;
mod handlers;

use std::error::Error;

use clap::Parser;

use crate::cli::{Action, Arguments};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let argument = Arguments::parse();
    match argument.action {
        Action::Poll => handlers::poll().await?,
        Action::Stream => handlers::stream().await?,
    }

    Ok(())
}
