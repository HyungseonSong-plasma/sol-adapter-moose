#![forbid(unsafe_code)]

mod model;
mod realization_v2;
mod validation;

pub use model::*;
pub use realization_v2::*;
pub use validation::*;

use serde_json::{Map, Value};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const PUBLIC_CONTRACT_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractVersion {
    major: u64,
    minor: u64,
}

impl ContractVersion {
    pub const fn new(major: u64, minor: u64) -> Self {
        Self { major, minor }
    }

    pub const fn supported() -> Self {
        Self::new(0, 1)
    }

    pub const fn major(self) -> u64 {
        self.major
    }

    pub const fn minor(self) -> u64 {
        self.minor
    }

    pub fn parse(raw: &str) -> Result<Self, VersionSyntaxError> {
        let (major, minor) = raw
            .split_once('.')
            .ok_or_else(|| VersionSyntaxError(raw.to_owned()))?;

        if minor.contains('.') || !is_canonical_decimal(major) || !is_canonical_decimal(minor) {
            return Err(VersionSyntaxError(raw.to_owned()));
        }

        let major = major
            .parse::<u64>()
            .map_err(|_| VersionSyntaxError(raw.to_owned()))?;
        let minor = minor
            .parse::<u64>()
            .map_err(|_| VersionSyntaxError(raw.to_owned()))?;

        Ok(Self::new(major, minor))
    }
}

impl Display for ContractVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSyntaxError(pub String);

impl Display for VersionSyntaxError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "malformed public contract version: {}", self.0)
    }
}

impl Error for VersionSyntaxError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractDocumentError {
    InvalidJson(String),
    RootNotObject,
    MissingVersion,
    VersionNotString,
    MalformedVersion(String),
    UnsupportedVersion(String),
}

impl Display for ContractDocumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(detail) => {
                write!(formatter, "invalid public contract JSON: {detail}")
            }
            Self::RootNotObject => {
                write!(formatter, "public contract document root must be an object")
            }
            Self::MissingVersion => write!(
                formatter,
                "public contract document is missing public_contract_version"
            ),
            Self::VersionNotString => {
                write!(formatter, "public_contract_version must be a string")
            }
            Self::MalformedVersion(version) => {
                write!(formatter, "malformed public contract version: {version}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported public contract version: {version}")
            }
        }
    }
}

impl Error for ContractDocumentError {}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalDocument {
    value: Value,
    version: ContractVersion,
}

impl CanonicalDocument {
    pub fn parse(input: &str) -> Result<Self, ContractDocumentError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ContractDocumentError::InvalidJson(error.to_string()))?;
        let object = value
            .as_object()
            .ok_or(ContractDocumentError::RootNotObject)?;
        let raw_version = object
            .get("public_contract_version")
            .ok_or(ContractDocumentError::MissingVersion)?;
        let raw_version = raw_version
            .as_str()
            .ok_or(ContractDocumentError::VersionNotString)?;
        let version = ContractVersion::parse(raw_version)
            .map_err(|_| ContractDocumentError::MalformedVersion(raw_version.to_owned()))?;

        if version != ContractVersion::supported() {
            return Err(ContractDocumentError::UnsupportedVersion(
                raw_version.to_owned(),
            ));
        }

        Ok(Self { value, version })
    }

    pub fn version(&self) -> ContractVersion {
        self.version
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&canonicalize_value(self.value.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedEnumError {
    pub value: String,
}

impl Display for ClosedEnumError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown closed-enum value: {}", self.value)
    }
}

impl Error for ClosedEnumError {}

pub fn require_closed_enum_value<'a>(
    value: &'a str,
    allowed: &[&str],
) -> Result<&'a str, ClosedEnumError> {
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(ClosedEnumError {
            value: value.to_owned(),
        })
    }
}

fn is_canonical_decimal(component: &str) -> bool {
    !component.is_empty()
        && component.bytes().all(|byte| byte.is_ascii_digit())
        && (component == "0" || !component.starts_with('0'))
}

fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<_> = object.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));

            let mut canonical = Map::new();
            for (key, value) in entries {
                canonical.insert(key, canonicalize_value(value));
            }
            Value::Object(canonical)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize_value).collect()),
        scalar => scalar,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContractVersion, VersionSyntaxError};

    #[test]
    fn supported_version_is_exact_major_minor() {
        assert_eq!(
            ContractVersion::parse("0.1").unwrap(),
            ContractVersion::supported()
        );
        assert_eq!(ContractVersion::supported().to_string(), "0.1");
    }

    #[test]
    fn patch_component_is_not_silently_accepted() {
        assert_eq!(
            ContractVersion::parse("0.1.0"),
            Err(VersionSyntaxError("0.1.0".to_owned()))
        );
    }

    #[test]
    fn noncanonical_decimal_component_is_rejected() {
        assert!(ContractVersion::parse("00.1").is_err());
        assert!(ContractVersion::parse("0.01").is_err());
        assert!(ContractVersion::parse(" 0.1 ").is_err());
    }
}
