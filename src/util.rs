use std::fmt::{self, Display};

pub struct Error {
    message: String,
    err_type: ErrorType,
}

#[derive(Display)]
pub enum ErrorType {
    Unknown,
    InvalidInput(InputErrorType),
    IO(IOErrorType),
}

#[derive(Display)]
pub enum IOErrorType {
    Unknown,
    MissingEnvironmentVariable,
    MissingFile,
    Discord,
    UnknownInteractionType,
}

#[derive(Display)]
pub enum InputErrorType {
    InvalidFormat,
    InvalidArgument,
    InvalidDiscordContext,
    MissingCharacter,
    CharacterNameTooLong,
    TooManyCharacters,
}

impl Error {
    pub fn new<S: Into<String>>(message: S, err_type: ErrorType) -> Error {
        Error {
            message: message.into(),
            err_type,
        }
    }

    pub fn message<'a>(&'a self) -> &'a str {
        &self.message
    }

    pub fn err_type(&self) -> &ErrorType {
        &self.err_type
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error {
            message: e.to_string(),
            err_type: ErrorType::IO(IOErrorType::Unknown),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error {
            message: e.to_string(),
            err_type: ErrorType::InvalidInput(InputErrorType::InvalidFormat),
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(e: serenity::Error) -> Error {
        Error {
            message: e.to_string(),
            err_type: ErrorType::IO(IOErrorType::Discord),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error {{ message: {}, err_type: {} }}",
            self.message(),
            self.err_type()
        )
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
