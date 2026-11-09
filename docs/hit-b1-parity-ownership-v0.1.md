# B1 MOOSE/HIT Python↔Rust parity and responsibility decision record

Status: **REVIEW ONLY — no Physics consumer cutover or scientific PASS**.
Scope: Adapter #29/#30, Physics #281, SOL #172. Adapter main SHA 467494ebc7ed2ae301148e65db7236a72fc7e928.

## A. Actual existing behavioral evidence

- Existing 18 supported input/operation differential cases + 3 intentional incompatible demonstrations: [Adapter CI #39952358067](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/39952358067), rust and hit-parity SUCCESS.
- The PR adds 10 further common-slice test inputs to tests/hit_b1_parity.rs. These do not count as passed until new exact-head hit-parity job succeeds.
- The Python oracle is pinned to Physics ea3e63ba4e4edd75e8227748e185ddd5fbb106d8. Read-back confirmed the current Physics main d60bb8422f0a68f2e2699e5cf8d28b58b53369d0 has **identical Git blob SHAs** for input.py (f00f27da6ce791bfc76c6cca1d6eb9e0cd613722), blocks.py (a5ef499ea245f0dfa93cbdcb9dbbc7116dac1148), and parameters.py (d514bdaf5f4326bf026539439d2de0d2f405e253). This does not cover their caller behavior.
- Common tested surface: parsing/unique/direct children; direct parameter get/upsert/remove; block remove/replace/insert/append; LF/CRLF, comments, Unicode, and malformed/ambiguous inputs. Tests require actual edited-text equality or structured error-class equality.

## Known deliberate differences — fail closed on these consumer slices

1. Legacy Python parameter regex can match nested child assignments under a parent block; Rust restricts parameter ownership to the direct block. Treat different results as a contractual divergence, not a reason to reproduce unsafe legacy behavior.
2. Python BlockSpan start/end are Unicode codepoint offsets; Rust HIT offsets are UTF-8 bytes. Demonstrated Python [4,11] vs Rust [5,12]. Never use the Rust spans to slice a Python str without a conversion policy.
3. The Rust editor permits a commented closing block line that legacy Python rejects for certain mutations. Do not claim parity on this syntax.
4. Original Python remove_paths supports multi-path removal and span metadata, and replace_parameters returns old/new metadata. Those are not yet guaranteed external Rust capabilities.

**Conclusion:** shared-slice parity does not imply full behavioral parity, actual Physics consumer compatibility, or SOL-callable HIT operation.

## B. Ownership & required verification

| Function | Authority/owner | Verification status |
| --- | --- | --- |
| Canonical quantity/unit/scope, contract action/result | SOL | Formal contract gate separate |
| Physical intent, temporal policy, numerical V&V, original callers | Physics moose-test-repo | #281 and #282 remain open |
| HIT parse, unique path, byte-preserving edit and generated input structure | sol-adapter-moose | B1 Rust + A1 merged; exact-main [CI #42085465024](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42085465024) PASS |
| Backend preflight/--check-input operational fact classification | Adapter candidate | Not migrated; needs per-function ownership and tests |
| JIT compiler/libMesh/FParser root cause | Physics/MOOSE runtime | #471 independent, not SOL semantics |
| CSV/Exodus actual same-run thermal facts | Adapter test-only V&V | Draft #38; Physics sign-off #39 open |
| Scientific acceptance and 0.3 result producer | Physics Validator and SOL | Not approved, do not advertise |

Existing A1 calls validate_generated_structure after adapter IR generation but **before backend readiness, workspace creation or process side effects**. Missing/duplicate generated paths are rejected as operational failures with SideEffectEvidence::None, not failed SOL semantic requests. This is NOT an external Physics caller cutover.

Next B work should characterize preflight.py and check_input.py by *function*, separating MOOSE input grammar and backend execution failure facts from Physics temporal/solver acceptance. Use deterministic negative controls for malformed input, absent backend, unsupported configuration, timeout, post-start ambiguous failure and no replay. Neither source_inspection.py nor target.py should be extracted based on a filename.

## C deferred; D evidence-only

Packaging/installer work #40 / Draft #43 is **deferred by user**. Do not modify SDK pins, releases or archives in this batch.
Thermal same-run scalar+field may be linked as **native evidence** only (Adapter Draft #38/#39, SOL #156/#173). No AP 0.3 implementation or canonical Temperature Field observation claim follows from a nonempty Exodus file or same-run V&V success.

## Gate before source retirement

Accepted SOL callable contract → selected Physics canonical caller → acknowledged full **selected-slice** parity (including intentional deltas) → immutable Adapter pin → actual external consumer CI and native results → zero verified old canonical callers → independent science/semantic gates → separately reviewed retirement. No legacy deletion now.

## Consumer-specific disposition — exact Physics source review (2026-11-09)

Physics source reference: moose-test-repo@d60bb8422f0a68f2e2699e5cf8d28b58b53369d0. Code search is a usage census, not confirmation that a historical script is still a production caller. None of these rows is a deployment approval.

| Consumer / source | Known B1 behavior required | Responsibility and explicit decision | Gate before any change |
| --- | --- | --- | --- |
| `physics_harness/adapters/moose/input_contract.py::audit_generated_input` | Parse block paths and exact **direct-child** assignments; compares generated HIT against Physics `MooseInputContract` | **Adapter:** parse, unique-path, duplicate/malformed input, generated structural proof. **Physics:** expected type/unit/object/assignment and semantic acceptance. Candidate first consumer; **KEEP** existing Physics caller until one accepted semantic transport exists. | Existing Adapter A1 only validates its *own* thermal-generated HIT; no external `audit_generated_input` API. Need SOL #172 decision, Physics #281 exact selected-slice consumer parity and no-bypass test. |
| `physics_harness/adapters/moose/transforms.py::_remove_paths` plus historical study callers | Legacy `remove_paths` accepts multi-block removal, rejects intersecting spans and returns offsets; Rust B1 has a single-block deletion | **Physics:** MutationCasePlan/experiment intent. **Adapter candidate:** atomic, backend-only multi-block edit **if** an actual admitted caller requires it. **KEEP**; do not emulate all-at-once semantics by sequential single-block deletion without proof. | Multi-path atomicity, overlap, metadata and failure parity fixtures; source/caller census and explicit action contract. |
| `physics_harness/adapters/moose/output_observation.py` | `_parameter_count`, `_set_parameter`, `_remove_optional_parameter`, `_block_parameter` use block-level substring and regex; nested-path behavior differs from Rust direct-child rules | **Mixed:** generic MOOSE output-block edits can eventually be Adapter-owned, but required time separation, observation-report evaluation and acceptance stay Physics-owned. **DEFER**. | Independently classify direct vs nested property treatment and legacy old/new metadata; approved canonical result boundary before external move. |
| `physics_harness/adapters/moose/species_plot.py::_find_temperature` | `doc.text[span.start:span.end]` slices a Python `str` then interprets `T_g_fixed` from a study-specific material | **Physics/study:** KEEP; `T_g_fixed` interpretation and oxygen species plot are not SOL or generic adapter semantics. Never substitute UTF-8 byte offsets for Python codepoint offsets. | If a future reader must share a raw span, agree explicit coordinate system (`UTF8_BYTE` vs `PYTHON_CODEPOINT`) and separate fact from study interpretation. |
| `physics_harness/adapters/moose/petsc_options.py` and `executioner.py` | get/upsert/remove lower-level parameters; typed flags, options, step fields | **MOOSE_GENERIC mechanism candidate**, but experiment-specific PETSc solver recommendations stay Physics. **DEFER after B1**. | Direct-child parameters, ambiguity/failure and option order/quoting parity; accepted typed solver-control realization only. |
| `physics_harness/adapters/moose/transforms.py` non-delete operations | `MutationCasePlan`/`OperationPlan` carries plan/evaluation rules as well as backend HIT transformations | **MIXED**; orchestration and campaign plan remain Physics; primitive implementation can be Adapter. **KEEP** until small contract and source-backed caller established. | Exact action-id/operation ordering, text identity, exception and replay invariants; no wholesale MutationSpec migration. |
| Historical `experiments/`, `studies/` and local CLI consumers | Diverse direct MooseInput and parameter edit usages; many pinned recipes and provenance links | **STUDY_OR_PROVENANCE or unclassified** until fresh canonical caller proof. Keep historical scripts; they cannot be moved or deleted merely because codeQA reports imports. | Proven production call-path census and separately approved retirement; zero-caller proof only over precise supported slice. |

### Classified B1 mismatch and chosen default policy

| Difference | Default disposition | Explicit non-goal |
| --- | --- | --- |
| Python parent query sees nested assignments, Rust strictly direct-child | **Keep Rust fail-closed direct-child semantics.** Review Physics consumers that relied on regex traversal and expose explicit nested target path when needed. | No silent legacy nested match in Adapter. |
| Python Unicode span codepoints vs Rust UTF-8 byte offsets | **Do not transport offsets across language boundary.** Prefer path-scoped operation/opaque evidence. Require explicit conversion/round-trip tests if an offset consumer is selected. | Do not store byte offsets as SOL semantic identity. |
| Commented closing line accepted by Rust edit but rejected by legacy | Treat as **explicit capability extension** only when validated output remains structurally sound; retain negative baseline. | No unqualified parity claim on commented-close case. |
| Python multi-delete and old/new metadata vs Rust one-operation surface | **No release claim** until an actual selected consumer proves an atomic operation and metadata requirements; preserve original Python if not needed. | No blanket Rust feature copying based on source API shape. |
| Python malformed payload acceptance and Rust stricter rejection | Prefer **fail-closed** Rust; characterize which historical Python inputs are malformed vs valid; target backend accepted syntax only. | No relaxation of structural checks to make counts green. |

## SOL-mediated external call contract: bounded design options (NOT accepted)

**Existing functional boundary:** SOL runtime and Adapter Protocol 0.2 support semantic `validate_plan` and `execute_plan` for `thermal.steady_conduction` only. `pub mod hit` is a Rust library surface, not a SOL-callable action. Adapter A1 validates adapter-generated HIT internally and before backend side effects. The current API cannot be asserted to accept caller-supplied generic HIT edits; Physics' plasma/`audit_generated_input` cutover remains BLOCKED.

| Option | Mechanics | Advantages | Decision / gate |
| --- | --- | --- | --- |
| **A (preferred for currently supported thermal)** | Physics supplies accepted SOL semantic intent; SOL validates AP/PC and forwards `RealizationSpec`/plan; Adapter internally realizes, structurally audits HIT, invokes MOOSE, returns existing protocol outcome and backend artifacts. Physics never submits native HIT text/offset through SOL. | Keeps semantic and backend authority separate, no new public API. | **Suitable now only for existing accepted thermal slice**, with explicit exact-pin E2E tests. Does NOT cut over raw Physics HIT parser. |
| **B (future versioned, only if genuine external evidence consumer exists)** | Separate solver-neutral `BackendRealizationEvidence` or output-envelope extension carrying opaque backend artifact digests, verification phase/failure facts, and source identity; transport/version and registry/no-replay rules owned by SOL; no user-controlled `edit_hit_text` endpoint. | Can justify a callable external structural proof without leaking backend-native mutation into ontology. | **Needs SOL #172 explicit contract acceptance** and negative old-protocol compatibility tests. Do NOT modify AP0.2 or reuse AP0.3 result DTO to sneak this in. |
| **C (keep original Physics `audit_generated_input` until plasma request accepted)** | Adapter A1 remains local; Physics continues local semantic/generated-input acceptance and separate Physics #281 controls. | No premature migration; stable legacy behavior. | **Present default for plasma or unsupported caller** until SOL/Physics gates. |

### Example evidence/progression contract (design narrative, no DTO)

1. Request identity: Physics semantic request + exact accepted SOL runtime/PC/AP versions + pinned Adapter binary/source and supported MOOSE executable identity. Unknown capability rejects before side effects.
2. Preflight: SOL contract/schema and capability status first, Adapter generated HIT structure second, MOOSE `--check-input` third; record actual phase, exit/timeout and structured failure; no backfilling a success status from skipped execution.
3. Execute: only after successful preflight, with fresh workspace and no duplicate replay after ambiguous post-start failure. Keep `SideEffectEvidence::None` vs `MayHaveOccurred` distinct.
4. Evidence: backend-local path/byte evidence and raw output SHA are **noncanonical**; semantic quantities/unit/scope and independent scientific oracle remain Physics/SOL authorities. No generic HIT block path, UTF-8 offset, private workspace path or raw mutation recipe becomes SOL semantics.
5. Consumer parity test: compare exact request, generated input bytes (for supported common subset), backend probe/termination/side-effects and stable response fields under a pinned external Runtime→Adapter version. Match scientific observable values only with independent Physics validation; CI/conformance alone is insufficient.
6. Negative controls: nested parameter ambiguity; invalid/missing block; unsupported capability/version; missing MOOSE; backend timeout; after-start response loss; malformed source and intentional Unicode mismatch. Fail closed without accidental runner replays.

**Decision asked at SOL #172:** first caller scope and output evidence boundary. Until approved, Option A is limited to existing thermal capability, Option C governs unsupported Physics plasma/HIT consumers, and Option B is merely a candidate. No SOL API, adapter runtime, original Physics model or package publication is changed in this Draft.
