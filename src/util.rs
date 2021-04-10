pub struct IOError {
    message: String,
    err_type: IOErrorType,
}

pub enum IOErrorType {
    Unknown,
    InvalidFormat,
    MissingEnvironmentVariable,
    FileSystemError,
}

impl IOError {
    pub fn from_str(message: &str, err_type: IOErrorType) -> IOError {
        IOError {
            message: String::from(message),
            err_type,
        }
    }

    pub fn from_string(message: String, err_type: IOErrorType) -> IOError {
        IOError { message, err_type }
    }

    pub fn message<'a>(&'a self) -> &'a str {
        &self.message
    }

    pub fn err_type(&self) -> &IOErrorType {
        &self.err_type
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
    fn output(&mut self, msg: String);
    fn output_line(&mut self, msg: String);
    fn new_line(&mut self);

    //Prints  a formatted table given a vector of its rows (note that any headers must simply be passed as rows/columns)
    fn output_table(&mut self, table: &Vec<Vec<String>>);
}

pub struct CLIOutputWrapper;
impl OutputWrapper for CLIOutputWrapper {
    fn output(&mut self, msg: String) {
        print!("{}", msg);
    }
    fn output_line(&mut self, msg: String) {
        println!("{}", msg);
    }
    fn new_line(&mut self) {
        println!();
    }

    fn output_table(&mut self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                print!("{:<17}", entry);
            }
            println!();
        }
    }
}
