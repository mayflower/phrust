//! Stable typed IDs for runtime IR arenas.

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            serde::Deserialize,
            serde::Serialize,
        )]
        pub struct $name(u32);

        impl $name {
            /// Creates a new ID from a zero-based index.
            #[must_use]
            pub const fn new(index: u32) -> Self {
                Self(index)
            }

            /// Returns the zero-based index for arena lookup.
            #[must_use]
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            /// Returns the raw integer representation for snapshots.
            #[must_use]
            pub const fn raw(self) -> u32 {
                self.0
            }
        }
    };
}

id_type!(UnitId);
id_type!(FileId);
id_type!(FunctionId);
id_type!(ClassId);
id_type!(BlockId);
id_type!(InstrId);
id_type!(LocalId);
id_type!(RegId);
id_type!(ConstId);
