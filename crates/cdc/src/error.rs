use std::fmt;
use std::io;
use std::sync::PoisonError;

use serde::de::Visitor;
use serde::ser::{Serialize, Serializer};
use serde::{Deserialize, Deserializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Dapr API error: {0}")]
    DaprApi(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("{0}")]
    General(String),
}

// From implementations
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::General(err.to_string())
    }
}

impl From<&str> for Error {
    fn from(arg: &str) -> Self {
        Error::General(arg.to_string())
    }
}

impl From<String> for Error {
    fn from(arg: String) -> Self {
        Error::General(arg)
    }
}

impl From<&dyn std::error::Error> for Error {
    fn from(arg: &dyn std::error::Error) -> Self {
        Error::General(arg.to_string())
    }
}

impl From<Error> for std::io::Error {
    fn from(arg: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, arg.to_string())
    }
}

impl From<rbdc::Error> for Error {
    fn from(arg: rbdc::Error) -> Self {
        Error::Database(arg.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(arg: serde_json::Error) -> Self {
        Error::Parse(arg.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(arg: reqwest::Error) -> Self {
        Error::DaprApi(arg.to_string())
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(arg: PoisonError<T>) -> Self {
        Error::General(arg.to_string())
    }
}

// Keep custom Serialize (serializes all variants as string)
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

struct ErrorVisitor;

impl<'de> Visitor<'de> for ErrorVisitor {
    type Value = String;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }
    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where E: std::error::Error {
        Ok(v)
    }
    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where E: std::error::Error {
        Ok(v.to_string())
    }
}

impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: Deserializer<'de> {
        let r = deserializer.deserialize_string(ErrorVisitor)?;
        Ok(Error::General(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error() {
        let e = Error::from("test error");
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("test error"));
    }
}
