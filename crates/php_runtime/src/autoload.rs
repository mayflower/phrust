//! Minimal runtime autoload registry for Phase 5.

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
        if self.callbacks.contains(&callback) {
            return true;
        }
        self.callbacks.push(callback);
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
}
