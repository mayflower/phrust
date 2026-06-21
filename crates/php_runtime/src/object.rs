//! Minimal object storage and class metadata for Phase 4.

use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

/// Minimal runtime type adapter used by the VM for Phase 3 annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeType {
    /// `int`
    Int,
    /// `float`
    Float,
    /// `string`
    String,
    /// `array`
    Array,
    /// `callable`
    Callable,
    /// `object`
    Object,
    /// `bool`
    Bool,
    /// `null`
    Null,
    /// `void`
    Void,
    /// `mixed`
    Mixed,
    /// Class-like type.
    Class { name: String },
    /// Nullable simple type.
    Nullable { inner: Box<RuntimeType> },
}

/// Runtime class table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassEntry {
    /// Canonical class lookup name.
    pub name: String,
    /// Runtime-visible instance methods.
    pub methods: Vec<ClassMethodEntry>,
    /// Runtime-visible instance properties.
    pub properties: Vec<ClassPropertyEntry>,
    /// Raw IR function ID for `__construct`, when present.
    pub constructor_id: Option<u32>,
    /// Class declaration flags.
    pub flags: ClassFlags,
}

/// Class declaration flags.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassFlags {
    /// Abstract class.
    pub is_abstract: bool,
    /// Final class.
    pub is_final: bool,
    /// Readonly class.
    pub is_readonly: bool,
}

/// Runtime method table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMethodEntry {
    /// Normalized method lookup name.
    pub name: String,
    /// Raw IR function ID for the method body.
    pub function_id: u32,
    /// Method flags.
    pub flags: ClassMethodFlags,
}

/// Runtime method flags.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassMethodFlags {
    /// Static method.
    pub is_static: bool,
    /// Private method.
    pub is_private: bool,
    /// Protected method.
    pub is_protected: bool,
    /// Abstract method.
    pub is_abstract: bool,
}

/// Runtime property table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassPropertyEntry {
    /// Property name without `$`.
    pub name: String,
    /// Default value for new instances.
    pub default: Value,
    /// Optional runtime type enforced on property writes.
    pub type_: Option<RuntimeType>,
    /// Property flags.
    pub flags: ClassPropertyFlags,
}

/// Runtime property flags.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassPropertyFlags {
    /// Static property.
    pub is_static: bool,
    /// Private property.
    pub is_private: bool,
    /// Protected property.
    pub is_protected: bool,
    /// Readonly property.
    pub is_readonly: bool,
    /// Typed property.
    pub is_typed: bool,
}

#[derive(Debug)]
struct ObjectStorage {
    class_name: String,
    properties: HashMap<String, Value>,
}

/// Reference to runtime object storage.
#[derive(Clone)]
pub struct ObjectRef {
    id: u64,
    storage: Rc<RefCell<ObjectStorage>>,
}

impl ObjectRef {
    /// Creates an object with properties initialized from the class entry.
    #[must_use]
    pub fn new(class: &ClassEntry) -> Self {
        let properties = class
            .properties
            .iter()
            .filter(|property| !property.flags.is_static)
            .map(|property| (property.name.clone(), property.default.clone()))
            .collect();
        Self {
            id: NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed),
            storage: Rc::new(RefCell::new(ObjectStorage {
                class_name: class.name.clone(),
                properties,
            })),
        }
    }

    /// Returns the stable object identity for tests and diagnostics.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the object's class name.
    #[must_use]
    pub fn class_name(&self) -> String {
        self.storage.borrow().class_name.clone()
    }

    /// Creates a new object identity with a shallow copy of the property map.
    #[must_use]
    pub fn clone_shallow(&self) -> Self {
        let storage = self.storage.borrow();
        Self {
            id: NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed),
            storage: Rc::new(RefCell::new(ObjectStorage {
                class_name: storage.class_name.clone(),
                properties: storage.properties.clone(),
            })),
        }
    }

    /// Reads a property value.
    #[must_use]
    pub fn get_property(&self, name: &str) -> Option<Value> {
        self.storage.borrow().properties.get(name).cloned()
    }

    /// Writes a property value.
    pub fn set_property(&self, name: impl Into<String>, value: Value) {
        self.storage
            .borrow_mut()
            .properties
            .insert(name.into(), value);
    }
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectRef")
            .field("id", &self.id)
            .field("class_name", &self.class_name())
            .finish()
    }
}

impl PartialEq for ObjectRef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ObjectRef {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_refs_preserve_identity_and_independent_properties() {
        let class = ClassEntry {
            name: "box".to_owned(),
            methods: Vec::new(),
            properties: vec![ClassPropertyEntry {
                name: "value".to_owned(),
                default: Value::Null,
                type_: None,
                flags: ClassPropertyFlags::default(),
            }],
            constructor_id: None,
            flags: ClassFlags::default(),
        };
        let one = ObjectRef::new(&class);
        let two = ObjectRef::new(&class);
        one.set_property("value", Value::Int(1));
        two.set_property("value", Value::Int(2));

        assert_ne!(one, two);
        assert_eq!(one.get_property("value"), Some(Value::Int(1)));
        assert_eq!(two.get_property("value"), Some(Value::Int(2)));
        assert_eq!(one.class_name(), "box");
    }

    #[test]
    fn object_clone_shallow_copies_properties_with_new_identity() {
        let class = ClassEntry {
            name: "box".to_owned(),
            methods: Vec::new(),
            properties: vec![ClassPropertyEntry {
                name: "value".to_owned(),
                default: Value::Null,
                type_: None,
                flags: ClassPropertyFlags::default(),
            }],
            constructor_id: None,
            flags: ClassFlags::default(),
        };
        let original = ObjectRef::new(&class);
        original.set_property("value", Value::Int(1));
        let copy = original.clone_shallow();

        assert_ne!(original, copy);
        assert_eq!(copy.class_name(), "box");
        assert_eq!(copy.get_property("value"), Some(Value::Int(1)));
        copy.set_property("value", Value::Int(2));
        assert_eq!(original.get_property("value"), Some(Value::Int(1)));
        assert_eq!(copy.get_property("value"), Some(Value::Int(2)));
    }
}
