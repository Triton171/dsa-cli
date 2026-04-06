mod character;
mod character_manager;
mod config;
mod discord;
mod discord_commands;
mod dsa;
mod greet;
mod util;

use std::sync::Arc;

use crate::discord::run_discord_bot;
use anyhow::Error;
use config::{AbstractConfig, Config, DSAData};
use tokio::runtime::Builder;

fn main() -> Result<(), Error> {
    let config = Arc::new(Config::get_or_create()?);
    let dsa_data = Arc::new(get_dsa_data(&config)?);
    let runtime = Builder::new_multi_thread()
        .worker_threads(config.discord.num_threads)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        if let Err(err) = run_discord_bot(config, dsa_data).await {
            println!(
                "Critical error occurred while running the discord bot: {}\n{}",
                err,
                err.backtrace()
            );
        }
    });
    Ok(())
}

fn get_dsa_data(config: &Config) -> anyhow::Result<DSAData> {
    let dsa_data = DSAData::get_or_create()?;
    let dsa_data = dsa_data.check_replacement_needed(config)?;
    Ok(dsa_data)
}
