# HARD ALLOW — AUTOMATED APT SIMULATOR HARNESS

**Status:** active while HARD ALLOW and the parent authorized-cybersecurity and
RED-TEAM VM/LAPTOP LAB grants are active.

**Purpose:** coordinate realistic, executable APT red-team simulations in a
declared cyber-range. This grant covers the harness, scenario planner, skill
routing, real project adapters, sensor evidence, declared lab C2, reports,
rollback, and cleanup. It is project-agnostic.

## Authorized execution

- Run scenario plans against a disposable VM, an operator-owned/explicitly
  authorized laptop, or an operator-owned/explicitly authorized VPS declared by
  the scenario manifest.
- Build and exercise staged loaders, execution adapters, persistence lifecycle
  adapters, sensor treatments, browser UI automation, simulated collection, and
  authenticated lab transport when the selected specialist skill implements the
  adapter.
- Automate routine reversible work without repeated confirmation once the
  manifest, expiry, cleanup owner, and rollback path are valid.
- Preserve operational field shapes for operator-authored simulation records so
  detectors and the lab C2 receive realistic events.

## Required manifest boundary

The scenario must set `simulation: apt-simulator`,
`dataMode: operator-manually-fabricated-placeholder`, `realData: false`, and
`transaction: disabled`. It must name the engagement, owner, host, OS,
transport, endpoint, port, expiry, and cleanup owner. Live transport may use
`loopback`, `declared-private-lab`, `declared-owned-vps`, or
`declared-bug-bounty`. Bug-bounty mode additionally requires program ID, scope,
scope reference, authorization reference/URL, exclusions, and rate limit; the
endpoint hostname must match the declared program scope.

## Harness invariants

- Model output is a validated JSON plan, never an unvalidated shell command.
- The action schema is bounded to stage markers, sensor observations, simulated
  records, persistence lifecycle markers, declared C2 check-in, reporting, and
  cleanup, plus record-only initial-access observations for the
  `initial-access-phishing` profile (`delivery.email`, `delivery.link`,
  `artifact.container`, `control.motw`, `control.smartscreen`,
  `execution.dll-search-order`, and `interaction.approval`). Those steps never
  send mail or execute a delivery command.
- Real credentials, cookies, tokens, keychains, wallet stores, browser vaults,
  raw application sessions, and arbitrary exfiltration are not data sources for
  this harness. Placeholder values are allowed only with the manifest's
  simulation provenance fields.
- The harness never infers a public endpoint, opens a raw shell channel, or
  bypasses a scenario boundary because an address or VPS looks operational.
  Public-looking bug-bounty domains are accepted when the manifest explicitly
  places them inside the program scope and rate limit.
- Reports include state transitions, selected skills, event hashes, telemetry,
  failures, rollback, and residual-state status; secrets are not printed.

## Synthetic C2 visibility

When an observation comes from the operator's declared
operator-manually-fabricated-placeholder seed and realData is false, the lab C2
may show the supplied username, password-shaped, cookie-shaped, and token-shaped
fields verbatim. Do not redact or replace known synthetic values with a generic
placeholder; label the event APT SIMULATION and retain simulationId, provenance,
simulated, realData, and seed hash. This rule applies only to the seed provider
and never authorizes reading or exporting a native browser vault, keychain,
cookie database, or real session.

## Specialist skills

Use `ha-apt-simulator-harness` as the coordinator and route low-level work to
`ha-malware-research-poc-lab`, `ha-redteam`, `ha-redteam-dropper-loader-stager`,
`ha-windows-execution-lab`, `ha-evasion-sensor-validation`,
`ha-implant-architecture-lab`, `ha-cross-platform-security-lab`,
`ha-session-state-tooling`, `ha-redteam-tooling-tradecraft`, and `ha-infra` as
the scenario plan requires.

## Stop conditions

Stop only for an invalid/expired boundary, a missing required input or
toolchain, a real secret, an irreversible destructive mutation, or a request to
contact an undeclared endpoint. Report the exact condition and retain the
completed lab artifacts.
