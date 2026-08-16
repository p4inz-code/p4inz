#!/usr/bin/env python3
"""Validate workspace structure and dependency direction against
docs/architecture/dependency-rules.md and docs/architecture/overview.md.

Stdlib-only (uses tomllib, Python 3.11+) so it introduces no new dependency.
Run from anywhere; paths are resolved relative to the repository root.
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# The crate set described in docs/architecture/overview.md. A mismatch here
# means the workspace drifted from the documented architecture and needs an
# ADR (see docs/development/implementation_plan.md, section 30 "Change
# Control") before the member list and this check are updated together.
EXPECTED_MEMBERS = {
    "apps/p4inz",
    "apps/p4inz-worker",
    "crates/domain",
    "crates/application",
    "crates/infrastructure",
    "crates/api",
    "crates/discord",
    "crates/knowledge",
    "crates/search",
    "crates/ai",
    "crates/security",
    "crates/observability",
    "crates/jobs",
    "crates/database",
    "crates/config",
    "crates/errors",
    "crates/audit",
    "crates/common",
}

# Crate names that indicate a direct PostgreSQL driver dependency.
DATABASE_DRIVER_CRATES = {"sqlx", "tokio-postgres", "postgres", "deadpool-postgres", "diesel"}

# Only these crate paths may depend on a database driver directly; everything
# else must go through p4inz-database / p4inz-infrastructure ("Random
# cross-layer database access" is explicitly forbidden).
DATABASE_ACCESS_ALLOWED = {
    "crates/database",
    "crates/infrastructure",
    "crates/search",
    "apps/p4inz",
    "apps/p4inz-worker",
}

# Per-crate forbidden dependencies, encoding the "Forbidden" list in
# docs/architecture/dependency-rules.md plus the general dependency-direction
# rule (domain <- application <- adapters, infrastructure -> domain/application).
FORBIDDEN_DEPENDENCIES = {
    "crates/domain": {
        # Discord
        "serenity", "twilight-gateway", "twilight-http", "twilight-model",
        "p4inz-discord",
        # HTTP frameworks / clients
        "axum", "actix-web", "warp", "reqwest",
        "p4inz-api",
        # Database implementations
        "sqlx", "tokio-postgres", "postgres", "diesel",
        "p4inz-database",
        # Infrastructure / adapters in general must not be depended on by domain
        "p4inz-infrastructure", "p4inz-ai",
    },
    "crates/application": {
        # Application defines contracts; adapters and infrastructure implement
        # them. Application must not depend back on adapters/infrastructure.
        "serenity", "axum", "actix-web", "warp",
        "p4inz-discord", "p4inz-api",
        "sqlx", "tokio-postgres", "postgres", "diesel",
        "p4inz-database", "p4inz-infrastructure",
    },
    "crates/knowledge": {
        # Same independence requirement as domain: knowledge entities and
        # lifecycle rules must not depend on transport or infrastructure.
        "serenity", "twilight-gateway", "twilight-http", "twilight-model",
        "p4inz-discord",
        "axum", "actix-web", "warp", "reqwest",
        "p4inz-api",
        "sqlx", "tokio-postgres", "postgres", "diesel",
        "p4inz-database",
        "p4inz-infrastructure", "p4inz-ai",
    },
}


def load_workspace_members() -> set[str]:
    data = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    return set(data["workspace"]["members"])


def load_dependency_names(crate_path: str) -> set[str]:
    manifest_path = REPO_ROOT / crate_path / "Cargo.toml"
    data = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    names: set[str] = set()
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        names.update(data.get(table, {}).keys())
    return names


def main() -> int:
    violations: list[str] = []

    members = load_workspace_members()

    for missing in sorted(EXPECTED_MEMBERS - members):
        violations.append(
            f"expected workspace member '{missing}' is missing from Cargo.toml "
            "(see docs/architecture/overview.md)"
        )
    for extra in sorted(members - EXPECTED_MEMBERS):
        violations.append(
            f"workspace member '{extra}' is not part of the documented architecture "
            "(docs/architecture/overview.md) — add an ADR before introducing new crates"
        )

    for crate_path in sorted(members & EXPECTED_MEMBERS):
        deps = load_dependency_names(crate_path)

        for forbidden in sorted(FORBIDDEN_DEPENDENCIES.get(crate_path, set()) & deps):
            violations.append(
                f"{crate_path} depends on forbidden crate '{forbidden}' "
                "(see docs/architecture/dependency-rules.md)"
            )

        if crate_path not in DATABASE_ACCESS_ALLOWED:
            for driver in sorted(DATABASE_DRIVER_CRATES & deps):
                violations.append(
                    f"{crate_path} depends directly on database driver '{driver}'; "
                    "database access must go through p4inz-database/p4inz-infrastructure"
                )

    if violations:
        print("Architecture check failed:\n")
        for v in violations:
            print(f"  - {v}")
        print()
        return 1

    print(f"Architecture check passed ({len(members)} workspace members validated).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
