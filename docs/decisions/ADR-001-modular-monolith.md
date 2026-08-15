# ADR-001 — Modular Monolith

## Decision

P4inz will begin as a modular monolith using a Rust Cargo workspace.

## Reason

This provides strong internal boundaries without the operational complexity of microservices.

Individual subsystems can be extracted later if scale or reliability requirements justify it.
