use std::fmt;

pub struct Error {
    message: String,
    err_type: ErrorType,
}

pub enum ErrorType {
    Unknown,
    InvalidFormat,
    InvalidArgument,
    MissingEnvironmentVariable,
    FileSystemError,
}

impl Error {
    pub fn from_str(message: &str, err_type: ErrorType) -> Error {
        Error {
            message: String::from(message),
            err_type,
        }
    }

    pub fn from_string(message: String, err_type: ErrorType) -> Error {
        Error { message, err_type }
    }

    pub fn message<'a>(&'a self) -> &'a str {
        &self.message
    }

    pub fn err_type(&self) -> &ErrorType {
        &self.err_type
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
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
    fn output(&mut self, msg: &impl fmt::Display);
    fn output_line(&mut self, msg: &impl fmt::Display);
    fn new_line(&mut self);

    //Prints  a formatted table given a vector of its rows (note that any headers must simply be passed as rows/columns)
    fn output_table(&mut self, table: &Vec<Vec<String>>);
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

    fn output_table(&mut self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                print!("{:<22}", entry);
            }
            println!();
        }
    }
}
