use std::cmp::max;

use super::character::Character;
use super::config::{self, Config, DSAData};
use super::util::*;
use clap::ArgMatches;
use itertools::Itertools;
use rand::distributions::{Distribution, Uniform};
use rand::Rng;

//The maximum number of dice in a roll expression
const MAX_NUM_DICE: u32 = 100;
//The maximum number of expressions in a roll command
const MAX_ROLL_EXPRESSIONS: u32 = 20;

const DICE_CHARS: [char; 2] = ['d', 'w'];

enum CheckType {
    //A simple check where you have to roll below your attributes (for example an attribute check)
    SimpleCheck,
    //A check where you can compensate for higher rolls with some available points
    PointsCheck(i64),
}

enum CritType {
    //A check without critical successes or failures
    None,
    //A check where crits have to be confirmed with a second roll
    Confirmable,
    //A check where some number of 1 (or 20) rolls are required to trigger a crit (this number can also be 1)
    MultipleRequired(u32),
}

// The facilitation for a skill check
struct Facilitation {
    // The facilitation for the individual attributes
    individual_facilitation: Vec<i64>,
    // The bonus to the available points/level, only applies for a PointsCheck
    points_bonus: i64,
    // Like points_bonus, but is only applied if the check was already a success before.
    // The second entry gives a reason for this bonus
    successful_points_bonus: Option<SuccessfulPointsBonus>,
}
struct SuccessfulPointsBonus {
    bonus: i64,
    reason: &'static str,
}
// enum Facilitation {
//     SimpleFacilitation(i64),
//     IndividualFacilitation(Vec<i64>),
// }

pub fn attribute_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    _: &Config,
    output: &mut impl OutputWrapper,
) {
    let (attr_name, attr_info) = match DSAData::match_search(
        dsa_data.attributes.iter(),
        cmd_matches.value_of("attribute_name").unwrap(),
    ) {
        Ok(name) => name,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let facilitation = match get_facilitation(cmd_matches, &[attr_name], None) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let attr = vec![(
        attr_info.short_name.as_str(),
        character.get_attribute_level(attr_name),
    )];
    roll_check(
        &attr,
        attr_name,
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::Confirmable,
        output,
    );
}

pub fn talent_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    config: &Config,
    output: &mut impl OutputWrapper,
) {
    let (talent_name, talent_entry) = match DSAData::match_search(
        dsa_data.talents.iter(),
        cmd_matches.value_of("skill_name").unwrap(),
    ) {
        Ok(name) => name,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let skill_attrs = &talent_entry.attributes;
    let facilitation = match get_facilitation(cmd_matches, skill_attrs, None) {
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
    let skill_level = character.get_skill_level(talent_name);

    let crit_type = match config.dsa_rules.crit_rules {
        config::ConfigDSACritType::None => CritType::None,
        config::ConfigDSACritType::Default => CritType::MultipleRequired(2),
        config::ConfigDSACritType::Alternative => CritType::Confirmable,
    };

    roll_check(
        &attrs,
        talent_name,
        character.get_name(),
        facilitation,
        CheckType::PointsCheck(skill_level),
        crit_type,
        output,
    );
    if let Some(application) = character.get_skill_specialization_application(talent_name) {
        output.output_line(&format!(
            "{} has a specialization for the application \"{}\",",
            character.get_name(),
            application
        ));
        output.output_line(&"If this is relevant, 2 extra points can be added to the check.");
    }
}

pub fn attack_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    _: &Config,
    output: &mut impl OutputWrapper,
) {
    let (technique_name, ranged) = match DSAData::match_search(
        dsa_data
            .combat_techniques
            .iter()
            .map(|(k, entry)| (k, entry.ranged))
            // Improve: Whether a technique is ranged or not should be retrievable
            // from the json file for custom techniques.
            .chain(character.get_custom_techniques().map(|t| (t, false))),
        cmd_matches.value_of("technique_name").unwrap(),
    ) {
        Ok(t) => t,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let facilitation = match get_facilitation(cmd_matches, &["attack"], None) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };

    let attack_level = character.get_attack_level(technique_name, ranged);
    roll_check(
        &[(technique_name, attack_level)],
        &format!("Attack: {}", technique_name),
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::Confirmable,
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
    let (spell_name, spell_attrs) = match DSAData::match_search(
        dsa_data
            .spells
            .iter()
            .map(|(k, v)| (k, &v.attributes))
            .chain(character.get_custom_spells()),
        cmd_matches.value_of("spell_name").unwrap(),
    ) {
        Ok(s) => s,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let successful_points_bonus = if character.is_favorite_spell(spell_name) {
        Some(SuccessfulPointsBonus {
            bonus: 2,
            reason: "favorite spell",
        })
    } else {
        None
    };
    let facilitation = match get_facilitation(cmd_matches, spell_attrs, successful_points_bonus) {
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
    let skill_level = character.get_spell_level(spell_name);

    let crit_type = match config.dsa_rules.crit_rules {
        config::ConfigDSACritType::None => CritType::None,
        config::ConfigDSACritType::Default => CritType::MultipleRequired(2),
        config::ConfigDSACritType::Alternative => CritType::Confirmable,
    };

    roll_check(
        &attrs,
        spell_name,
        character.get_name(),
        facilitation,
        CheckType::PointsCheck(skill_level),
        crit_type,
        output,
    );
}

pub fn chant_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    config: &Config,
    output: &mut impl OutputWrapper,
) {
    let (chant_name, chant_attrs) = match DSAData::match_search(
        dsa_data
            .chants
            .iter()
            .map(|(k, v)| (k, &v.attributes))
            .chain(character.get_custom_chants()),
        cmd_matches.value_of("chant_name").unwrap(),
    ) {
        Ok(r) => r,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let successful_points_bonus = if character.is_favorite_incantation(chant_name) {
        Some(SuccessfulPointsBonus {
            bonus: 2,
            reason: "favorite incantation",
        })
    } else {
        None
    };
    let facilitation = match get_facilitation(cmd_matches, chant_attrs, successful_points_bonus) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };

    let attrs: Vec<(&str, i64)> = chant_attrs
        .iter()
        .map(|attr| {
            (
                dsa_data.get_attr_short_name(attr),
                character.get_attribute_level(attr),
            )
        })
        .collect();
    let skill_level = character.get_chant_level(chant_name);

    let crit_type = match config.dsa_rules.crit_rules {
        config::ConfigDSACritType::None => CritType::None,
        config::ConfigDSACritType::Default => CritType::MultipleRequired(2),
        config::ConfigDSACritType::Alternative => CritType::Confirmable,
    };

    roll_check(
        &attrs,
        chant_name,
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
    _: &DSAData,
    _: &Config,
    output: &mut impl OutputWrapper,
) {
    let facilitation = match get_facilitation(cmd_matches, &["dodge"], None) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let dodge_level = character.get_dodge_level();
    roll_check(
        &[("Dodge", dodge_level)],
        "Dodge",
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::Confirmable,
        output,
    );
}

pub fn parry_check(
    cmd_matches: &ArgMatches,
    character: &Character,
    dsa_data: &DSAData,
    _: &Config,
    output: &mut impl OutputWrapper,
) {
    let (technique_name, technique_entry) = match DSAData::match_search(
        dsa_data.combat_techniques.iter(),
        cmd_matches.value_of("technique_name").unwrap(),
    ) {
        Err(e) => {
            output.output_line(&e);
            return;
        }
        Ok(r) => r,
    };
    let facilitation = match get_facilitation(cmd_matches, &["parry"], None) {
        Ok(f) => f,
        Err(e) => {
            output.output_line(&e);
            return;
        }
    };
    let parry_level = character.get_parry_level(technique_name, &technique_entry.attributes);
    roll_check(
        &[("Parry", parry_level)],
        &format!("Parry: {}", technique_name),
        character.get_name(),
        facilitation,
        CheckType::SimpleCheck,
        CritType::Confirmable,
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
                beg.push(' ');
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
        if term.contains(DICE_CHARS) {
            let split: Vec<&str> = term
                .split(DICE_CHARS)
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

            if die_type == 0 {
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
        let end_idx = match expr[begin_idx + 1..].find(['+', '-']) {
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
    let mut ini_information: Vec<(usize, Vec<i64>)> = Vec::with_capacity(characters.len());
    for (i, c) in characters.iter().enumerate() {
        ini_information.push((i, vec![c.1 + d6.sample(&mut rng)]));
    }

    //Roll additional dice for characters that have equal INI and rolls
    fn compute_ini_order(
        mut comp_characters: Vec<(usize, i64)>,
        ini_information: &mut [(usize, Vec<i64>)],
        characters: &[(String, i64)],
        d6: &Uniform<i64>,
    ) {
        comp_characters.sort_by(|c1, c2| c1.1.cmp(&c2.1));
        for (_, group) in &comp_characters.into_iter().group_by(|c| c.1) {
            let mut idxs: Vec<(usize, i64)> = group.into_iter().collect();
            if idxs.len() < 2 {
                continue;
            }
            for (idx, last) in idxs.iter_mut() {
                let value = if ini_information[*idx].1.len() == 1 {
                    characters[*idx].1
                } else {
                    d6.sample(&mut rand::thread_rng())
                };
                ini_information[*idx].1.push(value);
                *last = value;
            }
            compute_ini_order(idxs, ini_information, characters, d6);
        }
    }

    compute_ini_order(
        ini_information
            .iter()
            .map(|(i, arr)| (*i, arr[0]))
            .collect(),
        &mut ini_information,
        characters,
        &d6,
    );

    //Reverse sort
    ini_information.sort_by(|(_, vals1), (_, vals2)| vals2.cmp(vals1));

    //Display
    output.output_line(&"Initiative:");
    output.new_line();
    let mut table: Vec<Vec<String>> = Vec::new();
    for (rolls, name, ini_level) in ini_information
        .iter()
        .map(|(index, rolls)| (rolls, &characters[*index].0, characters[*index].1))
    {
        let mut row: Vec<String> = vec![
            format!("{}:", name),
            format!("{} ({} + {}/6)", rolls[0], ini_level, rolls[0] - ini_level),
        ];
        for roll in rolls.iter().skip(1) {
            row.push(format!("{}", roll));
        }
        table.push(row);
    }

    output.output_table(&table);
    ini_information
}

fn get_facilitation<S>(
    matches: &ArgMatches,
    attributes: &[S],
    successful_points_bonus: Option<SuccessfulPointsBonus>,
) -> Result<Facilitation, Error>
where
    S: AsRef<str>,
{
    let flat_facilitation: i64 = match matches.value_of("facilitation").unwrap().parse() {
        Ok(f) => f,
        Err(_) => {
            return Err(Error::new(
                "Unable to parse facilitation: Argument must be an integer",
                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
            ));
        }
    };
    let mut individual_facilitation = vec![flat_facilitation; attributes.len()];
    if let Some(m) = matches.value_of("attribute_facilitation") {
        for modifier in m.split(',') {
            let split: Vec<_> = modifier.split(':').collect();
            if split.len() != 2 {
                return Err(Error::new("Unable to parse facilitation: Attribute name and facilitation must be separated by a colon", ErrorType::InvalidInput(InputErrorType::InvalidArgument)));
            }
            let amount: i64 = match split[1].parse() {
                Ok(a) => a,
                Err(_) => {
                    return Err(Error::new(
                        "Unable to parse facilitation: Invalid attribute facilitation amount",
                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                    ));
                }
            };
            for (i, attr) in attributes.iter().enumerate() {
                if attr.as_ref().eq_ignore_ascii_case(split[0]) {
                    individual_facilitation[i] += amount;
                }
            }
        }
    };
    let points_bonus = match matches.value_of("bonus_points").map(|b| b.parse()) {
        None => 0,
        Some(Ok(p)) => p,
        Some(Err(_)) => {
            return Err(Error::new(
                "Unable to parse facilitation: bonus-points must be an integer",
                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
            ));
        }
    };
    Ok(Facilitation {
        individual_facilitation,
        points_bonus,
        successful_points_bonus,
    })
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
        CheckType::PointsCheck(avail_points) => max(0, avail_points + facilitation.points_bonus),
    };
    for ((_, level), facilitation) in attributes
        .iter()
        .zip(facilitation.individual_facilitation.iter())
    {
        let roll = d20.sample(&mut rng);
        if roll != 1 {
            points -= max(0, roll - (level + facilitation));
        }
        rolls.push(roll);
    }
    //Check for crits
    let (crit_succ, crit_fail, crit_row) = match crit_type {
        CritType::None => (false, false, None),
        CritType::Confirmable => {
            let mut crit_row: Vec<String> = vec![String::from("Crit roll:")];
            let mut crit_succ = false;
            let mut crit_fail = false;
            let mut rolled_for_crit = false;
            for ((_, level), &roll) in attributes.iter().zip(rolls.iter()) {
                if roll == 1 {
                    let crit_roll = d20.sample(&mut rng);
                    crit_row.push(crit_roll.to_string());
                    rolled_for_crit = true;
                    if crit_roll <= *level {
                        crit_succ = true;
                    }
                } else if roll == 20 {
                    let crit_roll = d20.sample(&mut rng);
                    crit_row.push(crit_roll.to_string());
                    rolled_for_crit = true;
                    if crit_roll > *level {
                        crit_fail = true;
                    }
                } else {
                    crit_row.push(String::from(""));
                }
            }
            let crit_row = if rolled_for_crit {
                Some(crit_row)
            } else {
                None
            };
            (crit_succ, crit_fail, crit_row)
        }
        CritType::MultipleRequired(num_required) => {
            let crit_succ = rolls.iter().filter(|&&r| r == 1).count() >= num_required as usize;
            let crit_fail = rolls.iter().filter(|&&r| r == 20).count() >= num_required as usize;
            (crit_succ, crit_fail, None)
        }
    };

    //Output
    match check_type {
        CheckType::SimpleCheck => {
            output.output_line(&format!(
                "{}, Check for {}",
                character_name,
                uppercase_first(check_name)
            ));
        }
        CheckType::PointsCheck(avail_points) => {
            let level = match facilitation.points_bonus {
                0 => avail_points.to_string(),
                bonus => format!("{} + {}", avail_points, bonus),
            };
            output.output_line(&format!(
                "{}, Check for {} (level {})",
                character_name,
                uppercase_first(check_name),
                level
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
    char_row.extend(
        attributes
            .iter()
            .zip(facilitation.individual_facilitation.iter())
            .map(|((_, level), facilitation)| {
                if *facilitation == 0 {
                    level.to_string()
                } else if *facilitation > 0 {
                    format!("{} + {}", level, facilitation)
                } else {
                    format!("{} - {}", level, -facilitation)
                }
            }),
    );
    table.push(char_row);

    let mut rolls_row: Vec<String> = Vec::with_capacity(attributes.len() + 1);
    rolls_row.push(String::from("Roll:"));
    rolls_row.extend(rolls.iter().map(|roll| roll.to_string()));
    table.push(rolls_row);

    if let Some(crit_row) = crit_row {
        table.push(crit_row);
    }
    output.output_table(&table);
    output.new_line();

    let passed = (points >= 0 && !crit_fail) || crit_succ;
    if passed {
        if let Some(b) = facilitation.successful_points_bonus {
            output.output_line(&format!(
                "Adding {} extra points to the successful check ({})",
                b.bonus, b.reason
            ));
            points += b.bonus;
        }
        match check_type {
            CheckType::SimpleCheck => {
                output.output_line(&"Check passed");
            }
            CheckType::PointsCheck(_) => {
                let mut quality: u32 = (max(points, 0) as f32 / 3f32).ceil() as u32;
                if quality == 0 {
                    quality = 1;
                } else if quality > 6 {
                    quality = 6;
                }
                output.output_line(&format!("Check passed, quality level {}", quality));
            }
        }
    } else {
        output.output_line(&"Check failed");
    }

    if crit_succ {
        output.output_line(&"Critical success");
    }
    if crit_fail {
        output.output_line(&"Critical failure");
    }
}
