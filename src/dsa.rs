use super::character::Character;
use super::config::Config;
use super::util;
use super::util::OutputWrapper;
use clap::ArgMatches;
use rand::Rng;
use rand::distributions::{Distribution, Uniform};

pub fn skill_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    config: &Config,
    output: &dyn OutputWrapper,
) {
    let skill_name = cmd_matches.value_of("skill_name").unwrap();
    let facilitation = match cmd_matches.value_of("facilitation").unwrap().parse() {
        Ok(f) => f,
        Err(_) => {
            output.output_line(String::from("Error: facilitation must be an integer"));
            return;
        }
    };

    let skill_attrs = &config.skills.get(skill_name).unwrap().attributes;
    let attrs: Vec<(String, i64)> = skill_attrs
        .iter()
        .map(|attr| (attr.clone(), character.get_attribute_level(attr)))
        .collect();
    let skill_level = character.get_skill_level(skill_name);

    let crit_type = match config.alternative_crits {
        Some(true) => CritType::ConfirmableCrits,
        _ => CritType::MultipleRequiredCrits(2),
    };

    roll_check(
        &attrs,
        skill_name,
        character.get_name(),
        facilitation,
        CheckType::PointsCheck(skill_level),
        crit_type,
        output,
    );
}

pub fn attack_check(cmd_matches: &ArgMatches, character: &Character, output: &dyn OutputWrapper) {
    let technique_name = cmd_matches.value_of("technique_name").unwrap();
    let facilitation: i64 = match cmd_matches.value_of("facilitation").unwrap().parse() {
        Ok(f) => f,
        Err(_) => {
            output.output_line(String::from("Error: facilitation must be an integer"));
            return;
        }
    };

    let attack_level = character.get_attack_level(technique_name);
    roll_check(
        &[(technique_name.to_string(), attack_level)],
        &format!("Attack: {}", technique_name),
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::ConfirmableCrits,
        output,
    );
}

pub fn roll(cmd_matches: &ArgMatches, output: &dyn OutputWrapper) {
    let mut rng = rand::thread_rng();
    let expr =
        cmd_matches
            .values_of("dice_expression")
            .unwrap()
            .fold(String::from(""), |mut beg, val| {
                beg.push_str(val);
                beg.push_str(" ");
                beg
            });
    let terms = expr.split('+').map(|term| term.trim());
    let mut res = 0;
    output.output(String::from("Rolls: "));
    for term in terms {
        //Dice
        if term.contains('d') {
            let split: Vec<&str> = term.split('d').filter(|s| !s.is_empty()).collect();
            if split.len()==0 {
                output.new_line();
                output.output_line(format!("Die type missing in expression \"{}\"", term));
                return;
            } else if split.len()<3 {
                let num_dice = match split.len() {
                    1 => 1,
                    //2
                    _ => {
                        match split[0].parse::<u32>() {
                            Ok(num) => num,
                            Err(_) => {
                                output.new_line();
                                output.output_line(format!("Invalid die number in expression \"{}\"", term));
                                return;
                            }
                        }
                    }
                };
                match split[split.len()-1].parse::<u32>() {
                    Ok(d_type) => {
                        for _ in 0..num_dice {
                            let roll = rng.gen_range(1..=d_type);
                            output.output(format!("{}/{} ", roll, d_type));
                            res += roll;
                        }
                    }
                    Err(_) => {
                        output.new_line();
                        output.output_line(format!("Invalid die type in expression \"{}\"", term));
                        return;
                    }
                }
            } else {
                output.new_line();
                output.output_line(format!("Too many \"d\"s in expression \"{}\"", term));
                return;
            }
        } else {
            match term.trim().parse::<u32>() {
                Ok(num) => { res += num; }
                Err(_) => {
                    output.new_line();
                    output.output_line(format!("Unable to parse number \"{}\"", term));
                    return;
                }
            }
        }
    }

    output.new_line();
    output.output_line(format!("Total: {}", res));
}

enum CheckType {
    //A simple check where you have to roll below your attributes (for example an attribute check)
    SimpleCheck,
    //A check where you can compensate for higher rolls with some available points
    PointsCheck(i64),
}

enum CritType {
    //A check without critical successes or failures
    NoCrits,
    //A check where crits have to be confirmed with a second roll
    ConfirmableCrits,
    //A check where some number of 1 (or 20) rolls are required to trigger a crit (this number can also be 1)
    MultipleRequiredCrits(u32),
}

fn roll_check(
    attributes: &[(String, i64)],
    check_name: &str,
    character_name: &str,
    facilitation: i64,
    check_type: CheckType,
    crit_type: CritType,
    output: &dyn OutputWrapper,
) {
    let mut rng = rand::thread_rng();
    let d20 = Uniform::new_inclusive(1, 20);

    //Compute the rolls
    let mut rolls: Vec<i64> = Vec::with_capacity(attributes.len());
    let mut points = match check_type {
        CheckType::SimpleCheck => 0,
        CheckType::PointsCheck(avail_points) => avail_points,
    };
    for (_, level) in attributes {
        let roll = d20.sample(&mut rng);
        points = points - std::cmp::max(0, roll - (level + facilitation));
        rolls.push(roll);
    }
    //Check for crits
    let mut crits = false;
    let mut unconfirmed_crit_succ = 0;
    let mut crit_succ = 0;
    let mut unconfirmed_crit_fail = 0;
    let mut crit_fail = 0;
    let mut crits_row: Vec<String> = Vec::new();
    match crit_type {
        CritType::NoCrits => {}
        CritType::ConfirmableCrits => {
            crits_row.push(String::from("Crit roll:"));
            for ((_, level), &roll) in attributes.iter().zip(rolls.iter()) {
                if roll == 1 {
                    let crit_roll = d20.sample(&mut rng);
                    crits_row.push(crit_roll.to_string());
                    crits = true;
                    if crit_roll <= *level {
                        crit_succ += 1;
                    } else {
                        unconfirmed_crit_succ += 1;
                    }
                } else if roll == 20 {
                    let crit_roll = d20.sample(&mut rng);
                    crits_row.push(crit_roll.to_string());
                    crits = true;
                    if crit_roll > *level {
                        crit_fail += 1;
                    } else {
                        unconfirmed_crit_fail += 1;
                    }
                } else {
                    crits_row.push(String::from(""));
                }
            }
        }
        CritType::MultipleRequiredCrits(num_required) => {
            let mut num_succ: u32 = 0;
            let mut num_fail: u32 = 0;
            for &roll in &rolls {
                if roll == 1 {
                    num_succ += 1;
                } else if roll == 20 {
                    num_fail += 1;
                }
            }
            if num_succ >= num_required {
                crits = true;
                crit_succ = 1;
            }
            if num_fail >= num_required {
                crits = true;
                crit_fail = 1;
            }
        }
    };

    //Output
    match check_type {
        CheckType::SimpleCheck => {
            output.output_line(format!(
                "{}, Check for \"{}\"",
                character_name,
                util::uppercase_first(check_name)
            ));
        }
        CheckType::PointsCheck(avail_points) => {
            output.output_line(format!(
                "{}, Check for {} (level {})",
                character_name,
                util::uppercase_first(check_name),
                avail_points
            ));
        }
    };
    output.new_line();

    let mut table: Vec<Vec<String>> = Vec::with_capacity(2);

    let mut header: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    header.push(String::from(""));
    header.extend(
        attributes
            .iter()
            .map(|(name, _)| util::uppercase_first(name)),
    );
    table.push(header);

    let mut char_row: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    char_row.push(String::from("Character:"));
    if facilitation == 0 {
        char_row.extend(attributes.iter().map(|(_, level)| level.to_string()));
    } else if facilitation > 0 {
        char_row.extend(
            attributes
                .iter()
                .map(|(_, level)| format!("{} + {}", level, facilitation)),
        );
    } else {
        char_row.extend(
            attributes
                .iter()
                .map(|(_, level)| format!("{} - {}", level, -facilitation)),
        );
    }
    table.push(char_row);

    let mut rolls_row: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    rolls_row.push(String::from("Roll:"));
    rolls_row.extend(rolls.iter().map(|roll| roll.to_string()));
    table.push(rolls_row);

    if let CritType::ConfirmableCrits = crit_type {
        if crits {
            table.push(crits_row);
        }
    }
    output.output_table(&table);
    output.new_line();

    if points < 0 {
        output.output_line(String::from("Check failed"));
    } else {
        match check_type {
            CheckType::SimpleCheck => {
                output.output_line(String::from("Check passed"));
            }
            CheckType::PointsCheck(_) => {
                let mut quality: u32 = (points as f32 / 3f32).ceil() as u32;
                if quality == 0 {
                    quality = 1;
                } else if quality > 6 {
                    quality = 6;
                }
                output.output_line(format!("Check passed, quality level {}", quality));
            }
        }
    }

    if crits {
        if crit_succ == 1 {
            output.output_line(String::from("Critical success"));
        } else if crit_succ > 1 {
            output.output_line(format!("{} critical successes", crit_succ));
        }
        if unconfirmed_crit_succ == 1 {
            output.output_line(String::from("Unconfirmed critical success"));
        } else if unconfirmed_crit_succ > 1 {
            output.output_line(format!(
                "{} unconfirmed critical successes",
                unconfirmed_crit_succ
            ));
        }
        if crit_fail == 1 {
            output.output_line(String::from("Critical failure"));
        } else if crit_fail > 1 {
            output.output_line(format!("{} critical failures", crit_fail));
        }
        if unconfirmed_crit_fail == 1 {
            output.output_line(String::from("Unconfirmed critical failure"));
        } else if unconfirmed_crit_fail > 1 {
            output.output_line(format!(
                "{} unconfirmed critical failures",
                unconfirmed_crit_fail
            ));
        }
    }
}
