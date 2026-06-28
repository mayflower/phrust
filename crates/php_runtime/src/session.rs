//! Request-local CLI session state.

use crate::{PhpArray, Value};

/// Session extension disabled.
pub const PHP_SESSION_DISABLED: i64 = 0;
/// Session extension available but no session is active.
pub const PHP_SESSION_NONE: i64 = 1;
/// Session is active for the current request.
pub const PHP_SESSION_ACTIVE: i64 = 2;

/// Deterministic request-local session storage for CLI execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionState {
    status: i64,
    name: String,
    id: String,
    data: PhpArray,
    next_id: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            status: PHP_SESSION_NONE,
            name: "PHPSESSID".to_owned(),
            id: String::new(),
            data: PhpArray::new(),
            next_id: 1,
        }
    }
}

impl SessionState {
    /// Returns the current request-local session status.
    #[must_use]
    pub const fn status(&self) -> i64 {
        self.status
    }

    /// Returns the current session name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Replaces the session name and returns the previous value.
    pub fn replace_name(&mut self, name: impl Into<String>) -> String {
        std::mem::replace(&mut self.name, name.into())
    }

    /// Returns the current session id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Replaces the session id and returns the previous value.
    pub fn replace_id(&mut self, id: impl Into<String>) -> String {
        std::mem::replace(&mut self.id, id.into())
    }

    /// Starts a deterministic CLI session.
    pub fn start(&mut self) {
        if self.id.is_empty() {
            self.id = format!("phrustcli{:08}", self.next_id);
            self.next_id = self.next_id.saturating_add(1);
        }
        self.status = PHP_SESSION_ACTIVE;
    }

    /// Destroys the current deterministic CLI session.
    pub fn destroy(&mut self) -> bool {
        if self.status != PHP_SESSION_ACTIVE {
            return false;
        }
        self.status = PHP_SESSION_NONE;
        self.id.clear();
        self.data = PhpArray::new();
        true
    }

    /// Returns a copy of the current `$_SESSION` array.
    #[must_use]
    pub fn data(&self) -> PhpArray {
        self.data.clone()
    }

    /// Replaces the stored `$_SESSION` array.
    pub fn set_data(&mut self, data: PhpArray) {
        self.data = data;
    }

    /// Returns the stored session data as a PHP value.
    #[must_use]
    pub fn data_value(&self) -> Value {
        Value::Array(self.data())
    }
}

#[cfg(test)]
mod tests {
    use super::{PHP_SESSION_ACTIVE, PHP_SESSION_NONE, SessionState};

    #[test]
    fn session_state_tracks_cli_lifecycle() {
        let mut state = SessionState::default();

        assert_eq!(state.status(), PHP_SESSION_NONE);
        assert_eq!(state.name(), "PHPSESSID");
        assert_eq!(state.id(), "");

        state.start();
        assert_eq!(state.status(), PHP_SESSION_ACTIVE);
        assert_eq!(state.id(), "phrustcli00000001");

        assert!(state.destroy());
        assert_eq!(state.status(), PHP_SESSION_NONE);
        assert_eq!(state.id(), "");
        assert!(!state.destroy());
    }
}
