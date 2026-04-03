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

use config::{AbstractConfig, Config, DSAData};
use util::{Error, OutputWrapper};

use crate::util::ErrorType;

fn main() {
    // TODO: Start bot
}

fn get_dsa_data(config: &Config, output: &mut impl OutputWrapper) -> Result<DSAData, Error> {
    let dsa_data = match DSAData::get_or_create(output) {
        Ok(d) => d,
        Err(e) => {
            if config.auto_update_dsa_data && matches!(e.err_type(), ErrorType::InvalidInput(_)) {
                output.output_line(&format!(
                    "Found invalid dsa data, replacing it with a newer version ({})",
                    e
                ));
                DSAData::create_default()?;
                return DSAData::read();
            } else {
                return Err(e);
            }
        }
    };
    let dsa_data = dsa_data.check_replacement_needed(config, output);
    Ok(dsa_data)
}
