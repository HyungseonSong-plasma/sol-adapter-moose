# Vendored SOL contract/transport source

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
