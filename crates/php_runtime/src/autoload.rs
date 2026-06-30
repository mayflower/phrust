//! Minimal runtime autoload registry for runtime-semantics.

use crate::CallableValue;

/// Deterministic autoload callback registry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AutoloadRegistry {
    callbacks: Vec<CallableValue>,
}

impl AutoloadRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Registers a callback when it is not already present.
    pub fn register(&mut self, callback: CallableValue) -> bool {
        self.register_with_prepend(callback, false)
    }

    /// Registers a callback at the front when requested and not already present.
    pub fn register_with_prepend(&mut self, callback: CallableValue, prepend: bool) -> bool {
        if self.callbacks.contains(&callback) {
            return true;
        }
        if prepend {
            self.callbacks.insert(0, callback);
        } else {
            self.callbacks.push(callback);
        }
        true
    }

    /// Removes a callback and reports whether it existed.
    pub fn unregister(&mut self, callback: &CallableValue) -> bool {
        let Some(position) = self.callbacks.iter().position(|entry| entry == callback) else {
            return false;
        };
        self.callbacks.remove(position);
        true
    }

    /// Removes all registered callbacks.
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// Returns registered callbacks in call order.
    #[must_use]
    pub fn callbacks(&self) -> &[CallableValue] {
        &self.callbacks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoload_registry_preserves_order_and_deduplicates() {
        let mut registry = AutoloadRegistry::new();
        let first = CallableValue::UserFunction {
            name: "first_loader".to_owned(),
        };
        let second = CallableValue::UserFunction {
            name: "second_loader".to_owned(),
        };

        assert!(registry.register(first.clone()));
        assert!(registry.register(second.clone()));
        assert!(registry.register(first.clone()));

        assert_eq!(registry.callbacks(), &[first, second]);
    }

    #[test]
    fn autoload_registry_prepends_new_callbacks_only() {
        let mut registry = AutoloadRegistry::new();
        let first = CallableValue::UserFunction {
            name: "first_loader".to_owned(),
        };
        let second = CallableValue::UserFunction {
            name: "second_loader".to_owned(),
        };

        assert!(registry.register(first.clone()));
        assert!(registry.register_with_prepend(second.clone(), true));
        assert!(registry.register_with_prepend(first.clone(), true));

        assert_eq!(registry.callbacks(), &[second, first]);
    }

    #[test]
    fn autoload_registry_unregisters_existing_callback() {
        let mut registry = AutoloadRegistry::new();
        let callback = CallableValue::UserFunction {
            name: "loader".to_owned(),
        };

        assert!(!registry.unregister(&callback));
        registry.register(callback.clone());
        assert!(registry.unregister(&callback));
        assert!(registry.callbacks().is_empty());
    }

    #[test]
    fn autoload_registry_clear_removes_all_callbacks() {
        let mut registry = AutoloadRegistry::new();
        registry.register(CallableValue::UserFunction {
            name: "first_loader".to_owned(),
        });
        registry.register(CallableValue::UserFunction {
            name: "second_loader".to_owned(),
        });

        registry.clear();

        assert!(registry.callbacks().is_empty());
    }
}
