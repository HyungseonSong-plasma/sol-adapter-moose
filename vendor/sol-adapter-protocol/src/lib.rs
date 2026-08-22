#![forbid(unsafe_code)]

mod execution;
mod failure;
mod preflight;
pub use execution::*;
pub use failure::*;
pub use preflight::*;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sol_public_contract::ContractVersion;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const ADAPTER_PROTOCOL_VERSION: &str = "0.1";

pub type Extensions = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterProtocolVersion {
    major: u64,
    minor: u64,
}

impl AdapterProtocolVersion {
    pub const fn new(major: u64, minor: u64) -> Self {
        Self { major, minor }
    }

    pub const fn current() -> Self {
        Self::new(0, 1)
    }

    pub const fn major(self) -> u64 {
        self.major
    }

    pub const fn minor(self) -> u64 {
        self.minor
    }

    pub fn parse(raw: &str) -> Result<Self, ProtocolError> {
        let (major, minor) = raw
            .split_once('.')
            .ok_or_else(|| ProtocolError::MalformedProtocolVersion(raw.to_owned()))?;

        if minor.contains('.') || !is_canonical_decimal(major) || !is_canonical_decimal(minor) {
            return Err(ProtocolError::MalformedProtocolVersion(raw.to_owned()));
        }

        let major = major
            .parse::<u64>()
            .map_err(|_| ProtocolError::MalformedProtocolVersion(raw.to_owned()))?;
        let minor = minor
            .parse::<u64>()
            .map_err(|_| ProtocolError::MalformedProtocolVersion(raw.to_owned()))?;

        Ok(Self::new(major, minor))
    }
}

impl Display for AdapterProtocolVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterBootstrap {
    pub adapter_id: String,
    pub adapter_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_adapter_protocol_versions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_public_contract_versions: Option<Vec<String>>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl AdapterBootstrap {
    pub fn normalize(&mut self) -> Result<(), ProtocolError> {
        if !is_namespaced_identifier(&self.adapter_id) {
            return Err(ProtocolError::InvalidAdapterId(self.adapter_id.clone()));
        }
        if self.adapter_version.is_empty() {
            return Err(ProtocolError::EmptyAdapterVersion);
        }
        reject_transport_markers_in_extensions(&self.extensions)?;

        if let Some(versions) = &mut self.supported_adapter_protocol_versions {
            normalize_protocol_versions(versions)?;
        }
        if let Some(versions) = &mut self.supported_public_contract_versions {
            normalize_public_contract_versions(versions)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDeclaration {
    pub capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl CapabilityDeclaration {
    fn normalize(&mut self) -> Result<(), ProtocolError> {
        require_stable_symbol("capability", &self.capability)?;
        if self.revision.as_deref() == Some("") {
            return Err(ProtocolError::EmptyField("capability revision"));
        }
        reject_transport_markers_in_extensions(&self.extensions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetDeclaration {
    pub target: String,
    #[serde(default)]
    pub capabilities: Vec<CapabilityDeclaration>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl TargetDeclaration {
    fn normalize(&mut self) -> Result<(), ProtocolError> {
        require_stable_symbol("target", &self.target)?;
        reject_transport_markers_in_extensions(&self.extensions)?;
        for capability in &mut self.capabilities {
            capability.normalize()?;
        }
        self.capabilities.sort_by(|left, right| {
            left.capability
                .cmp(&right.capability)
                .then_with(|| left.revision.cmp(&right.revision))
                .then_with(|| stable_json(&left.extensions).cmp(&stable_json(&right.extensions)))
        });
        self.capabilities.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterDescription {
    pub bootstrap: AdapterBootstrap,
    #[serde(default)]
    pub targets: Vec<TargetDeclaration>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl AdapterDescription {
    pub fn from_json(input: &str) -> Result<Self, ProtocolError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ProtocolError::InvalidJson(error.to_string()))?;
        reject_transport_markers(&value)?;
        let mut description: Self = serde_json::from_value(value)
            .map_err(|error| ProtocolError::InvalidDescription(error.to_string()))?;
        description.normalize()?;
        Ok(description)
    }

    pub fn to_canonical_json(&self) -> Result<String, ProtocolError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| ProtocolError::InvalidDescription(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| ProtocolError::InvalidDescription(error.to_string()))
    }

    fn normalize(&mut self) -> Result<(), ProtocolError> {
        self.bootstrap.normalize()?;
        reject_transport_markers_in_extensions(&self.extensions)?;
        for target in &mut self.targets {
            target.normalize()?;
        }
        self.targets.sort_by(|left, right| {
            left.target
                .cmp(&right.target)
                .then_with(|| stable_json(left).cmp(&stable_json(right)))
        });
        for pair in self.targets.windows(2) {
            if pair[0].target == pair[1].target {
                return Err(ProtocolError::DuplicateTarget(pair[0].target.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct BootstrapEnvelope {
    bootstrap: AdapterBootstrap,
}

pub fn parse_bootstrap(input: &str) -> Result<AdapterBootstrap, ProtocolError> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| ProtocolError::InvalidJson(error.to_string()))?;
    reject_transport_markers(&value)?;
    let mut envelope: BootstrapEnvelope = serde_json::from_value(value)
        .map_err(|error| ProtocolError::InvalidBootstrap(error.to_string()))?;
    envelope.bootstrap.normalize()?;
    Ok(envelope.bootstrap)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySupport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_protocol_versions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_contract_versions: Option<Vec<String>>,
}

impl CompatibilitySupport {
    pub fn current() -> Self {
        Self {
            adapter_protocol_versions: Some(vec![ADAPTER_PROTOCOL_VERSION.to_owned()]),
            public_contract_versions: Some(vec![
                sol_public_contract::PUBLIC_CONTRACT_VERSION.to_owned()
            ]),
        }
    }

    fn normalized(&self) -> Result<Self, ProtocolError> {
        let mut normalized = self.clone();
        if let Some(versions) = &mut normalized.adapter_protocol_versions {
            normalize_protocol_versions(versions)?;
        }
        if let Some(versions) = &mut normalized.public_contract_versions {
            normalize_public_contract_versions(versions)?;
        }
        Ok(normalized)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityOutcome {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisCompatibility {
    pub outcome: CompatibilityOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityAssessment {
    pub adapter_protocol: AxisCompatibility,
    pub public_contract: AxisCompatibility,
    pub overall: CompatibilityOutcome,
}

pub fn assess_compatibility(
    core: &CompatibilitySupport,
    adapter: &AdapterBootstrap,
) -> Result<CompatibilityAssessment, ProtocolError> {
    let core = core.normalized()?;
    let mut adapter = adapter.clone();
    adapter.normalize()?;

    let adapter_protocol = assess_protocol_axis(
        core.adapter_protocol_versions.as_deref(),
        adapter.supported_adapter_protocol_versions.as_deref(),
    )?;
    let public_contract = assess_public_contract_axis(
        core.public_contract_versions.as_deref(),
        adapter.supported_public_contract_versions.as_deref(),
    )?;
    let overall = overall_outcome(adapter_protocol.outcome, public_contract.outcome);

    Ok(CompatibilityAssessment {
        adapter_protocol,
        public_contract,
        overall,
    })
}

fn assess_protocol_axis(
    left: Option<&[String]>,
    right: Option<&[String]>,
) -> Result<AxisCompatibility, ProtocolError> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(AxisCompatibility {
            outcome: CompatibilityOutcome::Unknown,
            selected_version: None,
        });
    };

    let left = parse_protocol_versions(left)?;
    let right = parse_protocol_versions(right)?;
    let selected = left
        .iter()
        .filter(|version| right.contains(version))
        .max()
        .copied();

    Ok(match selected {
        Some(version) => AxisCompatibility {
            outcome: CompatibilityOutcome::Compatible,
            selected_version: Some(version.to_string()),
        },
        None => AxisCompatibility {
            outcome: CompatibilityOutcome::Incompatible,
            selected_version: None,
        },
    })
}

fn assess_public_contract_axis(
    left: Option<&[String]>,
    right: Option<&[String]>,
) -> Result<AxisCompatibility, ProtocolError> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(AxisCompatibility {
            outcome: CompatibilityOutcome::Unknown,
            selected_version: None,
        });
    };

    let left = parse_public_contract_versions(left)?;
    let right = parse_public_contract_versions(right)?;
    let selected = left
        .iter()
        .filter(|version| right.contains(version))
        .max()
        .copied();

    Ok(match selected {
        Some(version) => AxisCompatibility {
            outcome: CompatibilityOutcome::Compatible,
            selected_version: Some(version.to_string()),
        },
        None => AxisCompatibility {
            outcome: CompatibilityOutcome::Incompatible,
            selected_version: None,
        },
    })
}

fn overall_outcome(
    protocol: CompatibilityOutcome,
    public_contract: CompatibilityOutcome,
) -> CompatibilityOutcome {
    if matches!(protocol, CompatibilityOutcome::Incompatible)
        || matches!(public_contract, CompatibilityOutcome::Incompatible)
    {
        CompatibilityOutcome::Incompatible
    } else if matches!(protocol, CompatibilityOutcome::Compatible)
        && matches!(public_contract, CompatibilityOutcome::Compatible)
    {
        CompatibilityOutcome::Compatible
    } else {
        CompatibilityOutcome::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidJson(String),
    InvalidBootstrap(String),
    InvalidDescription(String),
    MalformedProtocolVersion(String),
    MalformedPublicContractVersion(String),
    InvalidAdapterId(String),
    EmptyAdapterVersion,
    EmptyField(&'static str),
    InvalidSymbol { field: &'static str, value: String },
    DuplicateTarget(String),
    TransportLeakage(String),
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(detail) => {
                write!(formatter, "invalid adapter protocol JSON: {detail}")
            }
            Self::InvalidBootstrap(detail) => {
                write!(formatter, "invalid adapter bootstrap: {detail}")
            }
            Self::InvalidDescription(detail) => {
                write!(formatter, "invalid adapter description: {detail}")
            }
            Self::MalformedProtocolVersion(version) => {
                write!(formatter, "malformed Adapter Protocol version: {version}")
            }
            Self::MalformedPublicContractVersion(version) => {
                write!(formatter, "malformed Public Contract version: {version}")
            }
            Self::InvalidAdapterId(id) => write!(formatter, "invalid adapter id: {id}"),
            Self::EmptyAdapterVersion => write!(formatter, "adapter_version must not be empty"),
            Self::EmptyField(field) => write!(formatter, "{field} must not be empty"),
            Self::InvalidSymbol { field, value } => {
                write!(formatter, "invalid {field} symbol: {value}")
            }
            Self::DuplicateTarget(target) => {
                write!(formatter, "duplicate target declaration: {target}")
            }
            Self::TransportLeakage(field) => write!(
                formatter,
                "transport-specific field is not Adapter Protocol semantic data: {field}"
            ),
        }
    }
}

impl Error for ProtocolError {}

fn normalize_protocol_versions(versions: &mut Vec<String>) -> Result<(), ProtocolError> {
    let parsed = parse_protocol_versions(versions)?;
    *versions = parsed
        .into_iter()
        .map(|version| version.to_string())
        .collect();
    Ok(())
}

fn parse_protocol_versions(
    versions: &[String],
) -> Result<Vec<AdapterProtocolVersion>, ProtocolError> {
    let mut parsed = versions
        .iter()
        .map(|version| AdapterProtocolVersion::parse(version))
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort();
    parsed.dedup();
    Ok(parsed)
}

fn normalize_public_contract_versions(versions: &mut Vec<String>) -> Result<(), ProtocolError> {
    let parsed = parse_public_contract_versions(versions)?;
    *versions = parsed
        .into_iter()
        .map(|version| version.to_string())
        .collect();
    Ok(())
}

fn parse_public_contract_versions(
    versions: &[String],
) -> Result<Vec<ContractVersion>, ProtocolError> {
    let mut parsed = versions
        .iter()
        .map(|version| {
            ContractVersion::parse(version)
                .map_err(|_| ProtocolError::MalformedPublicContractVersion(version.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort();
    parsed.dedup();
    Ok(parsed)
}

fn is_canonical_decimal(component: &str) -> bool {
    !component.is_empty()
        && component.bytes().all(|byte| byte.is_ascii_digit())
        && (component == "0" || !component.starts_with('0'))
}

fn is_namespaced_identifier(value: &str) -> bool {
    value.contains('.') && value.split('.').all(valid_symbol_segment)
}

fn valid_symbol_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn require_stable_symbol(field: &'static str, value: &str) -> Result<(), ProtocolError> {
    let valid = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(ProtocolError::InvalidSymbol {
            field,
            value: value.to_owned(),
        })
    }
}

fn reject_transport_markers(value: &Value) -> Result<(), ProtocolError> {
    const FORBIDDEN: &[&str] = &[
        "jsonrpc",
        "request_id",
        "jsonrpc_id",
        "transport_id",
        "stdio_frame",
        "process_id",
        "retry_policy",
        "reconnect_policy",
        "transport",
    ];

    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.as_str()) {
                    return Err(ProtocolError::TransportLeakage(key.clone()));
                }
                reject_transport_markers(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_transport_markers(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn reject_transport_markers_in_extensions(extensions: &Extensions) -> Result<(), ProtocolError> {
    reject_transport_markers(&Value::Object(
        extensions
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    ))
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

fn stable_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{AdapterProtocolVersion, ProtocolError};

    #[test]
    fn protocol_version_is_exact_major_minor() {
        assert_eq!(
            AdapterProtocolVersion::parse("0.1").unwrap(),
            AdapterProtocolVersion::current()
        );
        assert_eq!(AdapterProtocolVersion::current().to_string(), "0.1");
    }

    #[test]
    fn protocol_version_rejects_patch_and_noncanonical_forms() {
        for invalid in ["0.1.0", "00.1", "0.01", " 0.1 "] {
            assert!(matches!(
                AdapterProtocolVersion::parse(invalid),
                Err(ProtocolError::MalformedProtocolVersion(_))
            ));
        }
    }
}
