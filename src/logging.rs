use std::env;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;

use chrono::offset::Local;

const LOG_FILE_LOCATION: &str = "tag-bot.log";

pub type Result<T> = std::result::Result<T, BotError>;

pub trait GracefulUnwrap<T> {
    fn unwrap_gracefully(self) -> T;
}

impl<T> GracefulUnwrap<T> for Result<T> {
    fn unwrap_gracefully(self) -> T {
        match self {
            Ok(val) => val,
            Err(e) => {
                e.log();
                panic!();
            }
        }
    }
}

/// This macro generates variants for the BotError enum.
/// It also implements the following traits:
///  - GracefulUnwrap for Result<T, E>
///  - From<E> -> BotError
/// , where E are the original error types.
macro_rules! make_errors {
    ($($name:ident => $error:path),*) => {
        #[derive(Debug)]
        pub enum BotError {
            $(
                $name($error),
            )*
        }

        $(impl From<$error> for BotError {
            fn from(e: $error) -> Self {
                BotError::$name(e)
            }
        })*

        $(impl<T> GracefulUnwrap<T> for std::result::Result<T, $error>  {
            fn unwrap_gracefully(self) -> T {
                match self {
                    Ok(val) => val,
                    Err(e) => {
                        BotError::$name(e).log();
                        panic!();
                    }
                }
            }
        })*
    }
}

make_errors! {
    Io => std::io::Error,
    Toml => toml::de::Error,
    Discord => serenity::Error
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            BotError::Io(e) => {
                let path = env::current_dir();
                if path.is_err() {
                    write!(f, "Error getting info about current folder! Make sure you have the correct permissions. {}", e)
                } else {
                    write!(
                        f,
                        "Error reading file. Looking in path \"{}\". {}",
                        path.unwrap().display(),
                        e
                    )
                }
            }
            BotError::Toml(e) => write!(f, "Error parsing configuration file! {}", e),
            BotError::Discord(e) => write!(f, "Error communicating with Discord API! {}", e),
        }
    }
}

pub trait Log {
    fn log(&self);
}

impl Log for BotError {
    fn log(&self) {
        let time = Local::now();
        let timestamp = time.format("%e %b %Y %r %Z");
        let log_entry = format! {"{} ERROR: {}", timestamp, &self};

        eprintln!("{}", log_entry);
        _append_log_file(log_entry);
    }
}

impl Log for String {
    fn log(&self) {
        let time = Local::now();
        let timestamp = time.format("%e %b %Y %r %Z");
        let log_entry = format! {"{} INFO: {}", timestamp, &self};

        println!("{}", log_entry);
        _append_log_file(log_entry);
    }
}

impl<T> Log for Result<T> {
    fn log(&self) {
        match &self {
            Err(e) => e.log(),
            Ok(_) => (),
        }
    }
}

fn _append_log_file(log_entry: String) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(LOG_FILE_LOCATION);

    match file {
        Ok(mut f) => {
            writeln!(f, "{}", log_entry).expect("Log file opened but could not write to it!!!")
        }
        Err(e) => {
            eprintln!("Could not open log file: {}", e);
        }
    };
}
