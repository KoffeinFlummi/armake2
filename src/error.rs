pub trait PrintableError<T, E> {
    fn unwrap_or_print(self) -> T;
}
impl<T, E: std::fmt::Debug + std::fmt::Display> PrintableError<T, E> for Result<T, E> {
    fn unwrap_or_print(self) -> T {
        if let Err(error) = &self {
            println!("{}", error);
            std::process::exit(1);
        }
        self.unwrap()
    }
}

#[derive(Debug)]
pub struct IOPathError {
    pub source: std::io::Error,
    pub path: std::path::PathBuf,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct PreprocessParseError {
    pub path: Option<String>,
    pub message: String,
    pub source: crate::preprocess::preprocess_grammar::ParseError,
}

#[derive(Debug)]
pub struct PreprocessError {
    pub path: Option<String>,
    pub message: String,
    pub source: Box<ArmakeError>,
}

#[derive(Debug)]
pub struct ConfigParseError {
    pub path: Option<String>,
    pub message: String,
    pub source: crate::config::config_grammar::ParseError,
}

#[derive(Debug)]
pub enum ArmakeError {
    GENERIC(String),
    MESSAGE(String, Box<ArmakeError>),
    CONFIG(ConfigParseError),
    PARSE(PreprocessParseError),
    PREPROCESS(PreprocessError),
    IO(std::io::Error),
    IOPath(IOPathError),
}

#[macro_export]
macro_rules! error {
    ($e:expr) => {
        ArmakeError::GENERIC($e.to_string())
    };
    ($e:expr, $($p:expr),*) => {
        error!(format!($e, $($p,)*))
    };
}

impl std::fmt::Display for ArmakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ArmakeError::GENERIC(ref s) => write!(f, "{}", s),
            ArmakeError::MESSAGE(ref s, ref _e) => write!(f, "{}", s),
            ArmakeError::CONFIG(ref e) => write!(f, "Config: {}", e.message),
            ArmakeError::PARSE(ref e) => write!(f, "Preprocessor Parse: {}", e.message),
            ArmakeError::PREPROCESS(ref e) => write!(f, "Preprocessor: {}", e.message),
            ArmakeError::IO(ref e) => write!(f, "IO error: {}", e),
            ArmakeError::IOPath(ref e) => write!(f, "IO error: `{:#?}`\n{}", e.path, e.source),
        }
    }
}

impl std::error::Error for ArmakeError {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            ArmakeError::GENERIC(ref _s,) => Some(self),
            ArmakeError::MESSAGE(ref _s, ref e) => Some(e),
            ArmakeError::CONFIG(ref e) => Some(&e.source),
            ArmakeError::PARSE(ref e) => Some(&e.source),
            ArmakeError::PREPROCESS(ref e) => Some(&e.source),
            ArmakeError::IO(ref e) => Some(e),
            ArmakeError::IOPath(ref e) => Some(&e.source),
        }
    }
}

impl From<std::io::Error> for ArmakeError {
    fn from(err: std::io::Error) -> ArmakeError {
        ArmakeError::IO(err)
    }
}
