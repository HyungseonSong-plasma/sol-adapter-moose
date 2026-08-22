use sol_adapter_protocol::{ExecutePlanRequest, ProtocolFailure, ValidatePlanRequest};
use sol_adapter_transport::{
    decode_request, AdapterTransportMethod, JsonRpcResponse, RequestDisposition, StdioFrameDecoder,
    TransportRequest,
};
use sol_adaptor_moose::FoundationAdapter;
use std::io::{self, Read, Write};
use std::process;

fn main() {
    if let Err(error) = run() {
        eprintln!("adapter transport error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let adapter = FoundationAdapter;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut decoder = StdioFrameDecoder::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            decoder.finish().map_err(|error| error.to_string())?;
            return Ok(());
        }

        for frame in decoder.push(&buffer[..count]) {
            let frame = frame.map_err(|error| error.to_string())?;
            if let Some(response) = handle_frame(&adapter, &frame)? {
                output
                    .write_all(&response)
                    .map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
            }
        }
    }
}

fn handle_frame(adapter: &FoundationAdapter, frame: &str) -> Result<Option<Vec<u8>>, String> {
    match decode_request(frame) {
        RequestDisposition::Dispatch(request) => {
            Ok(Some(dispatch(adapter, &request)?.to_stdio_frame()))
        }
        RequestDisposition::Reject(response) => Ok(Some(response.to_stdio_frame())),
        RequestDisposition::IgnoreNotification => Ok(None),
    }
}

fn dispatch(
    adapter: &FoundationAdapter,
    request: &TransportRequest,
) -> Result<JsonRpcResponse, String> {
    match request.method() {
        AdapterTransportMethod::DescribeAdapter => match adapter.describe_adapter() {
            Ok(description) => success_response(
                request,
                description
                    .to_canonical_json()
                    .map_err(|error| error.to_string())?,
            ),
            Err(failure) => failure_response(request, &failure),
        },
        AdapterTransportMethod::ValidatePlan => {
            let result =
                parse_validate_request(request).and_then(|request| adapter.validate_plan(&request));
            match result {
                Ok(response) => success_response(
                    request,
                    response
                        .to_canonical_json()
                        .map_err(|error| error.to_string())?,
                ),
                Err(failure) => failure_response(request, &failure),
            }
        }
        AdapterTransportMethod::ExecutePlan => {
            let result =
                parse_execute_request(request).and_then(|request| adapter.execute_plan(&request));
            match result {
                Ok(response) => success_response(
                    request,
                    response
                        .to_canonical_json()
                        .map_err(|error| error.to_string())?,
                ),
                Err(failure) => failure_response(request, &failure),
            }
        }
    }
}

fn parse_validate_request(
    request: &TransportRequest,
) -> Result<ValidatePlanRequest, ProtocolFailure> {
    let params = request
        .params()
        .expect("validate_plan transport request always has params");
    let json = serde_json::to_string(params).expect("JSON Value always serializes");
    ValidatePlanRequest::from_json(&json).map_err(invalid_request_failure)
}

fn parse_execute_request(
    request: &TransportRequest,
) -> Result<ExecutePlanRequest, ProtocolFailure> {
    let params = request
        .params()
        .expect("execute_plan transport request always has params");
    let json = serde_json::to_string(params).expect("JSON Value always serializes");
    ExecutePlanRequest::from_json(&json).map_err(invalid_request_failure)
}

fn invalid_request_failure(error: impl ToString) -> ProtocolFailure {
    ProtocolFailure::invalid_request(error.to_string())
        .expect("transport parse error detail produces a valid ProtocolFailure")
}

fn success_response(
    request: &TransportRequest,
    canonical_payload: String,
) -> Result<JsonRpcResponse, String> {
    let payload = serde_json::from_str(&canonical_payload).map_err(|error| error.to_string())?;
    JsonRpcResponse::protocol_success(request.id(), payload).map_err(|error| error.to_string())
}

fn failure_response(
    request: &TransportRequest,
    failure: &ProtocolFailure,
) -> Result<JsonRpcResponse, String> {
    JsonRpcResponse::protocol_failure(request.id(), request.method(), failure)
        .map_err(|error| error.to_string())
}
