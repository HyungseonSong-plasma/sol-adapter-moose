#![forbid(unsafe_code)]

use sol_adapter_protocol::{
    protocol_diagnostic, ActionExecutionReport, ActionExecutionState, AdapterBootstrap,
    AdapterDescription, AdapterProtocolDiagnosticContext, ExecutePlanRequest, ExecutePlanResponse,
    ExecutionOutcome, PreflightOutcome, ProtocolFailure, ProtocolOperation, SideEffectEvidence,
    ValidatePlanRequest, ValidatePlanResponse, ADAPTER_PROTOCOL_VERSION,
    DIAGNOSTIC_EXECUTION_REJECTED, DIAGNOSTIC_MISSING_CAPABILITY, DIAGNOSTIC_TARGET_MISMATCH,
};
use sol_public_contract::{Diagnostic, PUBLIC_CONTRACT_VERSION};

/// Phase-0 adapter host.
///
/// It intentionally declares no MOOSE target until A0.1 Phase 1 records exact
/// executable/package evidence. This avoids converting an unverified backend
/// assumption into Protocol target/capability evidence.
#[derive(Debug, Clone, Copy, Default)]
pub struct FoundationAdapter;

impl FoundationAdapter {
    pub fn describe_adapter(&self) -> Result<AdapterDescription, ProtocolFailure> {
        let description = AdapterDescription {
            bootstrap: AdapterBootstrap {
                adapter_id: "sol.adapter.moose".to_owned(),
                adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
                supported_adapter_protocol_versions: Some(vec![
                    ADAPTER_PROTOCOL_VERSION.to_owned(),
                ]),
                supported_public_contract_versions: Some(vec![
                    PUBLIC_CONTRACT_VERSION.to_owned(),
                ]),
                extensions: Default::default(),
            },
            targets: Vec::new(),
            extensions: Default::default(),
        };

        let canonical = description.to_canonical_json().map_err(|error| {
            operation_failure(
                ProtocolOperation::DescribeAdapter,
                error,
                SideEffectEvidence::None,
            )
        })?;
        AdapterDescription::from_json(&canonical).map_err(|error| {
            operation_failure(
                ProtocolOperation::DescribeAdapter,
                error,
                SideEffectEvidence::None,
            )
        })
    }

    pub fn validate_plan(
        &self,
        request: &ValidatePlanRequest,
    ) -> Result<ValidatePlanResponse, ProtocolFailure> {
        let canonical = request.to_canonical_json().map_err(|error| {
            ProtocolFailure::invalid_request(error.to_string())
                .expect("non-blank request error produces ProtocolFailure")
        })?;
        let request = ValidatePlanRequest::from_json(&canonical).map_err(|error| {
            ProtocolFailure::invalid_request(error.to_string())
                .expect("non-blank request error produces ProtocolFailure")
        })?;

        let diagnostics = vec![
            protocol_diagnostic(
                DIAGNOSTIC_TARGET_MISMATCH,
                "no MOOSE target is declared until an exact backend executable is verified",
                AdapterProtocolDiagnosticContext {
                    target: Some(request.target.target.clone()),
                    ..Default::default()
                },
            ),
            protocol_diagnostic(
                DIAGNOSTIC_MISSING_CAPABILITY,
                "capability support is not declared before exact backend verification",
                AdapterProtocolDiagnosticContext::default(),
            ),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            operation_failure(
                ProtocolOperation::ValidatePlan,
                error,
                SideEffectEvidence::None,
            )
        })?;

        ValidatePlanResponse::new(false, false, PreflightOutcome::Rejected, diagnostics).map_err(
            |error| {
                operation_failure(
                    ProtocolOperation::ValidatePlan,
                    error,
                    SideEffectEvidence::None,
                )
            },
        )
    }

    pub fn execute_plan(
        &self,
        request: &ExecutePlanRequest,
    ) -> Result<ExecutePlanResponse, ProtocolFailure> {
        let canonical = request.to_canonical_json().map_err(|error| {
            ProtocolFailure::invalid_request(error.to_string())
                .expect("non-blank request error produces ProtocolFailure")
        })?;
        let request = ExecutePlanRequest::from_json(&canonical).map_err(|error| {
            ProtocolFailure::invalid_request(error.to_string())
                .expect("non-blank request error produces ProtocolFailure")
        })?;

        if request.plan.actions.is_empty() {
            return Err(
                ProtocolFailure::compatibility_not_established(
                    "no verified MOOSE target is declared and the empty plan has no action reports for a rejected execution response",
                )
                .expect("static compatibility detail is valid"),
            );
        }

        let mut response = ExecutePlanResponse {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION.to_owned(),
            execution: ExecutionOutcome::Rejected,
            execution_batches: Vec::new(),
            action_reports: request
                .plan
                .actions
                .iter()
                .map(|action| ActionExecutionReport {
                    action_id: action.id.clone(),
                    state: ActionExecutionState::NotStarted,
                    effects: Vec::new(),
                    diagnostics: Vec::new(),
                    provenance: None,
                    extensions: Default::default(),
                })
                .collect(),
            effects: Vec::new(),
            diagnostics: vec![Diagnostic::error(
                DIAGNOSTIC_EXECUTION_REJECTED,
                None,
                "authoritative execution rejected because no verified MOOSE target is declared",
            )],
            provenance: None,
            extensions: Default::default(),
        };

        response.validate_against(&request).map_err(|error| {
            operation_failure(
                ProtocolOperation::ExecutePlan,
                error,
                SideEffectEvidence::None,
            )
        })?;
        Ok(response)
    }
}

fn operation_failure(
    operation: ProtocolOperation,
    error: impl ToString,
    side_effects: SideEffectEvidence,
) -> ProtocolFailure {
    let failure = ProtocolFailure::operational(error.to_string(), side_effects)
        .expect("adapter error detail is non-blank");
    failure
        .validate_for(operation)
        .expect("side-effect evidence is valid for operation");
    failure
}

#[cfg(test)]
mod tests {
    use super::FoundationAdapter;
    use sol_adapter_protocol::{
        ActionExecutionState, ExecutePlanRequest, ExecutionOutcome, PreflightOutcome,
        ValidatePlanRequest, ADAPTER_PROTOCOL_VERSION,
    };
    use sol_public_contract::PUBLIC_CONTRACT_VERSION;

    const REQUEST_BODY: &str = r#"{
        "adapter_protocol_version":"0.1",
        "target":{
            "public_contract_version":"0.1",
            "target":"moose",
            "required_capabilities":["thermal.solve"]
        },
        "plan":{
            "public_contract_version":"0.1",
            "actions":[{"id":"opaque.action","dependencies":[]}]
        }
    }"#;

    #[test]
    fn description_declares_contract_versions_without_unverified_target() {
        let description = FoundationAdapter.describe_adapter().unwrap();
        assert_eq!(
            description.bootstrap.supported_adapter_protocol_versions,
            Some(vec![ADAPTER_PROTOCOL_VERSION.to_owned()])
        );
        assert_eq!(
            description.bootstrap.supported_public_contract_versions,
            Some(vec![PUBLIC_CONTRACT_VERSION.to_owned()])
        );
        assert!(description.targets.is_empty());
    }

    #[test]
    fn preflight_rejects_without_assigning_meaning_to_action_id() {
        let request = ValidatePlanRequest::from_json(REQUEST_BODY).unwrap();
        let response = FoundationAdapter.validate_plan(&request).unwrap();
        assert!(!response.target_compatible);
        assert!(!response.capabilities_satisfied);
        assert_eq!(response.preflight, PreflightOutcome::Rejected);
        assert_eq!(response.diagnostics.len(), 2);
    }

    #[test]
    fn execute_rejects_before_side_effects() {
        let request = ExecutePlanRequest::from_json(REQUEST_BODY).unwrap();
        let response = FoundationAdapter.execute_plan(&request).unwrap();
        assert_eq!(response.execution, ExecutionOutcome::Rejected);
        assert!(response.execution_batches.is_empty());
        assert!(response.effects.is_empty());
        assert_eq!(response.action_reports.len(), 1);
        assert_eq!(
            response.action_reports[0].state,
            ActionExecutionState::NotStarted
        );
    }
}
