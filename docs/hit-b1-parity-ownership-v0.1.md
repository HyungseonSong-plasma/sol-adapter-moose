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
