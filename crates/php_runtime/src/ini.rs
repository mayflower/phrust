//! Deterministic standard-library INI/config registry.

/// One supported INI entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IniEntrySnapshot {
    /// Extension that owns this INI option.
    pub extension: &'static str,
    /// Canonical INI option name.
    pub name: &'static str,
    /// Engine default value.
    pub global_value: String,
    /// Current per-request value.
    pub local_value: String,
    /// PHP-style access mask. The standard-library MVP treats supported entries as all.
    pub access: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IniEntry {
    extension: &'static str,
    name: &'static str,
    global_value: &'static str,
    local_value: String,
    access: i64,
}

/// Small, deterministic registry for Composer-typical INI checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IniRegistry {
    entries: Vec<IniEntry>,
}

impl Default for IniRegistry {
    fn default() -> Self {
        Self {
            entries: default_entries()
                .into_iter()
                .map(|(extension, name, value, access)| IniEntry {
                    extension,
                    name,
                    global_value: value,
                    local_value: value.to_owned(),
                    access,
                })
                .collect(),
        }
    }
}

impl IniRegistry {
    /// Returns a stable snapshot for supported options.
    #[must_use]
    pub fn entries(&self) -> Vec<IniEntrySnapshot> {
        self.entries
            .iter()
            .map(|entry| IniEntrySnapshot {
                extension: entry.extension,
                name: entry.name,
                global_value: entry.global_value.to_owned(),
                local_value: entry.local_value.clone(),
                access: entry.access,
            })
            .collect()
    }

    /// Returns a stable snapshot for options owned by an extension.
    #[must_use]
    pub fn entries_for_extension(&self, extension: &str) -> Vec<IniEntrySnapshot> {
        self.entries()
            .into_iter()
            .filter(|entry| entry.extension.eq_ignore_ascii_case(extension))
            .collect()
    }

    /// Reads the current per-request value.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.lookup(name).map(|entry| entry.local_value.as_str())
    }

    /// Reads the engine default value.
    #[must_use]
    pub fn cfg_var(&self, name: &str) -> Option<&str> {
        self.lookup(name).map(|entry| entry.global_value)
    }

    /// Overrides a supported option and returns its previous local value.
    pub fn set(&mut self, name: &str, value: impl Into<String>) -> Option<String> {
        let entry = self.lookup_mut(name)?;
        if entry.access & 1 == 0 {
            return None;
        }
        let previous = std::mem::replace(&mut entry.local_value, value.into());
        Some(previous)
    }

    fn lookup(&self, name: &str) -> Option<&IniEntry> {
        self.entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut IniEntry> {
        self.entries
            .iter_mut()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }
}

fn default_entries() -> [(&'static str, &'static str, &'static str, i64); 19] {
    [
        ("standard", "arg_separator.input", "&", 7),
        ("date", "date.timezone", "UTC", 7),
        ("standard", "default_charset", "UTF-8", 7),
        ("core", "display_errors", "1", 7),
        ("core", "error_reporting", "-1", 7),
        ("ffi", "ffi.enable", "preload", 4),
        ("ffi", "ffi.preload", "", 4),
        ("standard", "file_uploads", "1", 7),
        ("standard", "ignore_user_abort", "0", 7),
        ("standard", "include_path", ".", 7),
        ("standard", "max_file_uploads", "20", 7),
        ("standard", "max_input_nesting_level", "64", 7),
        ("standard", "max_input_vars", "1000", 7),
        ("core", "memory_limit", "128M", 7),
        ("standard", "post_max_size", "8M", 7),
        ("core", "precision", "14", 7),
        ("core", "serialize_precision", "-1", 7),
        ("standard", "upload_max_filesize", "2M", 7),
        ("standard", "upload_tmp_dir", "", 7),
    ]
}

#[cfg(test)]
mod tests {
    use super::IniRegistry;

    #[test]
    fn ini_registry_reads_and_overrides_supported_values() {
        let mut registry = IniRegistry::default();

        assert_eq!(registry.get("INCLUDE_PATH"), Some("."));
        assert_eq!(registry.cfg_var("include_path"), Some("."));
        assert_eq!(registry.set("include_path", "lib"), Some(".".to_owned()));
        assert_eq!(registry.get("include_path"), Some("lib"));
        assert_eq!(registry.cfg_var("include_path"), Some("."));
        assert_eq!(registry.get("file_uploads"), Some("1"));
        assert_eq!(registry.get("upload_tmp_dir"), Some(""));
        assert_eq!(registry.get("upload_max_filesize"), Some("2M"));
        assert_eq!(registry.get("post_max_size"), Some("8M"));
        assert_eq!(registry.get("max_file_uploads"), Some("20"));
        assert_eq!(registry.get("ffi.enable"), Some("preload"));
        assert_eq!(registry.cfg_var("ffi.preload"), Some(""));
        assert_eq!(registry.set("ffi.enable", "1"), None);
        assert_eq!(registry.get("ffi.enable"), Some("preload"));
        assert_eq!(registry.set("missing", "value"), None);
    }

    #[test]
    fn ini_registry_entries_are_deterministic() {
        let registry = IniRegistry::default();
        let names = registry
            .entries()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "arg_separator.input",
                "date.timezone",
                "default_charset",
                "display_errors",
                "error_reporting",
                "ffi.enable",
                "ffi.preload",
                "file_uploads",
                "ignore_user_abort",
                "include_path",
                "max_file_uploads",
                "max_input_nesting_level",
                "max_input_vars",
                "memory_limit",
                "post_max_size",
                "precision",
                "serialize_precision",
                "upload_max_filesize",
                "upload_tmp_dir"
            ]
        );

        let ffi_names = registry
            .entries_for_extension("FFI")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(ffi_names, vec!["ffi.enable", "ffi.preload"]);
    }
}
