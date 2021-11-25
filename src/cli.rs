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
        .subcommand(cmd_attribute_check())
        .subcommand(cmd_skillcheck())
        .subcommand(cmd_attack())
        .subcommand(cmd_spell())
        .subcommand(cmd_chant())
        .subcommand(cmd_dodge())
        .subcommand(cmd_parry())
        .subcommand(cmd_roll())
        .subcommand(App::new("ini").about("Performs an initiative roll for the current character"))
}

pub fn get_discord_app() -> App<'static> {
    App::new("dsa-cli")
        .about("Simple discord bot to simplify playing \"Das Schwarze Auge\"")
        .version(get_version())
        .subcommand(App::new("upload")
            .about("Uploads a character for your discord account. The .json file has to be attached to this message")
        )
        .subcommand(App::new("list")
            .about("Shows a list of the currently uploaded characters")
        )
        .subcommand(App::new("select")
            .about("Selects the character containing the given (partial) name")
            .arg(Arg::new("character_name")
                .about("The name for which to search in the character list")
                .takes_value(true)
                .required(true)
            )
        )
        .subcommand(App::new("remove")
            .about("Removes all your characters containing the given (partial) name")
            .arg(Arg::new("character_name")
                .about("The name for which to search in the character list")
                .takes_value(true)
                .required(true)
            )
        )
        .subcommand(cmd_attribute_check().with_discord_character_selection())
        .subcommand(cmd_skillcheck().with_discord_character_selection())
        .subcommand(cmd_attack().with_discord_character_selection())
        .subcommand(cmd_spell().with_discord_character_selection())
        .subcommand(cmd_chant().with_discord_character_selection())
        .subcommand(cmd_parry().with_discord_character_selection())
        .subcommand(cmd_dodge().with_discord_character_selection())
        .subcommand(cmd_roll())
        .subcommand(App::new("rename").about("Rename all players to their respective character name")
            .arg(
                Arg::new("reset")
                .about("Reset player nicknames to their original names")
                .short('r')
                .long("reset")
                .takes_value(false)
            ))
        .subcommand(App::new("ini").about("Performs an initiative roll for the current character")
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

trait DsaAppUtil {
    fn with_discord_character_selection(self) -> Self;
    fn with_simple_facilitation(self) -> Self;
    fn with_attribute_facilitation(self) -> Self;
    fn with_bonus_points(self) -> Self;
}

impl<'a> DsaAppUtil for App<'a> {
    fn with_discord_character_selection(self) -> Self {
        self.arg(
            Arg::new("character_name")
                .about("The name of the character to use")
                .takes_value(true)
                .short('c')
                .long("character"),
        )
        .arg(
            Arg::new("user_ide")
                .about("A discord user for whom to roll the check")
                .takes_value(true)
                .short('u')
                .long("user"),
        )
    }

    fn with_simple_facilitation(self) -> Self {
        self.setting(AppSettings::AllowLeadingHyphen).arg(
            Arg::new("facilitation")
                .about("The facilitation (if positive) or obstruction (if negative)")
                .takes_value(true)
                .default_value("0"),
        )
    }

    fn with_attribute_facilitation(self) -> Self {
        self.setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::new("facilitation")
                    .about("The facilitation (if positive) or obstruction (if negative)")
                    .takes_value(true)
                    .default_value("0"),
            )
            .arg(
                Arg::new("attribute_facilitation")
                    .about("A list of pairs [ATTRIBUTE]:[FACILITATION] separated by commata")
                    .takes_value(true)
                    .long("attribute-facilitation"),
            )
    }

    fn with_bonus_points(self) -> Self {
        self.arg(
            Arg::new("bonus_points")
                .about("Bonus points that are added to the level of the skill")
                .takes_value(true)
                .long("bonus-points"),
        )
    }
}

pub fn get_version() -> &'static str {
    match option_env!("FULL_VERSION") {
        Some(ver) => ver,
        _ => env!("CARGO_PKG_VERSION"),
    }
}

fn cmd_attribute_check() -> App<'static> {
    App::new("attribute")
        .about("Performs an attribute check for the given attribute")
        .arg(
            Arg::new("attribute_name")
                .about("The (partial) name of the attribute")
                .takes_value(true)
                .required(true),
        )
        .with_simple_facilitation()
}

fn cmd_skillcheck() -> App<'static> {
    App::new("check")
        .about("Performs a skillcheck for the given talent")
        .arg(
            Arg::new("skill_name")
                .about("The (partial) name of the talent")
                .takes_value(true)
                .required(true),
        )
        .with_attribute_facilitation()
        .with_bonus_points()
}
fn cmd_attack() -> App<'static> {
    App::new("attack")
        .about("Performs an attack skillcheck for the given combat technique")
        .arg(
            Arg::new("technique_name")
                .about("The (partial) name of the combat technique")
                .takes_value(true)
                .required(true),
        )
        .with_simple_facilitation()
}
fn cmd_spell() -> App<'static> {
    App::new("spell")
        .about("Performs a spell skillcheck for the given spell")
        .arg(
            Arg::new("spell_name")
                .about("The (partial) name of the spell")
                .takes_value(true)
                .required(true),
        )
        .with_attribute_facilitation()
        .with_bonus_points()
}

fn cmd_chant() -> App<'static> {
    App::new("chant")
        .about("Performs a skillcheck for the given chant")
        .arg(
            Arg::new("chant_name")
                .about("The (partial) name of the chant")
                .takes_value(true)
                .required(true),
        )
        .with_attribute_facilitation()
        .with_bonus_points()
}

fn cmd_dodge() -> App<'static> {
    App::new("dodge")
        .about("Performs a dodge skillcheck")
        .with_simple_facilitation()
}
fn cmd_parry() -> App<'static> {
    App::new("parry")
        .about("Performs a parry skillcheck for the given combat technique")
        .arg(
            Arg::new("technique_name")
                .about("The (partial) name of the combat technique")
                .takes_value(true)
                .required(true),
        )
        .with_simple_facilitation()
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
