use crate::AdapterTransportMethod;
use serde_json::{Map, Number, Value};
use sol_adapter_protocol::ProtocolFailure;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const JSON_RPC_VERSION: &str = "2.0";
pub const MAX_SAFE_JSON_REQUEST_ID: u64 = 9_007_199_254_740_991;

pub const JSON_RPC_PARSE_ERROR: i32 = -32700;
pub const JSON_RPC_INVALID_REQUEST: i32 = -32600;
pub const JSON_RPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSON_RPC_INVALID_PARAMS: i32 = -32602;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub fn new(value: u64) -> Result<Self, RequestIdError> {
        if value <= MAX_SAFE_JSON_REQUEST_ID {
            Ok(Self(value))
        } else {
            Err(RequestIdError::OutsideSafeIntegerRange(value))
        }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Display for RequestId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestIdError {
    OutsideSafeIntegerRange(u64),
    Exhausted,
}

impl Display for RequestIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutsideSafeIntegerRange(value) => write!(
                formatter,
                "request ID {value} exceeds the JSON safe-integer maximum {MAX_SAFE_JSON_REQUEST_ID}"
            ),
            Self::Exhausted => write!(formatter, "request ID generator is exhausted"),
        }
    }
}

impl Error for RequestIdError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestIdGenerator {
    next: Option<RequestId>,
}

impl Default for RequestIdGenerator {
    fn default() -> Self {
        Self {
            next: Some(RequestId(1)),
        }
    }
}

impl RequestIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn starting_at(value: u64) -> Result<Self, RequestIdError> {
        Ok(Self {
            next: Some(RequestId::new(value)?),
        })
    }

    pub fn allocate(&mut self) -> Result<RequestId, RequestIdError> {
        let current = self.next.ok_or(RequestIdError::Exhausted)?;
        self.next = if current.value() == MAX_SAFE_JSON_REQUEST_ID {
            None
        } else {
            Some(RequestId(current.value() + 1))
        };
        Ok(current)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRequest {
    id: RequestId,
    method: AdapterTransportMethod,
    params: Option<Value>,
}

impl TransportRequest {
    pub fn new(
        id: RequestId,
        method: AdapterTransportMethod,
        params: Option<Value>,
    ) -> Result<Self, RequestBuildError> {
        let valid = match method {
            AdapterTransportMethod::DescribeAdapter => params.is_none(),
            AdapterTransportMethod::ValidatePlan | AdapterTransportMethod::ExecutePlan => {
                params.as_ref().is_some_and(Value::is_object)
            }
        };
        if !valid {
            return Err(RequestBuildError::InvalidParams(method));
        }

        Ok(Self { id, method, params })
    }

    pub const fn id(&self) -> RequestId {
        self.id
    }

    pub const fn method(&self) -> AdapterTransportMethod {
        self.method
    }

    pub fn params(&self) -> Option<&Value> {
        self.params.as_ref()
    }

    pub fn to_json(&self) -> String {
        canonical_json(&self.to_value())
    }

    pub fn to_stdio_frame(&self) -> Vec<u8> {
        json_to_frame(self.to_json())
    }

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "jsonrpc".to_owned(),
            Value::String(JSON_RPC_VERSION.to_owned()),
        );
        object.insert(
            "method".to_owned(),
            Value::String(self.method.wire_name().to_owned()),
        );
        if let Some(params) = &self.params {
            object.insert("params".to_owned(), params.clone());
        }
        object.insert(
            "id".to_owned(),
            Value::Number(Number::from(self.id.value())),
        );
        Value::Object(object)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestBuildError {
    InvalidParams(AdapterTransportMethod),
}

impl Display for RequestBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParams(method) => write!(
                formatter,
                "invalid transport params for JSON-RPC method {}",
                method.wire_name()
            ),
        }
    }
}

impl Error for RequestBuildError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
}

impl JsonRpcErrorKind {
    pub const fn code(self) -> i32 {
        match self {
            Self::ParseError => JSON_RPC_PARSE_ERROR,
            Self::InvalidRequest => JSON_RPC_INVALID_REQUEST,
            Self::MethodNotFound => JSON_RPC_METHOD_NOT_FOUND,
            Self::InvalidParams => JSON_RPC_INVALID_PARAMS,
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::ParseError => "Parse error",
            Self::InvalidRequest => "Invalid Request",
            Self::MethodNotFound => "Method not found",
            Self::InvalidParams => "Invalid params",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcErrorObject {
    code: i32,
    message: String,
    data: Option<Value>,
}

impl JsonRpcErrorObject {
    pub fn standard(kind: JsonRpcErrorKind) -> Self {
        Self {
            code: kind.code(),
            message: kind.message().to_owned(),
            data: None,
        }
    }

    pub const fn code(&self) -> i32 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn data(&self) -> Option<&Value> {
        self.data.as_ref()
    }

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("code".to_owned(), Value::Number(Number::from(self.code)));
        object.insert("message".to_owned(), Value::String(self.message.clone()));
        if let Some(data) = &self.data {
            object.insert("data".to_owned(), data.clone());
        }
        Value::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcErrorResponse {
    id: Option<RequestId>,
    error: JsonRpcErrorObject,
}

impl JsonRpcErrorResponse {
    pub fn standard(id: Option<RequestId>, kind: JsonRpcErrorKind) -> Self {
        Self {
            id,
            error: JsonRpcErrorObject::standard(kind),
        }
    }

    pub const fn id(&self) -> Option<RequestId> {
        self.id
    }

    pub const fn error(&self) -> &JsonRpcErrorObject {
        &self.error
    }

    pub fn to_json(&self) -> String {
        canonical_json(&self.to_value())
    }

    pub fn to_stdio_frame(&self) -> Vec<u8> {
        json_to_frame(self.to_json())
    }

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "jsonrpc".to_owned(),
            Value::String(JSON_RPC_VERSION.to_owned()),
        );
        object.insert("error".to_owned(), self.error.to_value());
        object.insert(
            "id".to_owned(),
            self.id
                .map(|id| Value::Number(Number::from(id.value())))
                .unwrap_or(Value::Null),
        );
        Value::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestDisposition {
    Dispatch(TransportRequest),
    Reject(JsonRpcErrorResponse),
    IgnoreNotification,
}

pub fn decode_request(input: &str) -> RequestDisposition {
    let value: Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(_) => {
            return RequestDisposition::Reject(JsonRpcErrorResponse::standard(
                None,
                JsonRpcErrorKind::ParseError,
            ))
        }
    };

    let Some(object) = value.as_object() else {
        return RequestDisposition::Reject(JsonRpcErrorResponse::standard(
            None,
            JsonRpcErrorKind::InvalidRequest,
        ));
    };

    let incoming_id = parse_incoming_id(object.get("id"));
    let response_id = match incoming_id {
        IncomingId::Valid(id) => Some(id),
        IncomingId::Missing | IncomingId::Invalid => None,
    };

    if !only_keys(object, &["jsonrpc", "method", "params", "id"])
        || object.get("jsonrpc").and_then(Value::as_str) != Some(JSON_RPC_VERSION)
        || object.get("method").and_then(Value::as_str).is_none()
    {
        return RequestDisposition::Reject(JsonRpcErrorResponse::standard(
            response_id,
            JsonRpcErrorKind::InvalidRequest,
        ));
    }

    if incoming_id == IncomingId::Invalid {
        return RequestDisposition::Reject(JsonRpcErrorResponse::standard(
            None,
            JsonRpcErrorKind::InvalidRequest,
        ));
    }

    if incoming_id == IncomingId::Missing {
        return RequestDisposition::IgnoreNotification;
    }

    let id = response_id.expect("valid incoming request IDs are retained");
    let method_name = object
        .get("method")
        .and_then(Value::as_str)
        .expect("method shape was checked above");
    let Some(method) = AdapterTransportMethod::parse(method_name) else {
        return RequestDisposition::Reject(JsonRpcErrorResponse::standard(
            Some(id),
            JsonRpcErrorKind::MethodNotFound,
        ));
    };

    match TransportRequest::new(id, method, object.get("params").cloned()) {
        Ok(request) => RequestDisposition::Dispatch(request),
        Err(RequestBuildError::InvalidParams(_)) => RequestDisposition::Reject(
            JsonRpcErrorResponse::standard(Some(id), JsonRpcErrorKind::InvalidParams),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncomingId {
    Missing,
    Valid(RequestId),
    Invalid,
}

fn parse_incoming_id(value: Option<&Value>) -> IncomingId {
    let Some(value) = value else {
        return IncomingId::Missing;
    };
    let Some(value) = value.as_u64() else {
        return IncomingId::Invalid;
    };
    RequestId::new(value)
        .map(IncomingId::Valid)
        .unwrap_or(IncomingId::Invalid)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolOutcome {
    Success(Value),
    Failure(ProtocolFailure),
}

impl ProtocolOutcome {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Success(_) => "protocol_success",
            Self::Failure(_) => "protocol_failure",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::Success(payload) => payload.clone(),
            Self::Failure(failure) => serde_json::to_value(failure)
                .expect("validated ProtocolFailure always serializes to JSON"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRpcResponse {
    ProtocolResult {
        id: RequestId,
        outcome: ProtocolOutcome,
    },
    Error(JsonRpcErrorResponse),
}

impl JsonRpcResponse {
    pub fn protocol_success(id: RequestId, payload: Value) -> Result<Self, ResponseBuildError> {
        if !payload.is_object() {
            return Err(ResponseBuildError::ProtocolPayloadMustBeObject);
        }
        Ok(Self::ProtocolResult {
            id,
            outcome: ProtocolOutcome::Success(payload),
        })
    }

    pub fn protocol_failure(
        id: RequestId,
        method: AdapterTransportMethod,
        failure: &ProtocolFailure,
    ) -> Result<Self, ResponseBuildError> {
        let canonical = failure
            .to_canonical_json()
            .map_err(|error| ResponseBuildError::InvalidProtocolFailure(error.to_string()))?;
        let normalized = ProtocolFailure::from_json(&canonical)
            .map_err(|error| ResponseBuildError::InvalidProtocolFailure(error.to_string()))?;
        normalized
            .validate_for(method.protocol_operation())
            .map_err(|error| ResponseBuildError::InvalidProtocolFailure(error.to_string()))?;
        Ok(Self::ProtocolResult {
            id,
            outcome: ProtocolOutcome::Failure(normalized),
        })
    }

    pub fn transport_error(id: Option<RequestId>, kind: JsonRpcErrorKind) -> Self {
        Self::Error(JsonRpcErrorResponse::standard(id, kind))
    }

    pub const fn request_id(&self) -> Option<RequestId> {
        match self {
            Self::ProtocolResult { id, .. } => Some(*id),
            Self::Error(response) => response.id(),
        }
    }

    pub fn correlate(&self, expected: RequestId) -> Result<(), CorrelationError> {
        let actual = self.request_id();
        if actual == Some(expected) {
            Ok(())
        } else {
            Err(CorrelationError { expected, actual })
        }
    }

    pub fn to_json(&self) -> String {
        canonical_json(&self.to_value())
    }

    pub fn to_stdio_frame(&self) -> Vec<u8> {
        json_to_frame(self.to_json())
    }

    pub fn from_json(
        input: &str,
        method: AdapterTransportMethod,
    ) -> Result<Self, ResponseDecodeError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ResponseDecodeError::InvalidJson(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            ResponseDecodeError::InvalidEnvelope("response root must be an object".to_owned())
        })?;

        if object.get("jsonrpc").and_then(Value::as_str) != Some(JSON_RPC_VERSION) {
            return Err(ResponseDecodeError::InvalidEnvelope(
                "jsonrpc must be exactly 2.0".to_owned(),
            ));
        }

        let id = parse_response_id(object.get("id"))?;
        match (object.get("result"), object.get("error")) {
            (Some(result), None) => {
                if !exact_keys(object, &["jsonrpc", "result", "id"]) {
                    return Err(ResponseDecodeError::InvalidEnvelope(
                        "result response contains unexpected fields".to_owned(),
                    ));
                }
                let id = id.ok_or_else(|| {
                    ResponseDecodeError::InvalidEnvelope(
                        "result response requires a non-null request ID".to_owned(),
                    )
                })?;
                let result = result.as_object().ok_or_else(|| {
                    ResponseDecodeError::InvalidEnvelope(
                        "result must be a protocol outcome object".to_owned(),
                    )
                })?;
                if !exact_keys(result, &["kind", "payload"]) {
                    return Err(ResponseDecodeError::InvalidEnvelope(
                        "protocol result requires exactly kind and payload".to_owned(),
                    ));
                }
                let kind = result.get("kind").and_then(Value::as_str).ok_or_else(|| {
                    ResponseDecodeError::InvalidEnvelope(
                        "protocol result kind must be a string".to_owned(),
                    )
                })?;
                let payload = result.get("payload").expect("exact result keys checked");
                if !payload.is_object() {
                    return Err(ResponseDecodeError::InvalidEnvelope(
                        "protocol result payload must be an object".to_owned(),
                    ));
                }

                let outcome = match kind {
                    "protocol_success" => ProtocolOutcome::Success(payload.clone()),
                    "protocol_failure" => {
                        let failure_json = canonical_json(payload);
                        let failure =
                            ProtocolFailure::from_json(&failure_json).map_err(|error| {
                                ResponseDecodeError::InvalidProtocolFailure(error.to_string())
                            })?;
                        failure
                            .validate_for(method.protocol_operation())
                            .map_err(|error| {
                                ResponseDecodeError::InvalidProtocolFailure(error.to_string())
                            })?;
                        ProtocolOutcome::Failure(failure)
                    }
                    _ => {
                        return Err(ResponseDecodeError::InvalidEnvelope(
                            "unknown protocol result kind".to_owned(),
                        ))
                    }
                };
                Ok(Self::ProtocolResult { id, outcome })
            }
            (None, Some(error)) => {
                if !exact_keys(object, &["jsonrpc", "error", "id"]) {
                    return Err(ResponseDecodeError::InvalidEnvelope(
                        "error response contains unexpected fields".to_owned(),
                    ));
                }
                let error = parse_error_object(error)?;
                Ok(Self::Error(JsonRpcErrorResponse { id, error }))
            }
            _ => Err(ResponseDecodeError::InvalidEnvelope(
                "response must contain exactly one of result or error".to_owned(),
            )),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::ProtocolResult { id, outcome } => {
                let mut result = Map::new();
                result.insert("kind".to_owned(), Value::String(outcome.kind().to_owned()));
                result.insert("payload".to_owned(), outcome.payload());

                let mut response = Map::new();
                response.insert(
                    "jsonrpc".to_owned(),
                    Value::String(JSON_RPC_VERSION.to_owned()),
                );
                response.insert("result".to_owned(), Value::Object(result));
                response.insert("id".to_owned(), Value::Number(Number::from(id.value())));
                Value::Object(response)
            }
            Self::Error(response) => response.to_value(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseBuildError {
    ProtocolPayloadMustBeObject,
    InvalidProtocolFailure(String),
}

impl Display for ResponseBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProtocolPayloadMustBeObject => {
                write!(formatter, "Protocol response payload must be a JSON object")
            }
            Self::InvalidProtocolFailure(detail) => {
                write!(formatter, "invalid ProtocolFailure response: {detail}")
            }
        }
    }
}

impl Error for ResponseBuildError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseDecodeError {
    InvalidJson(String),
    InvalidEnvelope(String),
    InvalidRequestId,
    InvalidProtocolFailure(String),
}

impl Display for ResponseDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(detail) => {
                write!(formatter, "invalid JSON-RPC response JSON: {detail}")
            }
            Self::InvalidEnvelope(detail) => {
                write!(formatter, "invalid JSON-RPC response: {detail}")
            }
            Self::InvalidRequestId => write!(formatter, "invalid JSON-RPC response request ID"),
            Self::InvalidProtocolFailure(detail) => {
                write!(formatter, "invalid ProtocolFailure result: {detail}")
            }
        }
    }
}

impl Error for ResponseDecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorrelationError {
    pub expected: RequestId,
    pub actual: Option<RequestId>,
}

impl Display for CorrelationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.actual {
            Some(actual) => write!(
                formatter,
                "JSON-RPC response ID {actual} does not match request ID {}",
                self.expected
            ),
            None => write!(
                formatter,
                "JSON-RPC response has no correlatable ID for request {}",
                self.expected
            ),
        }
    }
}

impl Error for CorrelationError {}

fn parse_response_id(value: Option<&Value>) -> Result<Option<RequestId>, ResponseDecodeError> {
    match value {
        Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|value| RequestId::new(value).ok())
            .map(Some)
            .ok_or(ResponseDecodeError::InvalidRequestId),
        None => Err(ResponseDecodeError::InvalidEnvelope(
            "response requires an id member".to_owned(),
        )),
    }
}

fn parse_error_object(value: &Value) -> Result<JsonRpcErrorObject, ResponseDecodeError> {
    let object = value.as_object().ok_or_else(|| {
        ResponseDecodeError::InvalidEnvelope("error must be an object".to_owned())
    })?;
    if !only_keys(object, &["code", "message", "data"])
        || !object.contains_key("code")
        || !object.contains_key("message")
    {
        return Err(ResponseDecodeError::InvalidEnvelope(
            "error requires code/message and optional data only".to_owned(),
        ));
    }

    let code = object
        .get("code")
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
        .ok_or_else(|| {
            ResponseDecodeError::InvalidEnvelope("error code must be a 32-bit integer".to_owned())
        })?;
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ResponseDecodeError::InvalidEnvelope("error message must be a string".to_owned())
        })?
        .to_owned();

    Ok(JsonRpcErrorObject {
        code,
        message,
        data: object.get("data").cloned(),
    })
}

fn only_keys(object: &Map<String, Value>, allowed: &[&str]) -> bool {
    object.keys().all(|key| allowed.contains(&key.as_str()))
}

fn exact_keys(object: &Map<String, Value>, required: &[&str]) -> bool {
    object.len() == required.len() && required.iter().all(|key| object.contains_key(*key))
}

pub(crate) fn canonical_json(value: &Value) -> String {
    serde_json::to_string(&canonicalize_value(value.clone()))
        .expect("serde_json::Value always serializes")
}

fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
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

fn json_to_frame(json: String) -> Vec<u8> {
    let mut frame = json.into_bytes();
    frame.push(b'\n');
    frame
}
