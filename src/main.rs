mod cli;
mod util;
mod dsa;
mod print;

use print::Printer;

fn main() {
    let printer = print::CLIPrinter {};

    let mut config = match util::Config::get_or_create(&printer) {
        Ok(c) => c,
        Err(e) => {
            printer.output_line(format!("Error while trying to retrieve config: {}", e.message()));
            return;
        }
    };

    let app = cli::get_app(&config);
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("load", sub_m)) => {
            let character =
                match util::Character::load(sub_m.value_of("character_path").unwrap(), &mut config)
                {
                    Ok(c) => c,
                    Err(e) => {
                        printer.output_line(format!("Error loading character: {}", e.message()));
                        return;
                    }
                };
            match config.save() {
                Ok(()) => {
                    printer.output_line(format!("Successfully loaded character \"{}\"", character.get_name()));
                }
                Err(e) => {
                    printer.output_line(format!("Error saving new config: {}", e.message()));
                }
            }
        }

        Some(("unload", _)) => match util::Character::loaded_character(&config) {
            Ok(None) => {
                printer.output_line(String::from("There is no character currently loaded"));
            }
            Ok(Some(c)) => {
                util::Character::unload(&mut config);
                match config.save() {
                    Ok(()) => {
                        printer.output_line(format!("Successfully unloaded character \"{}\"", c.get_name()));
                    }
                    Err(e) => {
                        printer.output_line(format!("Error saving new config: {}", e.message()));
                    }
                }
            }
            Err(_) => {
                util::Character::unload(&mut config);
                match config.save() {
                    Ok(()) => {
                        printer.output_line(String::from("Successfully unloaded invalid character"));
                    }
                    Err(e) => {
                        printer.output_line(format!("Error saving new config: {}", e.message()));
                    }
                }
            }
        }

        Some(("gen-completions", _)) => {
            cli::generate_completions(&config, &printer);
        }

        Some(("skillcheck", sub_m)) => {
            let character = match util::Character::loaded_character(&config) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    printer.output_line(String::from("Error: No character loaded"));
                    return;
                }
                Err(e) => {
                    printer.output_line(format!("Error retrieving loaded character: {}", e.message()));
                    return;
                }
            };

            let (skill_name, facilitation): (&str, i64) = match sub_m.subcommand() {
                Some((s, sub_sub_m)) => {
                    match sub_sub_m.value_of("facilitation").unwrap().parse() {
                        Ok(f) => (s, f),
                        Err(_) => {
                            printer.output_line(format!("Error: facilitation must be an integer"));
                            return;
                        }
                    }
                },
                _ => {
                    printer.output_line(String::from("Error: skill name missing"));
                    return;
                }
            };
            dsa::skillcheck(skill_name, facilitation, &character, &config, &printer);
        }
        _ => {}
    };
}
