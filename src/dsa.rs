use super::util;
use super::print::Printer;
use rand::distributions::{Uniform, Distribution};

pub fn skillcheck(skill_id: &str, facilitation: i64, character: &util::Character, config: &util::Config, printer: &impl Printer) {
    let skill_config = match config.skills.get(skill_id) {
        Some(c) => c,
        None => {
            printer.output_line(format!("Error: No skill config found for \"{}\"", skill_id));
            return;
        }
    };

    let mut rng = rand::thread_rng();
    let d20 = Uniform::new_inclusive(1, 20);


    let mut levels: Vec<i64> = Vec::new();
    let mut rolls: Vec<i64> = Vec::new();

    let skill_level = character.get_skill_level(skill_id);
    let mut rem_points = skill_level;
    for attr in &skill_config.attributes {
        let attr_level = character.get_attribute_level(attr);
        let attr_roll: i64 = d20.sample(&mut rng);
        rem_points -= std::cmp::max(attr_roll - (attr_level + facilitation), 0);
        levels.push(attr_level);
        rolls.push(attr_roll);
    }

    let mut crits = false;
    let mut unconfirmed_crit_succ = 0;
    let mut crit_succ = 0;
    let mut unconfirmed_crit_fail = 0;
    let mut crit_fail = 0;
    
    let mut crit_rolls: Vec<String> = vec![String::from("Crit rolls:")];
    if let Some(true) = config.alternative_crits {
        for (i, &roll) in rolls.iter().enumerate() {
            if roll==1 {
                crits = true;
                let crit_roll = d20.sample(&mut rng);
                crit_rolls.push(crit_roll.to_string());
                if crit_roll<=levels[i] + facilitation {
                    crit_succ += 1;
                } else {
                    unconfirmed_crit_succ += 1;
                }
            } else if roll==20 {
                crits = true;
                let crit_roll = d20.sample(&mut rng);
                crit_rolls.push(crit_roll.to_string());
                if crit_roll>levels[i] + facilitation {
                    crit_fail += 1;
                } else {
                    unconfirmed_crit_fail += 1;
                }
            } else {
                crit_rolls.push(String::from(""));
            }
        }
    } else {
        //TODO
    }

    //Print
    let upped_skill_name = uppercase_first(skill_id);
    printer.output_line(format!("{}, {} level {}, facilitation: {}", 
        character.get_name(), upped_skill_name, skill_level, facilitation));

    if rem_points < 0 {
        printer.output_line(format!("Failed (remaining skill points: {})", rem_points))
    } else {
        if rem_points==0 {
            printer.output_line(format!("Passed with quality level 1 (remaing skill points: {})", rem_points));
        } else {
            let mut quality = (rem_points as f64/3f64).ceil();
            if quality > 6f64 {
                quality = 6f64;
            }
            printer.output_line(format!("Passed with quality level {} (remaining skill points: {})", quality, rem_points));
        }
    }
    if crits {
        printer.new_line();
        if crit_succ > 0 {
            printer.output_line(format!("Critical success: {}", crit_succ));
        }
        if unconfirmed_crit_succ > 0 {
            printer.output_line(format!("Unconfirmed critical success: {}", unconfirmed_crit_succ));
        }
        if crit_fail > 0 {
            printer.output_line(format!("Critical failure: {}", crit_fail));
        }
        if unconfirmed_crit_fail > 0 {
            printer.output_line(format!("Unconfirmed critical failure: {}", unconfirmed_crit_fail));
        }
    }
    printer.new_line();


    let mut table: Vec<Vec<String>> = Vec::new();
    let mut header: Vec<String> = Vec::new();
    header.push(String::from(""));
    header.extend(skill_config.attributes.iter().map(|s| uppercase_first(s)));
    table.push(header);

    let mut levels_row: Vec<String> = Vec::new();
    levels_row.push(String::from("Character:"));
    if facilitation==0 {
        levels_row.extend(levels.iter().map(|l| format!("{}", l)));
    } else if facilitation > 0 {
        levels_row.extend(levels.iter().map(|l| format!("{} + {}", l, facilitation)));
    } else {
        levels_row.extend(levels.iter().map(|l| format!("{} - {}", l, -facilitation)));
    }
    table.push(levels_row);

    let mut rolls_row: Vec<String> = Vec::new();
    rolls_row.push(String::from("Rolls:"));
    rolls_row.extend(rolls.iter().map(|r| r.to_string()));
    table.push(rolls_row);


    if let Some(true) = config.alternative_crits {
        if crits {
            table.push(crit_rolls);
        }
    }

    printer.output_table(&table);
}



fn uppercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}