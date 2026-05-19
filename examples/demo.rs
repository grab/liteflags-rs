use liteflags_rs::flag_loaders::yaml::load_flags;
use liteflags_rs::{FlagEvaluator, FlagStore, create_enhanced_engine};
use liteflags_rs::dto::{EvalRequest, AllEvalRequest};
use serde_json::json;
use std::collections::HashMap;

fn main() {
    let flags = load_flags("examples/flags.yaml").expect("Failed to load flags");
    let store = FlagStore::new(flags);
    let engine = create_enhanced_engine().expect("Failed to create engine");

    println!("=== liteflags-rs demo ===\n");

    // 1. Evaluate new_checkout_flow for a US user (50/50 rollout)
    let request = EvalRequest {
        namespace: "my_app".to_string(),
        flags: vec!["new_checkout_flow".to_string()],
        data: HashMap::from([("region".into(), json!("us"))]),
        include_reason: true,
        rollout_target_key: Some("user-42".to_string()),
    };
    let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
    let checkout = &result.0["new_checkout_flow"];
    println!("new_checkout_flow (region=us, user-42): {} (reason: {})",
        checkout.value, checkout.reason.as_deref().unwrap_or("n/a"));

    // 2. Evaluate results_per_page for desktop vs mobile
    for platform in ["desktop", "mobile"] {
        let request = EvalRequest {
            namespace: "my_app".to_string(),
            flags: vec!["results_per_page".to_string()],
            data: HashMap::from([("platform".into(), json!(platform))]),
            include_reason: true,
            rollout_target_key: Some(format!("session-{}", platform)),
        };
        let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
        let rpp = &result.0["results_per_page"];
        println!("results_per_page (platform={}): {} (reason: {})",
            platform, rpp.value, rpp.reason.as_deref().unwrap_or("n/a"));
    }

    // 3. Evaluate banner_text for a returning user
    let request = EvalRequest {
        namespace: "my_app".to_string(),
        flags: vec!["banner_text".to_string()],
        data: HashMap::from([("returning_user".into(), json!(true))]),
        include_reason: true,
        rollout_target_key: Some("user-99".to_string()),
    };
    let result = FlagEvaluator::evaluate_flags(&store, request, &engine).unwrap();
    let banner = &result.0["banner_text"];
    println!("banner_text (returning_user=true): {} (reason: {})",
        banner.value, banner.reason.as_deref().unwrap_or("n/a"));

    // 4. All flags at once
    println!("\n--- All flags (flat) for a returning desktop US user ---");
    let request = AllEvalRequest {
        namespace: "my_app".to_string(),
        data: HashMap::from([
            ("region".into(), json!("us")),
            ("platform".into(), json!("desktop")),
            ("returning_user".into(), json!(true)),
        ]),
        include_reason: false,
        rollout_target_key: Some("user-7".to_string()),
    };
    let result = FlagEvaluator::evaluate_all_flags_ordered_flat(&store, request, &engine).unwrap();
    for (flag, value) in &result.0 {
        println!("  {}: {}", flag, value);
    }
}
