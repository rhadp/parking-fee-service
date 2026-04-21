//! LOCKING_SERVICE — ASIL-B rated lock/unlock command processor.
//!
//! Start with: locking-service serve
//! Connects to DATA_BROKER, validates safety constraints, manages door lock state.

pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
pub mod proptest_cases;

/// Extract command_id from a raw JSON string without full deserialization.
/// Used when parse_command fails (e.g. missing required field) to still
/// publish an error response (03-REQ-2.E2, design Path 5).
///
/// Returns None if the JSON is malformed or command_id is missing/not a string.
pub fn extract_command_id(_json: &str) -> Option<String> {
    todo!("implemented in task group 3")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Require "serve" subcommand; otherwise print usage and exit 0.
    if args.len() < 2 || args[1] != "serve" {
        if args.iter().any(|a| a.starts_with('-') && a != "--") {
            eprintln!("usage: locking-service serve");
            std::process::exit(0);
        }
        println!("usage: locking-service serve");
        std::process::exit(0);
    }

    println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
    // Full async main implemented in task group 3.
    todo!("implemented in task group 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_command_id — valid JSON with command_id present
    #[test]
    fn test_extract_command_id_present() {
        let json = r#"{"command_id":"abc-123","doors":["driver"]}"#;
        let result = extract_command_id(json);
        assert_eq!(result, Some("abc-123".to_string()));
    }

    // extract_command_id — valid JSON without command_id
    #[test]
    fn test_extract_command_id_missing() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = extract_command_id(json);
        assert!(result.is_none(), "should return None when command_id absent");
    }

    // extract_command_id — invalid JSON
    #[test]
    fn test_extract_command_id_invalid_json() {
        let result = extract_command_id("not json {{{{");
        assert!(result.is_none(), "should return None for invalid JSON");
    }

    // extract_command_id — command_id is empty string
    #[test]
    fn test_extract_command_id_empty_string() {
        let json = r#"{"command_id":""}"#;
        let result = extract_command_id(json);
        // Empty command_id is technically extractable but invalid;
        // extract_command_id returns it and the caller decides what to do.
        // This behaviour documents that extract_command_id doesn't validate.
        // (Validation is done by validate_command separately.)
        // We allow either Some("") or None — the implementation decides.
        // Test just checks it doesn't panic.
        let _ = result;
    }

    // extract_command_id — command_id is non-string type
    #[test]
    fn test_extract_command_id_non_string() {
        let json = r#"{"command_id":42}"#;
        let result = extract_command_id(json);
        assert!(result.is_none(), "non-string command_id should return None");
    }
}
