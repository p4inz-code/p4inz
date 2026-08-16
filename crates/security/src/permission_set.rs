use std::collections::HashSet;

use crate::permission::Permission;
use crate::role::Role;

/// The permissions a subject currently holds — the union of its granted
/// roles' permissions.
///
/// This is a pure membership check with no notion of "who" holds it and no
/// default-allow path: an empty [`PermissionSet`] grants nothing
/// (`docs/development/implementation_plan.md` section 12: "Authorization
/// MUST fail closed").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionSet(HashSet<Permission>);

impl PermissionSet {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_roles<'a>(roles: impl IntoIterator<Item = &'a Role>) -> Self {
        let mut permissions = HashSet::new();
        for role in roles {
            permissions.extend(role.permissions().cloned());
        }
        Self(permissions)
    }

    pub fn contains(&self, permission: &Permission) -> bool {
        self.0.contains(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::RoleName;

    fn permission(raw: &str) -> Permission {
        Permission::parse(raw).unwrap()
    }

    #[test]
    fn empty_set_grants_nothing() {
        let set = PermissionSet::empty();
        assert!(!set.contains(&permission("project:register")));
    }

    #[test]
    fn from_roles_unions_permissions_across_roles() {
        let moderator =
            Role::new(RoleName::parse("moderator").unwrap(), [permission("project:register")]);
        let support =
            Role::new(RoleName::parse("support").unwrap(), [permission("ticket:respond")]);

        let set = PermissionSet::from_roles([&moderator, &support]);

        assert!(set.contains(&permission("project:register")));
        assert!(set.contains(&permission("ticket:respond")));
        assert!(!set.contains(&permission("admin:manage")));
    }

    #[test]
    fn from_roles_with_no_roles_grants_nothing() {
        let set = PermissionSet::from_roles([]);
        assert!(!set.contains(&permission("project:register")));
    }
}
