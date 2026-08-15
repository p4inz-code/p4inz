# P4inz AI Agent Guidelines

This document defines mandatory rules for AI coding agents working on P4inz.

## Core Rules

- Read ROADMAP.md before implementation work.
- Read the relevant architecture documentation before modifying architecture.
- Do not redesign locked architecture without an explicit architectural decision.
- Do not introduce paid infrastructure as a required dependency.
- Preserve security, privacy, correctness, and reliability over implementation speed.
- Never expose, commit, or generate real secrets.
- Never use destructive commands without explicit authorization.
- Run appropriate tests after every implementation change.
- Do not automatically push changes to remote repositories.
- Keep commits focused and explain meaningful architectural changes.
