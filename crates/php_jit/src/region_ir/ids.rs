//! Compact index identifiers for region IR tables.

macro_rules! index_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an identifier from a raw table index.
            #[must_use]
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            /// Returns the raw stable identifier value.
            #[must_use]
            pub const fn raw(self) -> u32 {
                self.0
            }

            /// Returns this identifier as a `usize` table index.
            #[must_use]
            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

index_id!(RegionId);
index_id!(NodeId);
index_id!(ConstId);
index_id!(SnapshotId);
index_id!(EntryId);
index_id!(ExitId);
index_id!(VmSlotId);
