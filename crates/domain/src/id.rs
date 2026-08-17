use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use uuid::Uuid;

/// A UUID identifier scoped to entity type `T`.
///
/// The phantom type parameter prevents an identifier for one entity from
/// being passed where an identifier for a different entity is expected,
/// without requiring `T` to implement any trait (`fn() -> T` avoids the
/// bogus `T: Trait` bounds that `#[derive]` would otherwise add).
pub struct Id<T> {
    value: Uuid,
    entity: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Generates a new random identifier.
    pub fn new() -> Self {
        Self { value: Uuid::new_v4(), entity: PhantomData }
    }

    /// Wraps an existing UUID as an identifier for entity type `T`.
    pub fn from_uuid(value: Uuid) -> Self {
        Self { value, entity: PhantomData }
    }

    /// Returns the underlying UUID.
    pub fn into_uuid(self) -> Uuid {
        self.value
    }

    /// Returns a reference to the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.value
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.value)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker;

    #[test]
    fn new_generates_distinct_ids() {
        let a: Id<Marker> = Id::new();
        let b: Id<Marker> = Id::new();
        assert_ne!(a, b);
    }

    #[test]
    fn from_uuid_roundtrips() {
        let uuid = Uuid::new_v4();
        let id: Id<Marker> = Id::from_uuid(uuid);
        assert_eq!(id.into_uuid(), uuid);
    }

    #[test]
    fn display_matches_uuid() {
        let uuid = Uuid::new_v4();
        let id: Id<Marker> = Id::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn id_is_copy() {
        let a: Id<Marker> = Id::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn as_uuid_returns_the_wrapped_value() {
        let uuid = Uuid::new_v4();
        let id: Id<Marker> = Id::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn default_generates_a_usable_id() {
        let a: Id<Marker> = Id::default();
        let b: Id<Marker> = Id::default();
        // Each call generates a fresh random id, the same as `Id::new` —
        // `Default` is not a fixed/nil placeholder value.
        assert_ne!(a, b);
    }

    #[test]
    fn debug_format_matches_the_documented_shape() {
        let uuid = Uuid::new_v4();
        let id: Id<Marker> = Id::from_uuid(uuid);
        assert_eq!(format!("{id:?}"), format!("Id({uuid})"));
    }

    #[test]
    fn equal_ids_hash_equally() {
        use std::collections::HashSet;

        let uuid = Uuid::new_v4();
        let a: Id<Marker> = Id::from_uuid(uuid);
        let b: Id<Marker> = Id::from_uuid(uuid);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(!set.insert(b), "a second, equal id should already be present in the set");
    }
}
