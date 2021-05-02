use super::util::OutputWrapper;
use clap::{App, AppSettings, Arg};
use clap_generate::{generate, generators::Bash};
use std::env;
use std::fs;
use std::io::BufWriter;
use std::path;

/*
Returns the clap app definition
*/
pub fn get_app() -> App<'static> {
    App::new("dsa-cli")
        .about("Simple command line tool to simplify playing \"Das Schwarze Auge\"")
        .version(get_version())
        .subcommand(
            App::new("load")
                .about("Loads a character from the given JSON file")
                .arg(
                    Arg::new("character_path")
                        .about("The path to the character JSON file")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(App::new("unload").about("Unloads the current character, if one is loaded"))
        .subcommand(App::new("discord").about("Starts the discord bot"))
        .subcommand(
            App::new("gen-completions").about("Generates completion scripts for detected shells"),
        )
        .subcommand(cmd_skillcheck())
        .subcommand(cmd_attack())
        .subcommand(cmd_spell())
        .subcommand(cmd_dodge())
        .subcommand(cmd_parry())
        .subcommand(cmd_roll())
        .subcommand(cmd_ini())
}

pub fn get_discord_app() -> App<'static> {
    App::new("dsa-cli")
        .about("Simple discord bot to simplify playing \"Das Schwarze Auge\"")
        .version(get_version())
        .subcommand(App::new("upload")
            .about("Uploads and loads a character for your discord account. The .json file has to be attached to this message")
        )
        .subcommand(cmd_skillcheck())
        .subcommand(cmd_attack())
        .subcommand(cmd_attack())
        .subcommand(cmd_spell())
        .subcommand(cmd_dodge())
        .subcommand(cmd_parry())
        .subcommand(cmd_roll())
        .subcommand(cmd_ini()
            .arg(
                Arg::new("all")
                    .about("Adds the characters of all users in this server to the initiative roll")
                    .short('a')
                    .long("all")
                    .takes_value(false)
            )
            .arg(
                Arg::new("new")
                    .about("Adds one or more custom character(s) to the roll. For each character, name and initiative level have to be specified, everything separated by spaces")
                    .short('n')
                    .long("new")
                    .takes_value(true)
                    .multiple(true)
                    .min_values(2)
            )
            .arg(
                Arg::new("rename")
                    .about("Changes the discord nickname of each user to start with the initiative")
                    .long("rename")
                    .takes_value(false)
                    .requires("all")
            )
            .arg(
                Arg::new("reset")
                    .about("Resets all nicknames that have been changed by the \"rename\" option")
                    .long("reset")
                    .takes_value(false)
                    .exclusive(true)
            )
        )
        .override_usage("![subcommand]")
}

fn get_version() -> &'static str {
    "1.1"
}

fn cmd_skillcheck() -> App<'static> {
    App::new("check")
        .about("Performs a skillcheck for the given skill")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("skill_name")
                .about("The (partial) name of the skill")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("facilitation")
                .about("The level of facilitation (if positive) or obstruction (if negative). Seperate several values by commas to use different values for each roll")
                .takes_value(true)
                .default_value("0"),
        )
}
fn cmd_attack() -> App<'static> {
    App::new("attack")
        .about("Performs an attack skillcheck for the given combat technique")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("technique_name")
                .about("The (partial) name of the combat technique")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("facilitation")
                .about("The level of facilitation (if positive) or obstruction (if negative)")
                .takes_value(true)
                .default_value("0"),
        )
}
fn cmd_spell() -> App<'static> {
    App::new("spell")
        .about("Performs a spell skillcheck for the given spell")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("spell_name")
                .about("The (partial) name of the spell")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("facilitation")
                .about("The level of facilitation (if positive) or obstruction (if negative). Seperate several values by commas to use different values for each roll")
                .takes_value(true)
                .default_value("0"),
        )
}
fn cmd_dodge() -> App<'static> {
    App::new("dodge")
        .about("Performs a dodge skillcheck")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("facilitation")
                .about("The level of facilitation (if positive) or obstruction (if negative)")
                .takes_value(true)
                .default_value("0"),
        )
}
fn cmd_parry() -> App<'static> {
    App::new("parry")
        .about("Performs a parry skillcheck for the given combat technique")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("technique_name")
                .about("The (partial) name of the combat technique")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("facilitation")
                .about("The level of facilitation (if positive) or obstruction (if negative)")
                .takes_value(true)
                .default_value("0"),
        )
}
fn cmd_roll() -> App<'static> {
    App::new("roll").about("Rolls some dice").arg(
        Arg::new("dice_expression")
            .about("The dice to roll. Syntax: [number_of_dice]d[dice_type] + [offset]")
            .takes_value(true)
            .multiple(true)
            .required(true),
    )
}
fn cmd_ini() -> App<'static> {
    App::new("ini").about("Performs an initiative roll for the current character")
}

pub fn generate_completions(printer: &mut impl OutputWrapper) {
    let mut app = get_app();

    if cfg!(target_os = "linux") {
        let home = match env::var("HOME") {
            Ok(s) => s,
            Err(_) => {
                printer.output_line(&"Could not read environment variable $HOME");
                return;
            }
        };
        let mut path = path::PathBuf::new();
        path.push(home);
        path.push(".bashrc");
        //Check for bash
        if path::Path::exists(&path) {
            match super::config::get_config_dir() {
                Ok(mut bash_completions) => {
                    bash_completions.push("completions_bash.bash");
                    let bash_completions_str = String::from(bash_completions.to_str().unwrap());
                    match fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&bash_completions)
                    {
                        Ok(file) => {
                            let mut writer = BufWriter::new(file);
                            generate::<Bash, _>(&mut app, "dsa-cli", &mut writer);

                            if path::Path::exists(&bash_completions) {
                                printer.output_line(&format!(
                                    "Generated bash completions script at {}",
                                    bash_completions_str
                                ));
                                printer.output_line(
                                    &"Call this script in your ~/.bashrc to enable completions",
                                );
                            } else {
                                printer.output_line(&"Unknown error occurred while trying to generate bash completions script");
                            }
                        }
                        Err(e) => {
                            printer.output_line(&format!(
                                "Unable to write to {}: {}",
                                bash_completions_str,
                                e.to_string()
                            ));
                        }
                    };
                }
                Err(e) => {
                    printer.output_line(&format!("Error resolving config folder: {}", e.message()));
                    return;
                }
            };
        }
    }
}
