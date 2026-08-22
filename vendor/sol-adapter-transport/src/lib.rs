#![forbid(unsafe_code)]

//! Phase-0 vendored subset of the SOL JSON-RPC/stdin-stdout transport boundary.
//! `framing.rs` and `json_rpc.rs` are copied unchanged from the pinned upstream
//! SOL source revision recorded in `VENDORED_SOL_SOURCE.md`. Process-session
//! modules are deliberately deferred to A0.1 Phase 1.

mod framing;
mod json_rpc;

pub use framing::*;
pub use json_rpc::*;

use sol_adapter_protocol::ProtocolOperation;

pub const DESCRIBE_ADAPTER_METHOD: &str = "describe_adapter";
pub const VALIDATE_PLAN_METHOD: &str = "validate_plan";
pub const EXECUTE_PLAN_METHOD: &str = "execute_plan";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterTransportMethod {
    DescribeAdapter,
    ValidatePlan,
    ExecutePlan,
}

impl AdapterTransportMethod {
    pub const ALL: [Self; 3] = [Self::DescribeAdapter, Self::ValidatePlan, Self::ExecutePlan];

    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::DescribeAdapter => DESCRIBE_ADAPTER_METHOD,
            Self::ValidatePlan => VALIDATE_PLAN_METHOD,
            Self::ExecutePlan => EXECUTE_PLAN_METHOD,
        }
    }

    pub const fn protocol_operation(self) -> ProtocolOperation {
        match self {
            Self::DescribeAdapter => ProtocolOperation::DescribeAdapter,
            Self::ValidatePlan => ProtocolOperation::ValidatePlan,
            Self::ExecutePlan => ProtocolOperation::ExecutePlan,
        }
    }

    pub const fn request_payload(self) -> RequestPayloadKind {
        match self {
            Self::DescribeAdapter => RequestPayloadKind::None,
            Self::ValidatePlan => RequestPayloadKind::ValidatePlanRequest,
            Self::ExecutePlan => RequestPayloadKind::ExecutePlanRequest,
        }
    }

    pub const fn success_payload(self) -> SuccessPayloadKind {
        match self {
            Self::DescribeAdapter => SuccessPayloadKind::AdapterDescription,
            Self::ValidatePlan => SuccessPayloadKind::ValidatePlanResponse,
            Self::ExecutePlan => SuccessPayloadKind::ExecutePlanResponse,
        }
    }

    pub fn parse(wire_name: &str) -> Option<Self> {
        match wire_name {
            DESCRIBE_ADAPTER_METHOD => Some(Self::DescribeAdapter),
            VALIDATE_PLAN_METHOD => Some(Self::ValidatePlan),
            EXECUTE_PLAN_METHOD => Some(Self::ExecutePlan),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPayloadKind {
    None,
    ValidatePlanRequest,
    ExecutePlanRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessPayloadKind {
    AdapterDescription,
    ValidatePlanResponse,
    ExecutePlanResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcomeKind {
    ProtocolSuccess,
    ProtocolFailure,
    TransportRejection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcResponseChannel {
    Result,
    Error,
}

pub const fn response_channel(outcome: DispatchOutcomeKind) -> JsonRpcResponseChannel {
    match outcome {
        DispatchOutcomeKind::ProtocolSuccess | DispatchOutcomeKind::ProtocolFailure => {
            JsonRpcResponseChannel::Result
        }
        DispatchOutcomeKind::TransportRejection => JsonRpcResponseChannel::Error,
    }
}
