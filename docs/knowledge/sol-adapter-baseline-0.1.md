# SOL Adapter Baseline 0.1 for MOOSE Adapter Development

**Status:** bootstrap reference snapshot  
**Source baseline:** `HyungseonSong-plasma/simulation-ontology` main `69779d3ab7880f56618b29af82616965776e0126`  
**Public Contract:** 0.1  
**Adapter Protocol:** 0.1

## Purpose

This document gives the MOOSE Adapter Team enough SOL-side context to develop a real backend adapter without reconstructing the architecture from private chat or operating documents.

It is a repository-local handoff/reference, not a fork of SOL semantic authority. When the upstream SOL Public Contract or Adapter Protocol publishes a new explicit version, this repository must review compatibility rather than silently editing the meaning recorded here.

## 1. SOL product boundary

SOL is a solver-independent semantic platform above simulation backends.

```text
canonical simulation intent
        |
        v
validation / constraints
        |
        v
mapping claims + MappingPlan
        |
        v
BackendTarget / Adapter Protocol
        |
        v
backend adapter
```

SOL does not make MOOSE, COMSOL, Ansys, or another solver's object model the canonical semantic model.

The first real MOOSE adapter exists specifically to test the complete path from canonical SOL intent to a real backend while keeping solver-specific realization outside Core.

## 2. Core / adapter responsibility split

### SOL Platform owns

- canonical ontology entities and relations;
- semantic identity;
- Public Contract DTO meaning and schemas;
- Adapter Protocol meaning;
- `MappingPlan` DAG/dependency semantics and deterministic canonical representation;
- general adapter conformance tooling;
- solver-neutral adapter runtime/registry and SDK surfaces.

### MOOSE adapter owns

- translation from canonical plans/actions to MOOSE realization;
- backend scheduling that preserves dependency edges;
- MOOSE input/backend artifact generation;
- configured application invocation;
- backend-native capability/version checks;
- regression tests for translation/integration;
- physical/numerical V&V where the repository claims it;
- opaque backend provenance.

## 3. Public Contract concepts the adapter consumes

Adapter Protocol 0.1 reuses canonical Public Contract 0.1 payload meanings rather than defining local copies. Important reused concepts include:

- `BackendTargetDto`;
- `MappingPlanDto`;
- `Diagnostic`;
- `RealizationEffectDto`.

The adapter must not rename/reinterpret these fields to mimic MOOSE API structures.

A MOOSE-native `Kernel`, object handle, input block path, mesh ID, variable object, or execution job ID is not automatically a canonical SOL entity.

## 4. MappingPlan semantics

Core owns the plan's canonical semantic structure and dependency graph.

Important distinction:

```text
canonical deterministic plan representation
          !=
mandatory physical backend total order
```

The adapter may reorder or parallelize independent backend work if all accepted dependency edges and execution invariants remain preserved.

The adapter must produce exactly the response/effect behavior required by the supported Protocol version; internal MOOSE ordering is an implementation concern unless it changes canonical evidence.

## 5. Adapter Protocol 0.1 logical surface

```text
describe_adapter -> AdapterDescription | ProtocolFailure
validate_plan    -> ValidatePlanResponse | ProtocolFailure
execute_plan     -> ExecutePlanResponse | ProtocolFailure
```

These are logical operations. The current local v0.x transport maps them over JSON-RPC/stdin-stdout, but transport framing is not their semantic definition.

## 6. `describe_adapter`

A conforming description declares at least the accepted Protocol 0.1 concepts for:

- adapter implementation identity/version;
- supported Adapter Protocol versions;
- supported Public Contract versions;
- addressed backend targets;
- capability declarations.

Compatibility is dual-axis:

```text
Adapter Protocol compatibility
AND
Public Contract compatibility
```

The adapter package version proves neither.

For MOOSE, capability declaration should ultimately reflect the configured MOOSE-based application/backend reality rather than assuming all MOOSE applications register identical objects/modules.

## 7. `validate_plan`

`validate_plan` is:

- advisory;
- side-effect free;
- not an execution authorization token;
- not a SOL lifecycle state.

Typical adapter checks may include:

1. target compatibility;
2. required capabilities;
3. action support;
4. backend/application prerequisites observable without execution.

Expected negative conditions should use published valid response semantics when applicable. They do not automatically become `ProtocolFailure`.

A successful MOOSE `--check-input` can be backend evidence used by an implementation, but it does not redefine `validate_plan` or authorize later execution.

## 8. `execute_plan`

`execute_plan` is authoritative for the current request/state.

The adapter must:

- receive the full current target and plan again;
- re-check the relevant request/backend state before creating new side effects;
- preserve dependency-safe execution;
- produce terminal action reports/effects according to the supported Protocol contract;
- keep canonical realization subjects separate from backend-native provenance.

`execute_plan` is non-idempotent by default.

Therefore:

```text
lost execute response
   -> execution may have occurred
   -> no automatic replay authority
```

A transport timeout/loss is not proof that the backend did nothing.

## 9. Failure boundaries

Keep these distinct:

- valid negative `ValidatePlanResponse`;
- valid rejected/partial/unavailable `ExecutePlanResponse`;
- logical `ProtocolFailure`;
- JSON-RPC/framing/process/transport failure;
- SOL lifecycle classification;
- backend solver failure;
- backend physical/numerical invalidity.

Do not collapse them into one generic failure state.

For execution operational failure, side-effect evidence must be conservative. Use `none` only when the implementation can establish execution never began; otherwise preserve that effects may have occurred.

## 10. Provenance versus identity

Canonical semantic identity comes from SOL.

MOOSE-specific information such as:

- generated input-file path;
- MOOSE application executable/version;
- output file path;
- backend job/process ID;
- object/input-block path;
- mesh/output identifiers;

may be useful as opaque namespaced provenance/evidence.

They must not replace canonical entity/relation IDs or define semantic equality.

## 11. Transport boundary

The accepted local transport baseline is an external process using JSON-RPC over stdio.

Conceptually:

```text
SOL runtime / caller
   <-> JSON-RPC over stdio
sol-adaptor-moose process
   -> backend translation
   -> MOOSE subprocess / artifacts
```

Transport-only concepts such as JSON-RPC request IDs, process IDs, reconnect state, stderr text, framing bytes, and timeout state must not leak into Protocol payload semantics.

The adapter worker's Protocol stdout must not be corrupted by MOOSE solver console output. Backend logs should be captured/routed separately.

## 12. Conformance versus backend correctness

SOL conformance establishes that the adapter behaves according to the declared Public Contract / Adapter Protocol versions.

It does **not** establish that the generated MOOSE model is physically or numerically correct.

The MOOSE adapter should maintain separate evidence tracks:

```text
Protocol/Public Contract conformance
MOOSE translation regression
MOOSE application integration
physical/numerical V&V
performance/scalability when claimed
```

Examples:

- conformant response + wrong heat conductivity mapping -> backend realization bug;
- correct MOOSE temperature solution + malformed Protocol effects -> SOL non-conformance;
- valid `.i` syntax + wrong boundary mapping -> syntax success, semantic realization failure.

## 13. M0.6 conformance tooling handoff

The SOL Core completed reusable external-adapter conformance tooling before this repository was created.

The accepted tooling distinguishes aggregate outcomes:

```text
Conformant
NonConformant
NotEstablished
```

`NotEstablished` represents harness/fixture/transport evidence that prevents a conformance determination; it is not automatically adapter non-conformance.

The accepted suite includes positive and adversarial behavior for compatibility, targets/capabilities, preflight negatives, execution states, ProtocolFailure behavior, dependency scheduling, prior execution, transport loss, aggregate effects, and provenance identity.

The exact public conformance CLI spelling/report schema/environment variables remain provisional unless a later explicit SOL public-interface decision stabilizes them.

## 14. Cross-team evolution rule

Real MOOSE work is expected to expose pressure on the SOL contract. That feedback is valuable, but the adapter cannot silently redefine Protocol 0.1 or Public Contract 0.1.

When a real MOOSE case cannot be represented without reinterpretation:

```text
MOOSE evidence
 -> adapter-team problem statement + counterexample
 -> SOL Platform meeting / semantic review
 -> accepted change or rejected proposal
 -> explicit version/compatibility result
 -> adapter consumes the result
```

An incompatible structural/semantic Protocol change requires an explicit new protocol version boundary rather than silently changing 0.1.

## 15. Initial MOOSE adapter acceptance philosophy

The first vertical slice should optimize for end-to-end architectural proof rather than domain complexity:

```text
SOL intent
 -> MappingPlan
 -> Adapter Protocol
 -> real MOOSE translation
 -> backend artifact
 -> backend validation/execution
 -> realization/provenance evidence
```

A simple steady heat-conduction realization is the current bootstrap target because it exercises variables/operators/material/boundary/solver artifacts without requiring plasma-specific semantics.

More complex MOOSE/Zapdos/CRANE mappings should follow only after the generic adapter boundary is proven and should not be used to force domain-specific objects into SOL Core prematurely.
