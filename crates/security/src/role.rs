use std::collections::HashSet;
use std::fmt;

use thiserror::Error;

use crate::permission::Permission;

/// Maximum accepted length for a [`RoleName`], in characters.
pub const ROLE_NAME_MAX_LEN: usize = 100;

/// A role's display/identifying name (e.g. `"administrator"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleName(String);

impl RoleName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, RoleNameError> {
        let trimmed = raw.as_ref().trim();

        if trimmed.is_empty() {
            return Err(RoleNameError::Empty);
        }
        if trimmed.chars().count() > ROLE_NAME_MAX_LEN {
            return Err(RoleNameError::TooLong { max: ROLE_NAME_MAX_LEN });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RoleNameError {
    #[error("role name must not be empty")]
    Empty,
    #[error("role name must be at most {max} characters")]
    TooLong { max: usize },
}

/// A named collection of [`Permission`]s.
///
/// Mapping an external identity (e.g. a Discord guild role) onto a `Role`
/// is deliberately out of scope here — that translation belongs to the
/// adapter that owns that identity source (Discord Permissions, Milestone
/// 14; Web Authentication, Milestone 40).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    name: RoleName,
    permissions: HashSet<Permission>,
}

impl Role {
    pub fn new(name: RoleName, permissions: impl IntoIterator<Item = Permission>) -> Self {
        Self { name, permissions: permissions.into_iter().collect() }
    }

    pub fn name(&self) -> &RoleName {
        &self.name
    }

    pub fn permissions(&self) -> impl Iterator<Item = &Permission> {
        self.permissions.iter()
    }

    pub fn grants(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permission(raw: &str) -> Permission {
        Permission::parse(raw).unwrap()
    }

    #[test]
    fn role_name_rejects_empty() {
        assert_eq!(RoleName::parse(""), Err(RoleNameError::Empty));
    }

    #[test]
    fn grants_reports_membership() {
        let role = Role::new(
            RoleName::parse("moderator").unwrap(),
            [permission("project:register"), permission("project:archive")],
        );

        assert!(role.grants(&permission("project:register")));
        assert!(!role.grants(&permission("admin:manage")));
    }

    #[test]
    fn role_with_no_permissions_grants_nothing() {
        let role = Role::new(RoleName::parse("guest").unwrap(), []);
        assert!(!role.grants(&permission("project:register")));
    }
}
