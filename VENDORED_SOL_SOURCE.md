# Archived vendored-contract provenance — replaced on P3 candidate branch

**Status on `packaging/official-sol-contract-consumer`: historical record, not current compilation.**

At Adapter Draft PR #43, the 16 previously duplicated source files under `vendor/sol-*` were deleted **after** accepted upstream git package builds, exact Cargo.lock dependency resolution, four request/response wire parity cases against the preserved `main@467494ebc7ed2ae301148e65db7236a72fc7e928` baseline, and a real MOOSE CI success checkpoint. The historical source remains inspectable at that exact `main` revision.

This branch now consumes the **single upstream authority** `HyungseonSong-plasma/simulation-ontology@b6924e6556684eec9dd93677518821cc7adc84b6` (Cargo package version `=0.1.0`) for `sol-public-contract`, `sol-adapter-protocol`, and `sol-adapter-transport`. The presence of the upstream scalar Public Contract 0.3 implementation does **not** mean this Adapter advertises Public Contract 0.3 or Adapter Protocol 0.3. The supported adapter compatibility pair remains 0.1/0.2 only.

**Release and acceptance gates remain open:** this is a Draft PR, and local workstation verification, approved immutable SOL/Adapter release distribution, human review and actual MOOSE scientific V&V are separate. If downstream parity fails, revert this specific branch to its original vendored baseline; do not silently modify upstream canonical semantics.

---

## Original vendored history (before P3)

A0.1 consumes accepted SOL contract source snapshots from `HyungseonSong-plasma/simulation-ontology`.

## Current contract baseline

Phase 3 pins the accepted M0.8 Realization 0.2 exit baseline:

- upstream repository: `HyungseonSong-plasma/simulation-ontology`
- upstream commit: `bceaf4c66fb33202e097e54da429e51f652c234a`
- Public Contract surfaces consumed: `0.1`, `0.2`
- Adapter Protocol surfaces consumed: `0.1`, `0.2`
- Adapter Transport surface consumed: unchanged `0.1` Phase 0 snapshot

The M0.8 pin is deliberate. Later SOL `main` work, including M0.10 / Public Contract 0.3 / Adapter Protocol 0.3 architecture, is outside A0.1 scope and is not consumed here.

## Snapshot history

A0.1 Phase 0 originally pinned upstream commit:

`69779d3ab7880f56618b29af82616965776e0126`

for Public Contract 0.1 / Adapter Protocol 0.1 / transport bootstrap.

Phase 3 synchronizes only the accepted M0.8 additions required for Realization 0.2. The pre-existing 0.1 default/parser semantics remain unchanged: 0.2 is an explicit version boundary, not a reinterpretation of 0.1.

## Included

- `vendor/sol-public-contract/src/*` — Phase 0 source plus accepted M0.8 Public Contract 0.2 realization additions.
- `vendor/sol-adapter-protocol/src/*` — Phase 0 source plus accepted M0.8 Adapter Protocol 0.2 realization additions.
- `vendor/sol-adapter-transport/` — Phase 0 transport snapshot; M0.8 introduced no required transport semantic change for A0.1.
- selected M0.8 0.2 thermal fixtures copied under `tests/fixtures/sol/0.2/` as immutable adapter-consumption evidence.

## Authority and replacement rule

Vendoring is a reproducible source-consumption mechanism, not a fork of SOL semantics and not a new public SDK/package contract. Normative authority remains the accepted published versioned SOL Public Contract and Adapter Protocol. The vendor boundary must remain replaceable by an accepted package/SDK/distribution mechanism without changing interoperability meaning.

Do not edit vendored semantic source locally to change SOL meaning. Contract changes must come from an accepted upstream versioned SOL decision and a reviewed snapshot update.
