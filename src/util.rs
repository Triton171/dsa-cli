pub struct IOError {
    message: String,
}

impl IOError {
    pub fn from_str(message: &str) -> IOError {
        IOError {
            message: String::from(message),
        }
    }

    pub fn from_string(message: String) -> IOError {
        IOError { message }
    }

    pub fn message<'a>(&'a self) -> &'a str {
        &self.message
    }
}

pub fn uppercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

pub trait OutputWrapper {
    fn output_line(&self, line: String);
    fn new_line(&self);

    //Prints  a formatted table given a vector of its rows (note that any headers must simply be passed as rows/columns)
    fn output_table(&self, table: &Vec<Vec<String>>);
}

pub struct CLIOutputWrapper;
impl OutputWrapper for CLIOutputWrapper {
    fn output_line(&self, line: String) {
        println!("{}", line);
    }
    fn new_line(&self) {
        println!();
    }

    fn output_table(&self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                print!("{:<17}", entry);
            }
            println!();
        }
    }
}
