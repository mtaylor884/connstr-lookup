// Synonym groups for ADO.NET / ODBC connection string keys. Different
// drivers and config files spell the same setting differently (Server vs.
// Data Source, Uid vs. User Id), so `get` needs to treat a group as one key
// rather than requiring the caller to guess which spelling is in use.
const GROUPS: &[&[&str]] = &[
    &["server", "address", "addr", "network address", "data source"],
    &["database", "initial catalog"],
    &["user id", "uid", "user", "username", "user name"],
    &["password", "pwd"],
];

// Returns the canonical name for key's synonym group, or key itself
// (lowercased) if it isn't part of any known group. Two keys are the same
// setting exactly when canonical() returns the same string for both.
pub fn canonical(key: &str) -> String {
    let lower = key.to_ascii_lowercase();
    for group in GROUPS {
        if group.contains(&lower.as_str()) {
            return group[0].to_string();
        }
    }
    lower
}
