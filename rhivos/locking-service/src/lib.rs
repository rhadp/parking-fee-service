/// Placeholder module for the LOCKING_SERVICE component.
///
/// This service will interact with the Eclipse Kuksa Databroker
/// to manage vehicle door lock state via VSS signals.

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true, "locking-service skeleton compiles and tests run");
    }

    /// TS-02-7: LOCKING_SERVICE parses command JSON (02-REQ-2.2)
    ///
    /// Verify LOCKING_SERVICE correctly parses command JSON payloads
    /// extracting command_id, action, and doors fields.
    #[test]
    fn test_locking_parses_command_json() {
        let json_input = r#"{
            "command_id": "abc",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000000
        }"#;

        // Once the command module is implemented, this will use:
        //   use crate::command::{LockCommand, LockAction};
        //   let cmd: LockCommand = serde_json::from_str(json_input).unwrap();
        //   assert_eq!(cmd.command_id, "abc");
        //   assert_eq!(cmd.action, LockAction::Lock);
        //   assert_eq!(cmd.doors, vec!["driver"]);

        // For now, verify the JSON is at least parseable as a generic value
        let value: serde_json::Value = serde_json::from_str(json_input)
            .expect("test JSON should be valid");

        // These assertions will need to change to use the actual LockCommand type
        assert_eq!(value["command_id"], "abc");
        assert_eq!(value["action"], "lock");

        // FAIL: the command module with LockCommand/LockAction types doesn't exist yet
        panic!(
            "not implemented: LockCommand and LockAction types not yet defined in command module"
        );
    }

    /// TS-02-7 (variant): Verify unlock action parses correctly
    #[test]
    fn test_locking_parses_unlock_command() {
        let json_input = r#"{
            "command_id": "def",
            "action": "unlock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000001
        }"#;

        let value: serde_json::Value = serde_json::from_str(json_input)
            .expect("test JSON should be valid");

        assert_eq!(value["command_id"], "def");
        assert_eq!(value["action"], "unlock");

        // FAIL: the command module with LockCommand/LockAction types doesn't exist yet
        panic!(
            "not implemented: LockCommand and LockAction types not yet defined in command module"
        );
    }
}
