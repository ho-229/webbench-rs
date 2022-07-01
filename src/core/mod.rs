pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub mod webbench;
pub mod protocol;

pub use self::webbench::{Config, Webbench};
