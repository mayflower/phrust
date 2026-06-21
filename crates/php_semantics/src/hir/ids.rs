//! Typed HIR and semantic IDs.

/// Common behavior for typed HIR IDs.
pub trait HirId: Copy + Eq {
    /// Creates an ID from an arena index.
    fn from_usize(index: usize) -> Self;

    /// Returns the arena index.
    fn to_usize(self) -> usize;
}

macro_rules! define_id {
    ($name:ident) => {
        #[doc = concat!("Typed semantic ID for `", stringify!($name), "`.")]
        #[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(usize);

        impl $name {
            /// Creates an ID from a raw arena index.
            #[must_use]
            pub const fn from_raw(index: usize) -> Self {
                Self(index)
            }

            /// Returns the raw arena index for deterministic snapshots.
            #[must_use]
            pub const fn raw(self) -> usize {
                self.0
            }
        }

        impl HirId for $name {
            fn from_usize(index: usize) -> Self {
                Self(index)
            }

            fn to_usize(self) -> usize {
                self.0
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }
    };
}

define_id!(ModuleId);
define_id!(NamespaceId);
define_id!(DeclId);
define_id!(FunctionId);
define_id!(ClassLikeId);
define_id!(TraitUseId);
define_id!(EnumCaseId);
define_id!(MethodId);
define_id!(PropertyId);
define_id!(ConstId);
define_id!(ParamId);
define_id!(ExprId);
define_id!(StmtId);
define_id!(ConstExprId);
define_id!(TypeId);
define_id!(AttributeId);
define_id!(ScopeId);
define_id!(SymbolId);
define_id!(NameId);
