use clap::{Parser, Subcommand, crate_description, crate_version};

#[derive(Debug, Parser)]
#[command(
    long_about = crate_description!(),
    propagate_version = true,
    version = crate_version!(),
)]
pub struct Arguments {
    /// The action to perform
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Debug, Subcommand)]
#[clap(rename_all = "kebab_case")]
pub enum Action {
    /// Perform a read operation to the driver device
    Read {
        // Number of bytes to read
        size: usize,
    },

    /// Perform a write operation to the driver device
    Write {
        // UTF-8 data to write
        data: String,
    },
}
