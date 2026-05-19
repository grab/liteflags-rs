# Liteflags-rs

A lightweight Rust library for feature flag evaluation. It comes with a default flag loader for YAML type. Flag loading is decoupled and custom source (e.g., Flag config stored in JSON file, Database) can be used and then supplied to library's FlagStore.

## Features

- **Multiple flag types**: boolean, string, number
- **Rule-based evaluation**: Using Rhai expressions for complex conditions
- **Percentage rollouts**: Gradual feature releases
- **Experiment windows**: Time-bounded experiments
- **Namespaces**: Organize flags by context
- **Real-time updates**: Hot-reload flag configurations

## Quick Start

### 1. Add to Cargo.toml

```toml
[dependencies]
liteflags-rs = "0.1.1"
```

### 2. Create flag configuration (YAML)

```yaml
my_namespace:
  my_feature:
    type: bool
    variations:
      enabled: true
      disabled: false
    default: disabled
    rules:
      - query: 'premium == true && region == "US"'
        percentage:
          enabled: 100
          disabled: 0
```

### 3. Basic usage

```rust
use liteflags-rs::{FlagStore, FlagEvaluator, FlagEvalEngine};
use std::collections::HashMap;
use serde_json::json;

// Load flags from YAML
let flags = liteflags-rs::flag_loaders::yaml::load_flags("flags.yaml")?;

// Create store and evaluator
let store = FlagStore::new(flags);
let engine = FlagEvalEngine::new();

// Evaluate flags
let request = liteflags-rs::dto::EvalRequest {
    namespace: "my_namespace".to_string(),
    flags: vec!["my_feature".to_string()],
    data: HashMap::from([
        ("premium".to_string(), json!(true)),
        ("region".to_string(), json!("US")),
    ]),
    include_reason: true,
    rollout_target_key: Some("user-123".to_string()), // For percentage-based rollouts
};

let result = FlagEvaluator::evaluate_flags(&store, request, &engine)?;

// Access the result
println!("{:?}", result.0["my_feature"].value); // true
println!("{:?}", result.0["my_feature"].reason); // Some("RULE_MATCH")
```

### 4. Using Custom Functions (Advanced)

Liteflags-rs supports custom functions for semantic versioning comparisons:

```rust
use liteflags-rs::{FlagStore, FlagEvaluator, create_enhanced_engine};
use std::collections::HashMap;
use serde_json::json;

// Create enhanced engine with custom functions
let engine = create_enhanced_engine()?;

// Now you can use advanced rules like:
let request = liteflags-rs::dto::EvalRequest {
    namespace: "my_namespace".to_string(),
    flags: vec!["advanced_feature".to_string()],
    data: HashMap::from([
        ("app_version".to_string(), json!("2.1.0")),
    ]),
    include_reason: true,
    rollout_target_key: Some("user-123".to_string()),
};

let result = FlagEvaluator::evaluate_flags(&store, request, &engine)?;
```

#### Available Custom Functions

**Semantic Version Function:**
- `semver(version1, operator, version2)` - Compare versions with operator
  - Supported operators: `>`, `>=`, `<`, `<=`, `==`, `!=`
  - Example: `semver(app_version, ">=", "2.0.0")`

#### Advanced Rule Examples

```yaml
my_namespace:
  version_feature:
    type: bool
    variations:
      enabled: true
      disabled: false
    default: disabled
    rules:
      # Enable for app version >= 2.0.0
      - query: 'semver(app_version, ">=", "2.0.0")'
        percentage:
          enabled: 100
```

## Percentage Rollouts

The `rollout_target_key` field is used for deterministic percentage-based rollouts. This key (typically a user ID or entity ID) is hashed to consistently assign users to the same variant.

**Without rollout_target_key:**
- Rules with percentage distribution are **skipped**
- Falls back to the default value

**With rollout_target_key:**
- Rules with percentage distribution are evaluated
- User is consistently assigned to the same variant based on their key

## Development

### With Nix (Recommended)
```bash
cd liteflags-rs
nix develop       # Enter development environment  
cargo test        # Run tests (16 tests)
cargo build       # Build library
cargo doc --open  # Generate documentation
```

### Without Nix
```bash
cargo test        # Run tests
cargo build       # Build library
```

## YAML Configuration

Flag definitions support:
- `type`: `bool`, `string`, `number`
- `variations`: Map of variant names to values
- `default`: Default variant name (optional in v0.1.1+)
- `rules`: List of conditional rules with percentage rollouts
- `experiment`: Optional time window for experiments

### Optional Defaults (v0.1.1+)
Flags can now omit the `default` field. When no default is specified and no rules match, the flag is excluded from evaluation results.


## Maintainer
The package is maintained by [Md Riyadh](https://github.com/riyadhctg) and [Jialong Loh](https://github.com/jlloh)