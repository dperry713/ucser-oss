#[cfg(test)]
mod tests {
    use crate::dag::Task;
    use crate::policy::{PolicyEngine, PolicyViolation};
    use crate::audit::AuditEngine;
    use std::collections::HashMap;

    #[test]
    fn test_policy_engine_blocks_unauthorized_command() {
        let mut policy = PolicyEngine::new();
        let malicious_task = Task {
            execution_id: "test-001".to_string(),
            id: "attack_1".to_string(),
            os: "linux".to_string(),
            shell: "bash".to_string(),
            command: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
            env_vars: HashMap::new(),
            dependencies: vec![],
            retries: 0,
            max_retries: 3,
            timeout_seconds: 30,
        };

        let result = policy.validate_task(&malicious_task);
        assert!(matches!(result, Err(PolicyViolation::DisallowedCommand(cmd)) if cmd == "rm"));
    }

    #[test]
    fn test_policy_engine_blocks_env_injection() {
        let mut policy = PolicyEngine::new();
        let mut env_vars = HashMap::new();
        env_vars.insert("LD_PRELOAD".to_string(), "/tmp/malicious.so".to_string());

        let malicious_task = Task {
            execution_id: "test-002".to_string(),
            id: "attack_2".to_string(),
            os: "linux".to_string(),
            shell: "bash".to_string(),
            command: "ls".to_string(),
            args: vec![],
            env_vars,
            dependencies: vec![],
            retries: 0,
            max_retries: 3,
            timeout_seconds: 30,
        };

        let result = policy.validate_task(&malicious_task);
        assert!(matches!(result, Err(PolicyViolation::RestrictedEnvVar(var)) if var == "LD_PRELOAD"));
    }

    #[test]
    fn test_audit_log_traceability() {
        let mut audit = AuditEngine::new("test_audit.log").unwrap();
        audit.log_event("TEST_EVENT", "Simulated trace").unwrap();
        
        let contents = std::fs::read_to_string("test_audit.log").unwrap();
        assert!(contents.contains("TEST_EVENT"));
        assert!(contents.contains("Simulated trace"));
        
        // Clean up
        std::fs::remove_file("test_audit.log").unwrap();
    }
}
