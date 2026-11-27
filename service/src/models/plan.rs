//! What a tenant's plan means for its people.
//!
//! The plan provider (`plans`) owns the plan and says only whether paid features
//! are locked, how many seats there are and which features are paid or write; IAM
//! knows who is an owner and applies the rules here. Nothing in this file talks to
//! the provider or the database.

use super::roles;

/// A tenant's plan state, as the plan provider answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The tenant is free or overdue: paid features are locked and only an
    /// owner may write.
    pub locked: bool,
    /// How many seats (active members plus pending invitations) are allowed.
    pub seat_limit: i64,
}

/// One entry of the plan provider's feature catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    pub key: String,
    pub name: String,
    pub paid: bool,
    pub write: bool,
}

/// Whether `member_roles` include the built-in owner role.
#[must_use]
pub fn is_owner(member_roles: &[roles::Model]) -> bool {
    member_roles
        .iter()
        .any(|role| role.is_system() && role.name == roles::OWNER)
}

/// `read_only` for a non-owner of a locked tenant, otherwise `full`.
#[must_use]
pub const fn access(locked: bool, owner: bool) -> &'static str {
    if locked && !owner {
        "read_only"
    } else {
        "full"
    }
}

fn find<'a>(catalogue: &'a [Feature], key: &str) -> Option<&'a Feature> {
    catalogue.iter().find(|feature| feature.key == key)
}

/// Whether the permission `key` changes data. A key the catalogue lacks is a
/// write unless its action, the part after the last dot, is `read`.
#[must_use]
pub fn is_write(catalogue: &[Feature], key: &str) -> bool {
    find(catalogue, key).map_or_else(
        || {
            key.rsplit_once('.')
                .is_none_or(|(_, action)| action != "read")
        },
        |feature| feature.write,
    )
}

/// The paid feature that guards a call needing one of `keys`, if every key is
/// a paid feature. A call that a free feature (or a key the catalogue lacks)
/// also allows is never locked.
#[must_use]
pub fn paid_feature_for<'a>(catalogue: &'a [Feature], keys: &[&str]) -> Option<&'a Feature> {
    let mut found = None;
    for key in keys {
        let feature = find(catalogue, key)?;
        if !feature.paid {
            return None;
        }
        found = found.or(Some(feature));
    }
    found
}

/// What a person may really do: `permissions` without the paid ones (when `locked`)
/// and, for a read-only person, without every write.
///
/// The all-permissions key `*` is spelled out first so it can be narrowed.
#[must_use]
pub fn allowed_permissions(
    catalogue: &[Feature],
    permissions: &[String],
    locked: bool,
    read_only: bool,
) -> Vec<String> {
    if !locked && !read_only {
        return permissions.to_vec();
    }
    let mut keys: Vec<String> = if permissions.iter().any(|key| key == roles::ALL_PERMISSIONS) {
        catalogue
            .iter()
            .map(|feature| feature.key.clone())
            .collect()
    } else {
        permissions.to_vec()
    };
    keys.retain(|key| {
        let paid_and_locked = locked && find(catalogue, key).is_some_and(|feature| feature.paid);
        let blocked = read_only && is_write(catalogue, key);
        !paid_and_locked && !blocked
    });
    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(key: &str, paid: bool, write: bool) -> Feature {
        Feature {
            key: key.to_string(),
            name: key.to_string(),
            paid,
            write,
        }
    }

    fn catalogue() -> Vec<Feature> {
        vec![
            feature("accounts.read", false, false),
            feature("accounts.manage", false, true),
            feature("bank.read", true, false),
            feature("bank.manage", true, true),
        ]
    }

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_call_is_locked_only_when_every_key_is_paid() {
        let catalogue = catalogue();
        assert_eq!(
            paid_feature_for(&catalogue, &["bank.read", "bank.manage"]).map(|f| f.key.as_str()),
            Some("bank.read")
        );
        assert!(paid_feature_for(&catalogue, &["accounts.read"]).is_none());
        assert!(paid_feature_for(&catalogue, &["billing.manage"]).is_none());
        assert!(paid_feature_for(&catalogue, &["bank.read", "accounts.read"]).is_none());
    }

    #[test]
    fn locked_permissions_are_left_out() {
        let catalogue = catalogue();
        let all = keys(&["accounts.read", "bank.read", "bank.manage"]);
        assert_eq!(
            allowed_permissions(&catalogue, &all, true, false),
            keys(&["accounts.read"])
        );
        assert_eq!(allowed_permissions(&catalogue, &all, false, false), all);
    }

    #[test]
    fn a_read_only_user_loses_every_write() {
        let catalogue = catalogue();
        let all = keys(&["accounts.read", "accounts.manage", "bank.manage"]);
        assert_eq!(
            allowed_permissions(&catalogue, &all, false, true),
            keys(&["accounts.read"])
        );
    }

    #[test]
    fn the_all_permissions_key_is_spelled_out_when_narrowed() {
        assert_eq!(
            allowed_permissions(&catalogue(), &keys(&["*"]), true, false),
            keys(&["accounts.manage", "accounts.read"])
        );
    }

    #[test]
    fn only_a_non_owner_of_a_locked_tenant_is_read_only() {
        assert_eq!(access(true, false), "read_only");
        assert_eq!(access(true, true), "full");
        assert_eq!(access(false, false), "full");
    }

    #[test]
    fn unknown_keys_are_writes_unless_they_end_in_read() {
        let catalogue = catalogue();
        assert!(is_write(&catalogue, "widgets.manage"));
        assert!(!is_write(&catalogue, "widgets.read"));
    }
}
