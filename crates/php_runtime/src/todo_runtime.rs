//! Explicit runtime placeholder for future runtime work.

/// Describes a runtime area that is intentionally not implemented yet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTodo {
    area: &'static str,
}

impl RuntimeTodo {
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
pub const fn runtime_skeleton_status() -> &'static str {
    "runtime-skeleton"
}
