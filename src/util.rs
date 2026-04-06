use std::fmt::{self};

pub fn uppercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

pub trait OutputWrapper {
    fn output(&mut self, msg: &impl fmt::Display);
    fn output_line(&mut self, msg: &impl fmt::Display);
    fn new_line(&mut self);

    //Prints  a formatted table given a vector of its rows (note that any headers must simply be passed as rows/columns)
    fn output_table(&mut self, table: &[Vec<String>]);
}

pub struct CLIOutputWrapper;
impl OutputWrapper for CLIOutputWrapper {
    fn output(&mut self, msg: &impl fmt::Display) {
        print!("{}", msg);
    }
    fn output_line(&mut self, msg: &impl fmt::Display) {
        println!("{}", msg);
    }
    fn new_line(&mut self) {
        println!();
    }

    fn output_table(&mut self, table: &[Vec<String>]) {
        for row in table {
            for entry in row {
                print!("{:<22}", entry);
            }
            println!();
        }
    }
}
