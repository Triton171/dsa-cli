use super::character::Character;
use super::config::{self, Config, DSAData};
use super::util::*;
use clap::ArgMatches;
use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use std::{cmp::Ordering, num::ParseIntError};

//The maximum number of dice in a roll expression
const MAX_NUM_DICE: u32 = 100;
//The maximum number of expressions in a roll command
const MAX_ROLL_EXPRESSIONS: u32 = 20;

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

enum Facilitation {
    SimpleFacilitation(i64),
    IndividualFacilitation(Vec<i64>),
}

impl Facilitation {
    fn iter<'a>(&'a self) -> Box<dyn std::iter::Iterator<Item = &i64> + 'a> {
        match self {
            Facilitation::SimpleFacilitation(fac) => Box::new(std::iter::repeat(fac)),
            Facilitation::IndividualFacilitation(facs) => Box::new(facs.iter()),
        }
    }
}

pub fn talent_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    config: &Config,
    output: &mut impl OutputWrapper,
) {
    let (talent_name, talent_entry) = match DSAData::match_search(
        &dsa_data.talents,
        cmd_matches.value_of("skill_name").unwrap(),
    ) {
        Ok(name) => name,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let skill_attrs = &talent_entry.attributes;
    let facilitation = match get_facilitation(cmd_matches, skill_attrs.len()) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let attrs: Vec<(&str, i64)> = skill_attrs
        .iter()
        .map(|attr| {
            (
                dsa_data.get_attr_short_name(attr),
                character.get_attribute_level(attr),
            )
        })
        .collect();
    let skill_level = character.get_skill_level(&talent_name);

    let crit_type = match config.dsa_rules.crit_rules {
        config::ConfigDSACritType::NoCrits => CritType::NoCrits,
        config::ConfigDSACritType::DefaultCrits => CritType::MultipleRequiredCrits(2),
        config::ConfigDSACritType::AlternativeCrits => CritType::ConfirmableCrits,
    };

    roll_check(
        &attrs,
        &talent_name,
        character.get_name(),
        facilitation,
        CheckType::PointsCheck(skill_level),
        crit_type,
        output,
    );
}

pub fn attack_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    output: &mut impl OutputWrapper,
) {
    let (technique_name, _) = match DSAData::match_search(
        &dsa_data.combat_techniques,
        cmd_matches.value_of("technique_name").unwrap(),
    ) {
        Ok(t) => t,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let facilitation = match get_facilitation(cmd_matches, 1) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };

    let attack_level = character.get_attack_level(technique_name);
    roll_check(
        &[(technique_name, attack_level)],
        &format!("Attack: {}", technique_name),
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::ConfirmableCrits,
        output,
    );
}

pub fn spell_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    config: &Config,
    output: &mut impl OutputWrapper,
) {
    let (spell_name, spell_entry) = match DSAData::match_search(
        &dsa_data.spells,
        cmd_matches.value_of("spell_name").unwrap(),
    ) {
        Ok(s) => s,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let spell_attrs = &spell_entry.attributes;
    let facilitation = match get_facilitation(cmd_matches, spell_attrs.len()) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };

    let attrs: Vec<(&str, i64)> = spell_attrs
        .iter()
        .map(|attr| {
            (
                dsa_data.get_attr_short_name(attr),
                character.get_attribute_level(attr),
            )
        })
        .collect();
    let skill_level = character.get_spell_level(&spell_name);

    let crit_type = match config.dsa_rules.crit_rules {
        config::ConfigDSACritType::NoCrits => CritType::NoCrits,
        config::ConfigDSACritType::DefaultCrits => CritType::MultipleRequiredCrits(2),
        config::ConfigDSACritType::AlternativeCrits => CritType::ConfirmableCrits,
    };

    roll_check(
        &attrs,
        &spell_name,
        character.get_name(),
        facilitation,
        CheckType::PointsCheck(skill_level),
        crit_type,
        output,
    );
}

pub fn dodge_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    output: &mut impl OutputWrapper,
) {
    let facilitation = match get_facilitation(cmd_matches, 1) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let dodge_level = character.get_dodge_level();
    roll_check(
        &[("Ausweichen", dodge_level)],
        "Ausweichen",
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::ConfirmableCrits,
        output,
    );
}

pub fn parry_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    output: &mut impl OutputWrapper,
) {
    let (technique_name, technique_entry) = match DSAData::match_search(
        &dsa_data.combat_techniques,
        cmd_matches.value_of("technique_name").unwrap(),
    ) {
        Err(e) => {
            output.output_line(&e);
            return;
        }
        Ok(r) => r,
    };
    let facilitation = match get_facilitation(cmd_matches, 1) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let parry_level = character.get_parry_level(&technique_name, &technique_entry.attributes);
    roll_check(
        &[("Parade", parry_level)],
        technique_name,
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::ConfirmableCrits,
        output,
    )
}

pub fn roll(cmd_matches: &ArgMatches, output: &mut impl OutputWrapper) {
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

    let mut res = 0;
    output.output(&"Rolls: ");

    let mut parse_term = |term: &str| {
        let (term_sign, term) = match term.chars().next() {
            Some('+') => (1, &term[1..]),
            Some('-') => (-1, &term[1..]),
            _ => (1, term),
        };
        let mut term_val: i64 = 0;
        if term.contains(|c| c == 'd' || c == 'w') {
            let split: Vec<&str> = term
                .split(|c| c == 'd' || c == 'w')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            let (num_dice, die_type) = match split.len() {
                0 => {
                    return Err(Error::new(
                        format!("Die type missing in expression \"{}\"", term),
                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                    ));
                }
                1 => (
                    1,
                    match split[0].parse::<u32>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(Error::new(
                                format!("Unable to parse die type in expression \"{}\"", term),
                                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                            ));
                        }
                    },
                ),
                2 => (
                    match split[0].parse::<u32>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(Error::new(
                                format!("Invalid die number in expression \"{}\"", term),
                                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                            ));
                        }
                    },
                    match split[1].parse::<u32>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(Error::new(
                                format!("Unable to parse die type in expression \"{}\"", term),
                                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                            ));
                        }
                    },
                ),
                _ => {
                    return Err(Error::new(
                        format!("Too many \"d\"s and/or \"w\"s in expression \"{}\"", term),
                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                    ));
                }
            };

            if die_type <= 0 {
                return Err(Error::new(
                    format!("Invalid die type: {}", die_type),
                    ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                ));
            }
            if num_dice > MAX_NUM_DICE {
                return Err(Error::new(
                    format!(
                        "Number of dice exceeds maximum of {}: {}",
                        MAX_NUM_DICE, num_dice
                    ),
                    ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                ));
            }

            for _ in 0..num_dice {
                let roll: i64 = rng.gen_range(1..=(die_type as i64));
                output.output(&format!("{}/{} ", roll, die_type));
                term_val += roll;
            }
        } else {
            match term.trim().parse::<i64>() {
                Ok(num) => {
                    term_val += num;
                }
                Err(_) => {
                    return Err(Error::new(
                        format!("Unable to parse number \"{}\"", term),
                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                    ));
                }
            }
        }
        term_val *= term_sign;
        Ok(term_val)
    };

    let mut begin_idx: usize = 0;
    let mut expr_count: u32 = 0;
    loop {
        expr_count += 1;
        if expr_count > MAX_ROLL_EXPRESSIONS {
            output.new_line();
            output.output_line(&format!(
                "Error: Number of roll expressions exceeds maximum of {}",
                MAX_ROLL_EXPRESSIONS
            ));
            return;
        }
        let end_idx = match expr[begin_idx + 1..].find(|c| c == '+' || c == '-') {
            Some(idx) => begin_idx + 1 + idx,
            None => expr.len(),
        };
        res += match parse_term(&expr[begin_idx..end_idx]) {
            Ok(term_val) => term_val,
            Err(e) => {
                output.new_line();
                output.output_line(&e);
                return;
            }
        };
        begin_idx = end_idx;
        if begin_idx >= expr.len() {
            break;
        }
    }

    output.new_line();
    output.output_line(&format!("Total: {}", res));
}

/*
Accepts a slice of (name, iniative-level) tupels, rolls and prints their initiatives
Returns a sorted vector of (index, rolls) tuples
*/
pub fn roll_ini(
    characters: &[(String, i64)],
    output: &mut impl OutputWrapper,
) -> Vec<(usize, Vec<i64>)> {
    let mut rng = rand::thread_rng();
    let d6 = Uniform::new_inclusive(1, 6);

    //A vector saving (index, rolls) for each character
    let mut inis: Vec<(usize, Vec<i64>)> = Vec::with_capacity(characters.len());
    for (i, _) in characters.iter().enumerate() {
        inis.push((i, vec![d6.sample(&mut rng)]));
    }

    //Roll additional dice for characters that have equal INI and rolls
    for i in 0..inis.len() {
        for j in i + 1..inis.len() {
            if characters[inis[i].0].1 == characters[inis[j].0].1 {
                while {
                    let mut needs_additional_rolls = true;
                    for k in 0..std::cmp::min(inis[i].1.len(), inis[j].1.len()) {
                        if inis[i].1[k] != inis[j].1[k] {
                            needs_additional_rolls = false;
                            break;
                        }
                    }
                    needs_additional_rolls
                } {
                    if inis[i].1.len() < inis[j].1.len() {
                        inis[i].1.push(d6.sample(&mut rng));
                    } else if inis[j].1.len() < inis[i].1.len() {
                        inis[j].1.push(d6.sample(&mut rng));
                    } else {
                        inis[i].1.push(d6.sample(&mut rng));
                        inis[j].1.push(d6.sample(&mut rng));
                    }
                }
            }
        }
    }

    //Reverse sort
    inis.sort_by(|(index1, rolls1), (index2, rolls2)| {
        if characters[*index1].1 + rolls1[0] < characters[*index2].1 + rolls2[0] {
            Ordering::Greater
        } else if characters[*index1].1 + rolls1[0] > characters[*index2].1 + rolls2[0] {
            Ordering::Less
        } else {
            if characters[*index1].1 < characters[*index2].1 {
                Ordering::Greater
            } else if characters[*index1].1 > characters[*index2].1 {
                Ordering::Less
            } else {
                for i in 1..rolls1.len() {
                    if rolls1[i] < rolls2[i] {
                        return Ordering::Greater;
                    } else if rolls1[i] > rolls2[i] {
                        return Ordering::Less;
                    }
                }
                panic!("Unable to sort initiatives");
            }
        }
    });

    //Display
    output.output_line(&"Initiative:");
    output.new_line();
    let mut table: Vec<Vec<String>> = Vec::new();
    for (rolls, name, ini_level) in inis
        .iter()
        .map(|(index, rolls)| (rolls, &characters[*index].0, characters[*index].1))
    {
        let mut row: Vec<String> = vec![
            format!("{}:", name),
            format!("{} ({} + {}/6)", ini_level + rolls[0], ini_level, rolls[0]),
        ];
        for roll in rolls.iter().skip(1) {
            row.push(format!("{}/6", roll));
        }
        table.push(row);
    }

    output.output_table(&table);
    inis
}

fn get_facilitation(matches: &ArgMatches, num_attributes: usize) -> Result<Facilitation, Error> {
    let facilitation = matches.value_of("facilitation").unwrap();
    if facilitation.contains(',') {
        let facilitations: Result<Vec<i64>, ParseIntError> = facilitation
            .split(',')
            .map(|fac| fac.parse::<i64>())
            .collect();
        let facilitations = match facilitations {
            Ok(vec) => vec,
            Err(_) => {
                return Err(Error::new(
                    "Unable to parse facilitation",
                    ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                ));
            }
        };
        if facilitations.len() != num_attributes {
            return Err(Error::new(
                "Number of individual facilitations has to match number of check attributes",
                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
            ));
        }
        Ok(Facilitation::IndividualFacilitation(facilitations))
    } else {
        let facilitation = match facilitation.parse::<i64>() {
            Ok(num) => num,
            Err(_) => {
                return Err(Error::new(
                    "Unable to parse facilitation",
                    ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                ));
            }
        };
        Ok(Facilitation::SimpleFacilitation(facilitation))
    }
}

fn roll_check(
    attributes: &[(&str, i64)],
    check_name: &str,
    character_name: &str,
    facilitation: Facilitation,
    check_type: CheckType,
    crit_type: CritType,
    output: &mut impl OutputWrapper,
) {
    let mut rng = rand::thread_rng();
    let d20 = Uniform::new_inclusive(1, 20);

    //Compute the rolls
    let mut rolls: Vec<i64> = Vec::with_capacity(attributes.len());
    let mut points = match check_type {
        CheckType::SimpleCheck => 0,
        CheckType::PointsCheck(avail_points) => avail_points,
    };
    for ((_, level), facilitation) in attributes.iter().zip(facilitation.iter()) {
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
            output.output_line(&format!(
                "{}, Check for \"{}\"",
                character_name,
                uppercase_first(check_name)
            ));
        }
        CheckType::PointsCheck(avail_points) => {
            output.output_line(&format!(
                "{}, Check for {} (level {})",
                character_name,
                uppercase_first(check_name),
                avail_points
            ));
        }
    };
    output.new_line();

    let mut table: Vec<Vec<String>> = Vec::with_capacity(2);

    let mut header: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    header.push(String::from(""));
    header.extend(attributes.iter().map(|(name, _)| uppercase_first(name)));
    table.push(header);

    let mut char_row: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    char_row.push(String::from("Character:"));
    char_row.extend(attributes.iter().zip(facilitation.iter()).map(
        |((_, level), facilitation)| {
            if *facilitation == 0 {
                level.to_string()
            } else if *facilitation > 0 {
                format!("{} + {}", level, facilitation)
            } else {
                format!("{} - {}", level, -facilitation)
            }
        },
    ));
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
        output.output_line(&"Check failed");
    } else {
        match check_type {
            CheckType::SimpleCheck => {
                output.output_line(&"Check passed");
            }
            CheckType::PointsCheck(_) => {
                let mut quality: u32 = (points as f32 / 3f32).ceil() as u32;
                if quality == 0 {
                    quality = 1;
                } else if quality > 6 {
                    quality = 6;
                }
                output.output_line(&format!("Check passed, quality level {}", quality));
            }
        }
    }

    if crits {
        if crit_succ == 1 {
            output.output_line(&"Critical success");
        } else if crit_succ > 1 {
            output.output_line(&format!("{} critical successes", crit_succ));
        }
        if unconfirmed_crit_succ == 1 {
            output.output_line(&"Unconfirmed critical success");
        } else if unconfirmed_crit_succ > 1 {
            output.output_line(&format!(
                "{} unconfirmed critical successes",
                unconfirmed_crit_succ
            ));
        }
        if crit_fail == 1 {
            output.output_line(&"Critical failure");
        } else if crit_fail > 1 {
            output.output_line(&format!("{} critical failures", crit_fail));
        }
        if unconfirmed_crit_fail == 1 {
            output.output_line(&"Unconfirmed critical failure");
        } else if unconfirmed_crit_fail > 1 {
            output.output_line(&format!(
                "{} unconfirmed critical failures",
                unconfirmed_crit_fail
            ));
        }
    }
}
