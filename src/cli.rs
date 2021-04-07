use super::util;
use super::print::Printer;
use clap::{App, Arg};
use clap_generate::{generate, generators::Bash};
use std::env;
use std::fs;
use std::path;
use std::io::BufWriter;

/*
Returns the clap app definition
*/
pub fn get_app(config: &util::Config) -> App<'static> {
    let mut app = App::new("DSA-CLI")
        .about("Simple command line program for playing DSA")
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
        .subcommand(App::new("unload"))
            .about("Unloads the current character, if one is loaded")
        .subcommand(App::new("gen-completions")
            .about("Generates completion scripts for detected shells"));

    let mut skillcheck = App::new("skillcheck").about("Performs a skillcheck for the given skill");
    for skill in &config.skills {
        skillcheck = skillcheck.subcommand(
            App::new(skill.0).arg(
                Arg::new("facilitation")
                    .about("The level of facilitation (if positive) or obstruction (if negative)")
                    .index(1)
                    .takes_value(true)
                    .default_value("0"),
            )
            .setting(clap::AppSettings::AllowLeadingHyphen),
        );
    }
    app = app.subcommand(skillcheck);

    app
}


pub fn generate_completions(config: &util::Config, printer: &impl Printer) {
    let mut app = get_app(config);

    if cfg!(target_os = "linux") {
        let home = match env::var("HOME") {
            Ok(s) => s,
            Err(_) => {
                printer.output_line(String::from("Could not read environment variable $HOME"));
                return;
            }
        };
        let mut path = path::PathBuf::new();
        path.push(home);
        path.push(".bashrc");
        //Check for bash
        if path::Path::exists(&path) {
            match util::get_config_dir() {
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
                                printer.output_line(format!("Generated bash completions script at {}", bash_completions_str));
                                printer.output_line(String::from("Call this script in your ~/.bashrc to enable completions"));
                            } else {
                                printer.output_line(String::from("Unknown error occurred while trying to generate bash completions script"));
                            }
                        }
                        Err(e) => {
                            printer.output_line(format!("Unable to write to {}: {}", bash_completions_str, e.to_string()));
                        }
                    };
                },
                Err(e) => {
                    printer.output_line(format!("Error resolving config folder: {}", e.message()));
                    return;
                }
            };
        }
    }

    
}