mod character;
mod character_manager;
mod config;
mod discord;
mod discord_commands;
mod dsa;
mod greet;
mod util;

#[macro_use]
extern crate enum_display_derive;

use std::sync::Arc;

use crate::{discord::run_discord_bot, util::ErrorType};
use anyhow::{Context, Error};
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

fn get_dsa_data(config: &Config) -> Result<DSAData, Error> {
    let dsa_data = match DSAData::get_or_create() {
        Ok(d) => d,
        Err(e) => {
            if config.auto_update_dsa_data && matches!(e.err_type(), ErrorType::InvalidInput(_)) {
                println!(
                    "Found invalid dsa data, replacing it with a newer version ({})",
                    e
                );
                DSAData::create_default()?;
                return Ok(DSAData::read()?);
            } else {
                return Err(Error::from(e));
            }
        }
    };
    let dsa_data = dsa_data.check_replacement_needed(config);
    Ok(dsa_data)
}
