use clap::{App, AppSettings, Arg};

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
            .about("Uploads a character for your discord account. The .json file has to be attached to this message")
        )
        .subcommand(cmd_skillcheck())
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
                Arg::new("rename")
                    .about("Adds the initiative to everyones discord nickname")
                    .short('r')
                    .long("rename")
                    .takes_value(false)
                    .requires("all")
            )
            .arg(
                Arg::new("new")
                    .about("Adds one or more custom character(s) to the roll")
                    .short('n')
                    .long("new")
                    .takes_value(true)
                    .multiple(true)
                    .min_values(2)
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

pub fn get_version() -> &'static str {
    match option_env!("FULL_VERSION") {
        Some(ver) => ver,
        _ => env!("CARGO_PKG_VERSION"),
    }
}

fn cmd_skillcheck() -> App<'static> {
    App::new("check")
        .about("Performs a skillcheck for the given talent")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::new("skill_name")
                .about("The (partial) name of the talent")
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
                .about("The level of facilitation (if positive) or obstruction (if negative)")
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
    App::new("roll")
        .about("Rolls some dice")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
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
