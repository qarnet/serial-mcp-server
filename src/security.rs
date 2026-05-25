use glob::Pattern;
use tracing::info;

/// Security manager that handles port allowlist checks.
///
/// If the allowlist is empty, all ports are allowed.
#[derive(Clone)]
pub struct SecurityManager {
    allowlist: Vec<Pattern>,
}

impl SecurityManager {
    /// Create a new security manager from the `SERIAL_MCP_ALLOWLIST` environment variable.
    pub fn from_env() -> Self {
        let allowlist = Self::parse_allowlist_env();
        Self { allowlist }
    }

    /// Check if a port matches the allowlist. Empty allowlist = allow all.
    pub fn is_port_allowed(&self, port: &str) -> bool {
        if self.allowlist.is_empty() {
            return true;
        }
        self.allowlist.iter().any(|pattern| pattern.matches(port))
    }

    /// Human-readable summary of allowlist patterns for error messages.
    pub fn allowlist_summary(&self) -> String {
        if self.allowlist.is_empty() {
            "(all ports allowed)".to_string()
        } else {
            self.allowlist
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    /// Parse `SERIAL_MCP_ALLOWLIST` environment variable into glob patterns.
    /// Returns empty Vec if not set (allowing all ports).
    fn parse_allowlist_env() -> Vec<Pattern> {
        let env_val = std::env::var("SERIAL_MCP_ALLOWLIST").unwrap_or_default();
        if env_val.is_empty() {
            return Vec::new();
        }

        let patterns: Vec<Pattern> = env_val
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| Pattern::new(s).ok())
            .collect();

        if !patterns.is_empty() {
            info!(
                "Port allowlist active: {}",
                patterns
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        patterns
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::from_env()
    }
}
