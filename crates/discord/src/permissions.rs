use std::collections::HashMap;

use p4inz_security::{PermissionSet, Role};
use serenity::model::id::RoleId;

/// Maps a guild member's Discord role ids onto P4inz [`Role`]s, then unions
/// their permissions into a [`PermissionSet`]
/// (`docs/development/implementation_plan.md` section 12: "Discord Role
/// Mapping -> P4inz Permission").
///
/// The Discord-role-id -> P4inz-role assignments are injected rather than
/// hardcoded, since P4inz has no per-guild configuration storage yet — no
/// milestone up to this point in the roadmap adds it. Whichever milestone
/// eventually builds guild administration is expected to construct a
/// `GuildRoleMapping` from persisted configuration and hand it to whatever
/// needs to authorize a member's action; this only defines the mapping
/// mechanism itself.
#[derive(Debug, Clone, Default)]
pub struct GuildRoleMapping {
    assignments: HashMap<RoleId, Role>,
}

impl GuildRoleMapping {
    pub fn new(assignments: HashMap<RoleId, Role>) -> Self {
        Self { assignments }
    }

    /// Resolves the [`PermissionSet`] granted by the given Discord role
    /// ids — typically a guild member's `roles` field. Role ids with no
    /// assignment contribute nothing; an unmapped member resolves to an
    /// empty (fail-closed) [`PermissionSet`].
    pub fn resolve(&self, member_role_ids: &[RoleId]) -> PermissionSet {
        let roles = member_role_ids.iter().filter_map(|id| self.assignments.get(id));
        PermissionSet::from_roles(roles)
    }
}

#[cfg(test)]
mod tests {
    use p4inz_security::{Permission, RoleName};

    use super::*;

    fn permission(raw: &str) -> Permission {
        Permission::parse(raw).unwrap()
    }

    #[test]
    fn unmapped_roles_resolve_to_no_permissions() {
        let mapping = GuildRoleMapping::default();
        let permissions = mapping.resolve(&[RoleId::new(1)]);
        assert!(!permissions.contains(&permission("project:register")));
    }

    #[test]
    fn mapped_role_grants_its_permissions() {
        let moderator =
            Role::new(RoleName::parse("moderator").unwrap(), [permission("project:register")]);
        let mapping = GuildRoleMapping::new(HashMap::from([(RoleId::new(42), moderator)]));

        let permissions = mapping.resolve(&[RoleId::new(42)]);
        assert!(permissions.contains(&permission("project:register")));
    }

    #[test]
    fn unions_permissions_across_multiple_mapped_roles() {
        let moderator =
            Role::new(RoleName::parse("moderator").unwrap(), [permission("project:register")]);
        let support =
            Role::new(RoleName::parse("support").unwrap(), [permission("ticket:respond")]);
        let mapping = GuildRoleMapping::new(HashMap::from([
            (RoleId::new(1), moderator),
            (RoleId::new(2), support),
        ]));

        let permissions = mapping.resolve(&[RoleId::new(1), RoleId::new(2), RoleId::new(999)]);
        assert!(permissions.contains(&permission("project:register")));
        assert!(permissions.contains(&permission("ticket:respond")));
    }
}
