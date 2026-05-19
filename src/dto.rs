use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, BTreeMap};
use dashmap::DashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Flags(pub DashMap<String, FlagDefinition>);

#[derive(Debug, Deserialize, Clone)]
pub struct NamespaceFlagsMap(pub DashMap<String, Flags>);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FlagDefinition {
    #[serde(rename = "type")]
    pub flag_type: String,
    pub variations: HashMap<String, JsonValue>,
    pub default: Option<String>,
    pub rules: Vec<Rule>,
    pub experiment: Option<ExperimentWindow>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rule {
    pub query: String,
    pub percentage: BTreeMap<String, u8>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExperimentWindow {
    pub start: String,
    pub end: String,
}

// Public API types - used by HTTP handlers
#[derive(Debug, Deserialize, Clone)]
pub struct EvalRequest {
    pub namespace: String,
    pub flags: Vec<String>,
    pub data: HashMap<String, JsonValue>,
    #[serde(default)]
    pub include_reason: bool,
    pub rollout_target_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AllEvalRequest {
    pub namespace: String,
    pub data: HashMap<String, JsonValue>,
    #[serde(default)]
    pub include_reason: bool,
    pub rollout_target_key: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct FlagResponse {
    pub value: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct EvalResponse(pub HashMap<String, FlagResponse>);

// BTreeMap-based response for deterministic ordering (signature support)
#[derive(Serialize, Debug)]
pub struct OrderedEvalResponse(pub BTreeMap<String, FlagResponse>);

// Flat BTreeMap-based response with just flag name -> value (no reason)
#[derive(Serialize, Debug)]
pub struct OrderedEvalResponseFlat(pub BTreeMap<String, JsonValue>);

#[derive(Debug)]
pub enum FlagEvalReason {
    ExperimentWindow,
    RuleMatch,
    Default,
}

impl From<FlagEvalReason> for String {
    fn from(reason: FlagEvalReason) -> String {
        match reason {
            FlagEvalReason::ExperimentWindow => "EXPERIMENT_WINDOW".to_string(),
            FlagEvalReason::RuleMatch => "RULE_MATCH".to_string(),
            FlagEvalReason::Default => "DEFAULT".to_string(),
        }
    }
}