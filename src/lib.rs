pub mod dto;
pub mod flag_loaders;
pub mod flag_store;
pub mod flag_evaluator;
pub mod custom_functions;
pub mod engine_builder;

// Re-export the main types for convenience
pub use flag_store::FlagStore;
pub use flag_evaluator::FlagEvaluator;
pub use engine_builder::{FlagEvalEngine, FlagEvalEngineBuilder};
pub use dto::OrderedEvalResponse;
pub use custom_functions::semver_compare_checked;

// Convenience function for creating an engine with all custom functions
pub fn create_enhanced_engine() -> Result<FlagEvalEngine, Box<rhai::EvalAltResult>> {
    FlagEvalEngineBuilder::new()
        .with_custom_functions()
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::{HashMap, BTreeMap};
    use chrono::{Utc, Duration};
    use dashmap::DashMap;

    fn create_test_flags() -> dto::NamespaceFlagsMap {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        namespace_flags.insert("test_flag".to_string(), dto::FlagDefinition {
            flag_type: "boolean".to_string(),
            variations: HashMap::from([
                ("enabled".to_string(), json!(true)),
                ("disabled".to_string(), json!(false)),
            ]),
            default: Some("disabled".to_string()),
            rules: vec![
                dto::Rule {
                    query: "premium == true".to_string(),
                    percentage: BTreeMap::from([
                        ("enabled".to_string(), 100),
                        ("disabled".to_string(), 0),
                    ]),
                }
            ],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        dto::NamespaceFlagsMap(flags)
    }

    fn create_percentage_rollout_flags() -> dto::NamespaceFlagsMap {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        namespace_flags.insert("rollout_flag".to_string(), dto::FlagDefinition {
            flag_type: "string".to_string(),
            variations: HashMap::from([
                ("variant_a".to_string(), json!("A")),
                ("variant_b".to_string(), json!("B")),
                ("variant_c".to_string(), json!("C")),
            ]), 
            default: Some("variant_a".to_string()),
            rules: vec![
                dto::Rule {
                    query: "true".to_string(),
                    percentage: BTreeMap::from([
                        ("variant_a".to_string(), 50),
                        ("variant_b".to_string(), 30),
                        ("variant_c".to_string(), 20),
                    ]),
                }
            ],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        dto::NamespaceFlagsMap(flags)
    }

    fn create_experiment_flags(active: bool, future: bool) -> dto::NamespaceFlagsMap {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();

        let now = Utc::now();
        let (start_time, end_time) = if future {
            let start = now + Duration::days(1);
            let end = now + Duration::days(7);
            (start, end)
        } else if active {
            let start = now - Duration::days(1);
            let end = now + Duration::days(7);
            (start, end)
        } else {
            let start = now - Duration::days(7);
            let end = now - Duration::days(1);
            (start, end)
        };
        
        namespace_flags.insert("experiment_flag".to_string(), dto::FlagDefinition {
            flag_type: "boolean".to_string(),
            variations: HashMap::from([
                ("enabled".to_string(), json!(true)),
                ("disabled".to_string(), json!(false)),
            ]),
            default: Some("disabled".to_string()),
            rules: vec![
                dto::Rule {
                    query: "true".to_string(),
                    percentage: BTreeMap::from([
                        ("enabled".to_string(), 100),
                        ("disabled".to_string(), 0),
                    ]),
                }
            ],
            experiment: Some(dto::ExperimentWindow {
                start: start_time.to_rfc3339(),
                end: end_time.to_rfc3339(),
            }),
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        dto::NamespaceFlagsMap(flags)
    }

    #[test]
    fn test_flag_store_creation() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        
        let flag = store.get_flag("test_namespace", "test_flag");
        assert!(flag.is_some());
        assert_eq!(flag.unwrap().flag_type, "boolean");
    }

    #[test]
    fn test_flag_store_get_nonexistent() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        
        let flag = store.get_flag("test_namespace", "nonexistent");
        assert!(flag.is_none());
        
        let flag = store.get_flag("nonexistent", "test_flag");
        assert!(flag.is_none());
    }

    #[test]
    fn test_flag_evaluation_with_matching_rule() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"].value, json!(true));
        assert_eq!(result.0["test_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_flag_evaluation_default_fallback() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(false)),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"].value, json!(false));
        assert_eq!(result.0["test_flag"].reason, Some("DEFAULT".to_string()));
    }

    #[test]
    fn test_flag_evaluation_no_default_no_match() {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        // Create a flag with no default
        namespace_flags.insert("no_default_flag".to_string(), dto::FlagDefinition {
            flag_type: "string".to_string(),
            variations: HashMap::from([
                ("option_a".to_string(), json!("A")),
                ("option_b".to_string(), json!("B")),
            ]),
            default: None, // No default specified
            rules: vec![
                dto::Rule {
                    query: "premium == true".to_string(), // This will not match
                    percentage: BTreeMap::from([
                        ("option_a".to_string(), 100),
                    ]),
                }
            ],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        let flag_map = dto::NamespaceFlagsMap(flags);
        let store = FlagStore::new(flag_map);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["no_default_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(false)), // Rule won't match
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        // Flag should be excluded from response since no default and no rule matched
        assert!(!result.0.contains_key("no_default_flag"));
    }

    #[test]
    fn test_flag_evaluation_no_default_with_match() {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        // Create a flag with no default
        namespace_flags.insert("no_default_flag".to_string(), dto::FlagDefinition {
            flag_type: "string".to_string(),
            variations: HashMap::from([
                ("option_a".to_string(), json!("A")),
                ("option_b".to_string(), json!("B")),
            ]),
            default: None, // No default specified
            rules: vec![
                dto::Rule {
                    query: "premium == true".to_string(), // This will match
                    percentage: BTreeMap::from([
                        ("option_a".to_string(), 100),
                    ]),
                }
            ],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        let flag_map = dto::NamespaceFlagsMap(flags);
        let store = FlagStore::new(flag_map);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["no_default_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(true)), // Rule will match
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        // Flag should be included since rule matched
        assert!(result.0.contains_key("no_default_flag"));
        assert_eq!(result.0["no_default_flag"].value, json!("A"));
        assert_eq!(result.0["no_default_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_flag_evaluation_nonexistent_namespace() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "nonexistent".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::new(),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Namespace not found"));
    }

    #[test]
    fn test_evaluate_all_flags() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "test_namespace".to_string(),
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_all_flags(&store, request, &engine).unwrap();
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"].value, json!(true));
        assert_eq!(result.0["test_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_percentage_rollout_deterministic() {
        let flags = create_percentage_rollout_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["rollout_flag".to_string()],
            data: HashMap::from([
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result1 = FlagEvaluator::evaluate_flags(&store, request.clone(), &engine).unwrap();
        let result2 = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        
        assert_eq!(result1.0["rollout_flag"].value, result2.0["rollout_flag"].value);
    }

    #[test]
    fn test_percentage_rollout_distribution() {
        let flags = create_percentage_rollout_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let mut results = HashMap::new();
        
        for i in 0..1000 {
            let request = dto::EvalRequest {
                namespace: "test_namespace".to_string(),
                flags: vec!["rollout_flag".to_string()],
                data: HashMap::new(),
                include_reason: false,
                rollout_target_key: Some(format!("user-{}", i)),
            };

            let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
            let variant = result.0["rollout_flag"].value.as_str().unwrap();
            *results.entry(variant.to_string()).or_insert(0) += 1;
        }

        assert!(results.get("A").unwrap_or(&0) > &400);
        assert!(results.get("B").unwrap_or(&0) > &200);
        assert!(results.get("C").unwrap_or(&0) > &100);
    }

    #[test]
    fn test_murmur_hash_distribution_quality() {
        // Test that MurmurHash provides good uniform distribution
        let flags = create_percentage_rollout_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let mut results = HashMap::new();
        let test_size = 10000; // Larger sample for distribution testing
        
        for i in 0..test_size {
            let request = dto::EvalRequest {
                namespace: "test_namespace".to_string(),
                flags: vec!["rollout_flag".to_string()],
                data: HashMap::new(),
                include_reason: false,
                rollout_target_key: Some(format!("user-{}", i)),
            };

            let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
            let variant = result.0["rollout_flag"].value.as_str().unwrap();
            *results.entry(variant.to_string()).or_insert(0) += 1;
        }

        // With MurmurHash, we should get very close to expected percentages
        // variant_a: 50%, variant_b: 30%, variant_c: 20%
        let a_count = results.get("A").unwrap_or(&0);
        let b_count = results.get("B").unwrap_or(&0);
        let c_count = results.get("C").unwrap_or(&0);
        
        // Allow ±2% variance from expected (should be much tighter with MurmurHash)
        assert!(*a_count >= (test_size * 48 / 100), "variant_a: expected ~50%, got {}%", *a_count * 100 / test_size);
        assert!(*a_count <= (test_size * 52 / 100), "variant_a: expected ~50%, got {}%", *a_count * 100 / test_size);
        
        assert!(*b_count >= (test_size * 28 / 100), "variant_b: expected ~30%, got {}%", *b_count * 100 / test_size);
        assert!(*b_count <= (test_size * 32 / 100), "variant_b: expected ~30%, got {}%", *b_count * 100 / test_size);
        
        assert!(*c_count >= (test_size * 18 / 100), "variant_c: expected ~20%, got {}%", *c_count * 100 / test_size);
        assert!(*c_count <= (test_size * 22 / 100), "variant_c: expected ~20%, got {}%", *c_count * 100 / test_size);
    }

    #[test]
    fn test_percentage_rollout_deterministic_ordering() {
        // Test that percentage rollout is deterministic regardless of map key insertion ordering
        // by creating flags with variants in different insertion orders
        let flags1 = {
            let flags = DashMap::new();
            let namespace_flags = DashMap::new();
            
            namespace_flags.insert("rollout_flag".to_string(), dto::FlagDefinition {
                flag_type: "string".to_string(),
                variations: HashMap::from([
                    ("variant_a".to_string(), json!("A")),
                    ("variant_b".to_string(), json!("B")),
                    ("variant_c".to_string(), json!("C")),
                ]), 
                default: Some("variant_a".to_string()),
                rules: vec![
                    dto::Rule {
                        query: "true".to_string(),
                        percentage: BTreeMap::from([
                            ("variant_a".to_string(), 50),
                            ("variant_b".to_string(), 30),
                            ("variant_c".to_string(), 20),
                        ]),
                    }
                ],
                experiment: None,
            });
            flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
            dto::NamespaceFlagsMap(flags)
        };

        let flags2 = {
            let flags = DashMap::new();
            let namespace_flags = DashMap::new();
            
            namespace_flags.insert("rollout_flag".to_string(), dto::FlagDefinition {
                flag_type: "string".to_string(),
                variations: HashMap::from([
                    ("variant_c".to_string(), json!("C")),
                    ("variant_a".to_string(), json!("A")),
                    ("variant_b".to_string(), json!("B")),
                ]), 
                default: Some("variant_a".to_string()),
                rules: vec![
                    dto::Rule {
                        query: "true".to_string(),
                        percentage: BTreeMap::from([
                            ("variant_c".to_string(), 20),
                            ("variant_a".to_string(), 50),
                            ("variant_b".to_string(), 30),
                        ]),
                    }
                ],
                experiment: None,
            });
            flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
            dto::NamespaceFlagsMap(flags)
        };

        let store1 = FlagStore::new(flags1);
        let store2 = FlagStore::new(flags2);
        let engine = FlagEvalEngine::new();
        
        // Test same user gets same result from both flag configurations
        for i in 0..100 {
            let request = dto::EvalRequest {
                namespace: "test_namespace".to_string(),
                flags: vec!["rollout_flag".to_string()],
                data: HashMap::new(),
                include_reason: false,
                rollout_target_key: Some(format!("user-{}", i)),
            };

            let result1 = FlagEvaluator::evaluate_flags(&store1, request.clone(), &engine).unwrap();
            let result2 = FlagEvaluator::evaluate_flags(&store2, request, &engine).unwrap();
            
            assert_eq!(
                result1.0["rollout_flag"].value, 
                result2.0["rollout_flag"].value,
                "User {} got different variants from different flag orderings", i
            );
        }
    }

    #[test]
    fn test_active_experiment_window() {
        let flags = create_experiment_flags(true, false);
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["experiment_flag".to_string()],
            data: HashMap::from([
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert_eq!(result.0["experiment_flag"].value, json!(true));
        assert_eq!(result.0["experiment_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_expired_experiment_window() {
        let flags = create_experiment_flags(false, false);
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["experiment_flag".to_string()],
            data: HashMap::from([
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert_eq!(result.0["experiment_flag"].value, json!(false));
        assert_eq!(result.0["experiment_flag"].reason, Some("EXPERIMENT_WINDOW".to_string()));
    }

    #[test]
    fn test_future_experiment_window() {
        let flags = create_experiment_flags(false, true);
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["experiment_flag".to_string()],
            data: HashMap::from([
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert_eq!(result.0["experiment_flag"].value, json!(false));
        assert_eq!(result.0["experiment_flag"].reason, Some("EXPERIMENT_WINDOW".to_string()));
    }

    #[test]
    fn test_flag_store_update() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        
        let original_flag = store.get_flag("test_namespace", "test_flag");
        assert!(original_flag.is_some());
        
        let new_flags = create_percentage_rollout_flags();
        store.update_flags(new_flags);
        
        let old_flag = store.get_flag("test_namespace", "test_flag");
        assert!(old_flag.is_none());
        
        let new_flag = store.get_flag("test_namespace", "rollout_flag");
        assert!(new_flag.is_some());
    }

    #[test]
    fn test_semver_functions_in_flag_evaluation() {
        let flags = create_semver_test_flags();
        let store = FlagStore::new(flags);
        let engine = create_enhanced_engine().expect("Failed to create enhanced engine");
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["version_flag".to_string()],
            data: HashMap::from([
                ("app_version".to_string(), json!("2.1.0")),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert!(result.0.contains_key("version_flag"));
        assert_eq!(result.0["version_flag"].value, json!(true));
        assert_eq!(result.0["version_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_backward_compatibility_with_basic_engine() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new(); // Basic engine without custom functions
        
        let request = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"].value, json!(true));
        assert_eq!(result.0["test_flag"].reason, Some("RULE_MATCH".to_string()));
    }

    #[test]
    fn test_include_reason_flag() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        // Test with include_reason = true
        let request_with_reason = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result_with = FlagEvaluator::evaluate_flags(&store, request_with_reason, &engine).unwrap();
        assert!(result_with.0["test_flag"].reason.is_some());
        assert_eq!(result_with.0["test_flag"].reason, Some("RULE_MATCH".to_string()));
        
        // Test with include_reason = false
        let request_without_reason = dto::EvalRequest {
            namespace: "test_namespace".to_string(),
            flags: vec!["test_flag".to_string()],
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: false,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result_without = FlagEvaluator::evaluate_flags(&store, request_without_reason, &engine).unwrap();
        assert!(result_without.0["test_flag"].reason.is_none());
        assert_eq!(result_without.0["test_flag"].value, json!(true));
    }

    #[test]
    fn test_evaluate_all_flags_ordered() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "test_namespace".to_string(),
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_all_flags_ordered(&store, request, &engine).unwrap();
        
        // Verify the result is a BTreeMap (ordered)
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"].value, json!(true));
        assert_eq!(result.0["test_flag"].reason, Some("RULE_MATCH".to_string()));
        
        // Verify it can be serialized deterministically
        let json_str1 = serde_json::to_string(&result.0).unwrap();
        let json_str2 = serde_json::to_string(&result.0).unwrap();
        assert_eq!(json_str1, json_str2);
    }

    #[test]
    fn test_evaluate_all_flags_ordered_flat() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "test_namespace".to_string(),
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
            ]),
            include_reason: true, // Should be ignored in flat response
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_all_flags_ordered_flat(&store, request, &engine).unwrap();
        
        // Verify the result is a flat BTreeMap with just values
        assert!(result.0.contains_key("test_flag"));
        assert_eq!(result.0["test_flag"], json!(true));
        
        // Verify it can be serialized deterministically
        let json_str1 = serde_json::to_string(&result.0).unwrap();
        let json_str2 = serde_json::to_string(&result.0).unwrap();
        assert_eq!(json_str1, json_str2);
    }

    #[test]
    fn test_evaluate_all_flags_ordered_flat_multiple_flags() {
        // Create flags with multiple flags for better testing
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        namespace_flags.insert("flag_a".to_string(), dto::FlagDefinition {
            flag_type: "boolean".to_string(),
            variations: HashMap::from([
                ("enabled".to_string(), json!(true)),
                ("disabled".to_string(), json!(false)),
            ]),
            default: Some("enabled".to_string()),
            rules: vec![],
            experiment: None,
        });
        
        namespace_flags.insert("flag_b".to_string(), dto::FlagDefinition {
            flag_type: "string".to_string(),
            variations: HashMap::from([
                ("value1".to_string(), json!("hello")),
                ("value2".to_string(), json!("world")),
            ]),
            default: Some("value1".to_string()),
            rules: vec![],
            experiment: None,
        });
        
        namespace_flags.insert("flag_c".to_string(), dto::FlagDefinition {
            flag_type: "number".to_string(),
            variations: HashMap::from([
                ("low".to_string(), json!(10)),
                ("high".to_string(), json!(100)),
            ]),
            default: Some("high".to_string()),
            rules: vec![],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        let flag_map = dto::NamespaceFlagsMap(flags);
        let store = FlagStore::new(flag_map);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "test_namespace".to_string(),
            data: HashMap::from([
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: false,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_all_flags_ordered_flat(&store, request, &engine).unwrap();
        
        // Verify all flags are present with their values
        assert_eq!(result.0.len(), 3);
        assert_eq!(result.0["flag_a"], json!(true));
        assert_eq!(result.0["flag_b"], json!("hello"));
        assert_eq!(result.0["flag_c"], json!(100));
        
        // Verify the keys are ordered alphabetically (BTreeMap property)
        let keys: Vec<&String> = result.0.keys().collect();
        assert_eq!(keys, vec!["flag_a", "flag_b", "flag_c"]);
    }

    #[test]
    fn test_evaluate_all_flags_ordered_flat_vs_ordered_consistency() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "test_namespace".to_string(),
            data: HashMap::from([
                ("premium".to_string(), json!(true)),
                ("entity_id".to_string(), json!("user-123")),
            ]),
            include_reason: true,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result_ordered = FlagEvaluator::evaluate_all_flags_ordered(&store, request.clone(), &engine).unwrap();
        let result_flat = FlagEvaluator::evaluate_all_flags_ordered_flat(&store, request, &engine).unwrap();
        
        // Verify both have the same flags
        assert_eq!(result_ordered.0.len(), result_flat.0.len());
        
        // Verify values match
        for (flag_name, flag_response) in result_ordered.0.iter() {
            assert!(result_flat.0.contains_key(flag_name));
            assert_eq!(&flag_response.value, result_flat.0.get(flag_name).unwrap());
        }
    }

    #[test]
    fn test_evaluate_all_flags_ordered_flat_nonexistent_namespace() {
        let flags = create_test_flags();
        let store = FlagStore::new(flags);
        let engine = FlagEvalEngine::new();
        
        let request = dto::AllEvalRequest {
            namespace: "nonexistent".to_string(),
            data: HashMap::new(),
            include_reason: false,
            rollout_target_key: Some("user-123".to_string()),
        };

        let result = FlagEvaluator::evaluate_all_flags_ordered_flat(&store, request, &engine);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Namespace not found"));
    }

    fn create_semver_test_flags() -> dto::NamespaceFlagsMap {
        let flags = DashMap::new();
        let namespace_flags = DashMap::new();
        
        namespace_flags.insert("version_flag".to_string(), dto::FlagDefinition {
            flag_type: "boolean".to_string(),
            variations: HashMap::from([
                ("enabled".to_string(), json!(true)),
                ("disabled".to_string(), json!(false)),
            ]),
            default: Some("disabled".to_string()),
            rules: vec![
                dto::Rule {
                    query: r#"semver(app_version, ">=", "2.0.0")"#.to_string(),
                    percentage: BTreeMap::from([
                        ("enabled".to_string(), 100),
                        ("disabled".to_string(), 0),
                    ]),
                }
            ],
            experiment: None,
        });

        flags.insert("test_namespace".to_string(), dto::Flags(namespace_flags));
        dto::NamespaceFlagsMap(flags)
    }
} 