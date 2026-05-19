use crate::dto::*;
use crate::flag_store::FlagStore;
use crate::FlagEvalEngine;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rhai::{Dynamic, Scope};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, BTreeMap};
use murmur3::murmur3_32;
use tracing::{debug, warn};

pub struct FlagEvaluator;

impl FlagEvaluator {
    pub fn evaluate_flags(
        store: &FlagStore,
        request: EvalRequest,
        engine: &FlagEvalEngine,
    ) -> Result<EvalResponse> {
        let Some(namespace_flags) = store.flags.0.get(&request.namespace) else {
            warn!("Namespace not found: {}", request.namespace);
            return Err(anyhow!("Namespace not found: {}", request.namespace));
        };

        let context = Self::convert_context(&request.data);
        let mut results = HashMap::new();
        let include_reason = request.include_reason;
        let rollout_target_key = request.rollout_target_key.as_deref();

        for flag_name in request.flags.iter() {
            if let Some(flag_entry) = namespace_flags.0.get(flag_name) {
                let flag = flag_entry.value();
                if let Some((value, reason)) = Self::evaluate_flag(flag, &context, rollout_target_key, engine) {
                    results.insert(
                        flag_name.to_string(),
                        FlagResponse { 
                            value, 
                            reason: if include_reason { Some(reason) } else { None },
                        },
                    );
                } else {
                    debug!("Flag '{}' has no default and no rules matched - excluding from response", flag_name);
                }
            } else {
                warn!("Flag not found: {}", flag_name);
            }
        }

        Ok(EvalResponse(results))
    }

    pub fn evaluate_all_flags(
        store: &FlagStore,
        request: AllEvalRequest,
        engine: &FlagEvalEngine,
    ) -> Result<EvalResponse> {
        let Some(namespace_flags) = store.flags.0.get(&request.namespace) else {
            warn!("Namespace not found: {}", request.namespace);
            return Err(anyhow!("Namespace not found: {}", request.namespace));
        };

        let context = Self::convert_context(&request.data);
        let mut results = HashMap::new();
        let include_reason = request.include_reason;
        let rollout_target_key = request.rollout_target_key.as_deref();

        for flag_entry in namespace_flags.0.iter() {
            let flag_name = flag_entry.key();
            let flag = flag_entry.value(); 
            if let Some((value, reason)) = Self::evaluate_flag(flag, &context, rollout_target_key, engine) {
                results.insert(
                    flag_name.clone(),
                    FlagResponse { 
                        value, 
                        reason: if include_reason { Some(reason) } else { None },
                    },
                );
            } else {
                debug!("Flag '{}' has no default and no rules matched - excluding from response", flag_name);
            }
        }

        Ok(EvalResponse(results))
    }

    pub fn evaluate_all_flags_ordered(
        store: &FlagStore,
        request: AllEvalRequest,
        engine: &FlagEvalEngine,
    ) -> Result<OrderedEvalResponse> {
        let Some(namespace_flags) = store.flags.0.get(&request.namespace) else {
            warn!("Namespace not found: {}", request.namespace);
            return Err(anyhow!("Namespace not found: {}", request.namespace));
        };

        let context = Self::convert_context(&request.data);
        let mut results = BTreeMap::new();
        let include_reason = request.include_reason;
        let rollout_target_key = request.rollout_target_key.as_deref();

        for flag_entry in namespace_flags.0.iter() {
            let flag_name = flag_entry.key();
            let flag = flag_entry.value(); 
            if let Some((value, reason)) = Self::evaluate_flag(flag, &context, rollout_target_key, engine) {
                results.insert(
                    flag_name.clone(),
                    FlagResponse { 
                        value, 
                        reason: if include_reason { Some(reason) } else { None },
                    },
                );
            } else {
                debug!("Flag '{}' has no default and no rules matched - excluding from response", flag_name);
            }
        }

        Ok(OrderedEvalResponse(results))
    }

    pub fn evaluate_all_flags_ordered_flat(
        store: &FlagStore,
        request: AllEvalRequest,
        engine: &FlagEvalEngine,
    ) -> Result<OrderedEvalResponseFlat> {
        let Some(namespace_flags) = store.flags.0.get(&request.namespace) else {
            warn!("Namespace not found: {}", request.namespace);
            return Err(anyhow!("Namespace not found: {}", request.namespace));
        };

        let context = Self::convert_context(&request.data);
        let mut results = BTreeMap::new();
        let rollout_target_key = request.rollout_target_key.as_deref();

        for flag_entry in namespace_flags.0.iter() {
            let flag_name = flag_entry.key();
            let flag = flag_entry.value(); 
            if let Some((value, _reason)) = Self::evaluate_flag(flag, &context, rollout_target_key, engine) {
                results.insert(flag_name.clone(), value);
            } else {
                debug!("Flag '{}' has no default and no rules matched - excluding from response", flag_name);
            }
        }

        Ok(OrderedEvalResponseFlat(results))
    }

    /// Convert JSON context to Rhai Dynamic context for eval by Rhai engine
    fn convert_context(data: &HashMap<String, JsonValue>) -> HashMap<String, Dynamic> {
        debug!("Converting context with {} keys", data.len());
        let mut out = HashMap::new();
        for (k, v) in data {
            let d = match v {
                JsonValue::Bool(b) => {
                    debug!("Converting boolean value '{}' for key '{}'", b, k);
                    (*b).into()
                }
                JsonValue::Number(n) => {
                    debug!("Converting number value '{}' for key '{}'", n, k);
                    if let Some(i) = n.as_i64() {
                        i.into()
                    } else if let Some(f) = n.as_f64() {
                        f.into()
                    } else {
                        Dynamic::UNIT
                    }
                }
                JsonValue::String(s) => {
                    debug!("Converting string value '{}' for key '{}'", s, k);
                    s.clone().into()
                }
                _ => {
                    debug!("Unhandled value type for key '{}': {:?}", k, v);
                    Dynamic::UNIT
                }
            };
            out.insert(k.clone(), d);
        }
        debug!("Converted context: {:#?}", out);
        out
    }

    /// Core feature flag evaluation logic
    fn evaluate_flag(
        flag: &FlagDefinition,
        context: &HashMap<String, Dynamic>,
        rollout_target_key: Option<&str>,
        engine: &FlagEvalEngine,
    ) -> Option<(JsonValue, String)> {
        debug!("\nEvaluating flag...");
        debug!("Full context: {:#?}", context);
        debug!(
            "Available context keys: {:?}",
            context.keys().collect::<Vec<_>>()
        );

        // Priority#1 - check if the flag has an experiment defined and only continue further eval if it is within the window
        // if the data is outside the window, then use default without any further eval
        if let Some(exp) = &flag.experiment {
            let now = Utc::now();
            let start = DateTime::parse_from_rfc3339(&exp.start)
                .ok()?
                .with_timezone(&Utc);
            let end = DateTime::parse_from_rfc3339(&exp.end)
                .ok()?
                .with_timezone(&Utc);
            if now < start || now > end {
                debug!("Outside experiment window. Checking for default.");
                if let Some(default_key) = &flag.default {
                    return flag.variations.get(default_key).map(|v| {
                        (v.clone(), FlagEvalReason::ExperimentWindow.into())
                    });
                } else {
                    debug!("No default specified and outside experiment window. Returning None.");
                    return None;
                }
            }
        }

        // Priority#2 - if no experiment is defined or the data is within the window, then check if the flag matches a rule
        // Rules are evaluated in order, so the first rule that matches is used
        for rule in &flag.rules {
            let mut scope = Scope::new();
            for (k, v) in context {
                scope.push(k.clone(), v.clone());
            }

            debug!("Rule query: {}", rule.query);
            debug!("Scope contents: {:#?}", scope);

            match engine.eval_with_scope::<bool>(&mut scope, &rule.query) {
                Ok(true) => { /* matched */}
                Ok(false) => continue,
                Err(e) => {
                    debug!("Error evaluating rule '{}': {}", rule.query, e);
                    continue;
                }
                
            }

            let Some(key_str) = rollout_target_key else {
                debug!("No rollout_target_key provided, skipping percentage rule");
                continue;
            };
            if let Some(val) = Self::select_by_percentage(
                &flag.variations,
                &rule.percentage,
                key_str,
                &flag.default,
            ) {
                debug!("Selected variation: {:?}", val);
                return Some((val, FlagEvalReason::RuleMatch.into()));
            } else {
                debug!("Percentage selection returned None, continuing to next rule");
                continue;
            }
        }

        // Priority#3 - return the default variation if none of the above conditions are met
        debug!("No rule matched or no targeting key available. Checking for default...");
        if let Some(default_key) = &flag.default {
            flag.variations.get(default_key).map(|v| {
                (v.clone(), FlagEvalReason::Default.into())
            })
        } else {
            debug!("No default specified and no rules matched. Returning None.");
            None
        }
    }

    /// Select a variation based on percentage distribution and entity key
    fn select_by_percentage(
        variations: &HashMap<String, JsonValue>,
        dist: &BTreeMap<String, u8>,
        key: &str,
        default: &Option<String>,
    ) -> Option<JsonValue> {
        // Use MurmurHash for excellent uniform distribution in bucketing
        let hash = murmur3_32(&mut key.as_bytes(), 0).unwrap_or(0);
        let bucket = (hash % 100) as u8;

        let mut cumulative = 0u8;
        // BTreeMap naturally iterates in sorted order
        for (variant, percentage) in dist {
            cumulative += percentage;
            if bucket < cumulative {
                if let Some(value) = variations.get(variant) {
                    return Some(value.clone());
                } else if let Some(default_key) = default {
                    return variations.get(default_key).cloned();
                } else {
                    return None;
                }
            }
        }

        // If no percentage bucket matched, return default if available
        if let Some(default_key) = default {
            variations.get(default_key).cloned()
        } else {
            None
        }
    }
} 