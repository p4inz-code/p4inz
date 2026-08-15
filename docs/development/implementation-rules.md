# Implementation Rules

## Session Model

One implementation prompt should represent one meaningful milestone that can reasonably be completed in approximately one focused hour.

Agents may combine closely related work when doing so improves consistency.

Do not artificially split work into tiny tasks.

## Required Loop

Inspect
→ Implement
→ Test
→ Debug
→ Validate
→ Report

## Scope Control

A session must have a clearly defined scope.

Do not implement future-phase functionality unless it is required by the current milestone.

Do not perform unrelated refactoring.

## Quality

Prefer:
- Simple designs
- Explicit boundaries
- Strong typing
- Deterministic behavior
- Testable code
- Clear errors
- Minimal dependencies

Avoid:
- Premature abstraction
- Cleverness without value
- Large untested rewrites
- Hidden global state
- Unnecessary macros
- Unnecessary unsafe Rust

## Definition of Done

A milestone is done when:
- Implementation works.
- Relevant tests pass.
- Formatting passes.
- Clippy passes where applicable.
- No known critical regression exists.
- The agent can clearly explain the changes.

Dedicated audits are separate from implementation completion.
