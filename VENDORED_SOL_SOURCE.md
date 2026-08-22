# Vendored SOL contract/transport source

A0.1 Phase 0 consumes an exact source snapshot from:

- repository: `HyungseonSong-plasma/simulation-ontology`
- commit: `69779d3ab7880f56618b29af82616965776e0126`
- Public Contract: `0.1`
- Adapter Protocol: `0.1`

## Included

- `crates/sol-public-contract/src/*` — unchanged source snapshot.
- `crates/sol-adapter-protocol/src/*` — unchanged source snapshot.
- `crates/sol-adapter-transport/src/framing.rs` — unchanged source snapshot.
- `crates/sol-adapter-transport/src/json_rpc.rs` — unchanged source snapshot.
- transport method mapping from the same upstream `lib.rs`.

The upstream transport process/recovery modules are intentionally not vendored in Phase 0 because backend/process integration belongs to A0.1 Phase 1.

## Authority and replacement rule

Vendoring is a reproducible source-consumption mechanism, not a fork of SOL semantics and not a new public SDK/package contract. Normative authority remains the published versioned SOL Public Contract and Adapter Protocol. The vendor boundary must remain replaceable by an accepted package/SDK/distribution mechanism without changing interoperability meaning.

Do not edit vendored semantic source locally to change SOL meaning. Contract changes must come from an accepted upstream versioned SOL decision and a reviewed snapshot update.
