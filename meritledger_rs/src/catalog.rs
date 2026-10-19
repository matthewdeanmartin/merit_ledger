//! The built-in practice kinds (`catalog/practices.json`), compiled into
//! the firmware and decoded once with the framework's JSON decoder. Keys
//! are stable: entries store them, so never rename one; retire it instead.
use crate::messages::{KindCatalog, StoredKind};
use miniframework::wire::{self, Format};
use std::sync::OnceLock;

const JSON: &str = include_str!("../catalog/practices.json");

/// Pinned on a Today page until the person chooses their own.
pub const DEFAULT_PINS: [&str; 6] = [
    "meditation",
    "chanting",
    "prostrations",
    "reading",
    "precepts",
    "vegetarian",
];

pub fn builtins() -> &'static [StoredKind] {
    static KINDS: OnceLock<Vec<StoredKind>> = OnceLock::new();
    KINDS.get_or_init(|| {
        let catalog: KindCatalog = wire::decode(Format::Json, JSON.as_bytes())
            .expect("catalog/practices.json is valid (tested)");
        catalog.kinds
    })
}

pub fn builtin(key: &str) -> Option<&'static StoredKind> {
    builtins().iter().find(|k| k.key == key)
}

/// What someone else sees in place of a `generic` entry.
pub fn generic_label(kind: &StoredKind) -> String {
    match (kind.polarity.as_str(), kind.category.as_str()) {
        (_, "repentance") => "A repentance practice".into(),
        ("demerit", _) => "A moment of reflection".into(),
        (_, category) => format!("A {category} practice"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_well_formed() {
        let kinds = builtins();
        assert!(kinds.len() >= 20);
        let keys: HashSet<_> = kinds.iter().map(|k| k.key.as_str()).collect();
        assert_eq!(keys.len(), kinds.len(), "keys are unique");
        for k in kinds {
            assert!(!k.name.is_empty() && !k.icon.is_empty(), "{}", k.key);
            assert!(
                k.key.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{}",
                k.key
            );
            assert!(
                !k.key.starts_with('c') || !k.key[1..].chars().all(|c| c.is_ascii_digit()),
                "c<n> is for custom kinds"
            );
            assert!(
                matches!(k.polarity.as_str(), "merit" | "demerit"),
                "{}",
                k.key
            );
            assert!(
                matches!(k.unit.as_str(), "minutes" | "count" | "once"),
                "{}",
                k.key
            );
            assert!(
                matches!(k.visibility.as_str(), "public" | "generic" | "private"),
                "{}",
                k.key
            );
            assert_eq!(k.unit == "once", k.default_amount == 0, "{}", k.key);
            assert_eq!(k.owner, 0);
        }
        for pin in DEFAULT_PINS {
            assert!(builtin(pin).is_some(), "{pin}");
        }
    }

    #[test]
    fn demerits_and_repentance_default_to_private_or_generic() {
        for k in builtins() {
            if k.polarity == "demerit" {
                assert_eq!(k.visibility, "private", "{}", k.key);
                assert!(!k.content, "{}", k.key);
            }
        }
        assert_eq!(builtin("repentance").unwrap().visibility, "generic");
    }

    #[test]
    fn generic_labels() {
        assert_eq!(
            generic_label(builtin("repentance").unwrap()),
            "A repentance practice"
        );
        assert_eq!(
            generic_label(builtin("anger").unwrap()),
            "A moment of reflection"
        );
        assert_eq!(
            generic_label(builtin("meditation").unwrap()),
            "A meditation practice"
        );
    }
}
