//! Explicit Prompt 01 placeholder for future VM work.

/// Describes a Phase 4 VM area that is intentionally not implemented yet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Phase4VmTodo {
    area: &'static str,
}

impl Phase4VmTodo {
    /// Creates a new documented placeholder.
    #[must_use]
    pub const fn new(area: &'static str) -> Self {
        Self { area }
    }

    /// Returns the planned area name.
    #[must_use]
    pub const fn area(&self) -> &'static str {
        self.area
    }
}

/// Stable status string used by early wiring tests.
#[must_use]
pub const fn vm_skeleton_status() -> &'static str {
    "phase4-vm-skeleton"
}
