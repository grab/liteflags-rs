use crate::dto::NamespaceFlagsMap;
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use serde_yml::Value as YamlValue;
use std::fs;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};


pub fn load_flags<P: AsRef<str>>(path: P) -> Result<NamespaceFlagsMap> {
    let path_str = path.as_ref();
    let yaml_str = fs::read_to_string(path_str)
        .with_context(|| format!("Failed to read YAML file: {}", path_str))?;
    
    let yaml_value: YamlValue = serde_yml::from_str(&yaml_str)
        .with_context(|| format!("Failed to parse YAML file: {}", path_str))?;
    
    let json_value = yaml_to_json(yaml_value);
    
    serde_json::from_value(json_value)
        .with_context(|| format!("Failed to convert YAML to internal format: {}", path_str))
}

/// Create a polling task that can be spawned by the caller
/// Returns an async task that polls the YAML file and calls the callback
pub async fn poll_for_flag_config_changes<P, F>(
    file_path: P,
    polling_interval: Duration,
    update_callback: F,
) where
    P: AsRef<str>,
    F: Fn(NamespaceFlagsMap) + Send + Sync + 'static,
{
    let file_path = file_path.as_ref().to_string();
    let mut interval_timer = tokio::time::interval(polling_interval);

    let mut last_modified_time: Option<SystemTime> = None;
    
    info!("Started YAML polling for: {} (interval: {:?})", file_path, polling_interval);
    
    loop {
        interval_timer.tick().await;

        let modified_time = fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .ok();

        let has_changed = match (modified_time, last_modified_time) {
            (Some(new), Some(old)) => new > old,
            (Some(_), None) => true,
            _ => false,
        };

        if has_changed {
            match load_flags(&file_path) {
                Ok(new_flags) => {
                    let namespace_count = new_flags.0.len();
                    info!("Updated {} namespaces from YAML file: {}", namespace_count, file_path);
                    update_callback(new_flags);
                    last_modified_time = modified_time;
                }
                Err(e) => {
                    warn!("Failed to reload YAML flags from {}: {}", file_path, e);
                }
            }
        }
    }
}


fn yaml_to_json(yaml: YamlValue) -> JsonValue {
    match yaml {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                JsonValue::Number(serde_json::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                JsonValue::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)))
            } else {
                JsonValue::Null
            }
        },
        YamlValue::String(s) => JsonValue::String(s),
        YamlValue::Sequence(seq) => {
            JsonValue::Array(seq.into_iter().map(yaml_to_json).collect())
        },
        YamlValue::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let YamlValue::String(key) = k {
                    obj.insert(key, yaml_to_json(v));
                }
            }
            JsonValue::Object(obj)
        },
        YamlValue::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_yaml_load_flags() {
        let yaml_content = r#"
test_namespace:
  simple_flag:
    type: "string"
    default: "default_value"
    variations:
      default_value: "default"
      other_value: "other"
    rules: []
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();
        let file_path = temp_file.path().to_str().unwrap();

        let result = load_flags(file_path).unwrap();
        assert!(result.0.contains_key("test_namespace"));
        
        let namespace = result.0.get("test_namespace").unwrap();
        assert!(namespace.0.contains_key("simple_flag"));
        
        let flag = namespace.0.get("simple_flag").unwrap();
        assert_eq!(flag.flag_type, "string");
        assert_eq!(flag.default, Some("default_value".to_string()));
    }

    #[test]
    fn test_yaml_load_invalid_file() {
        let result = load_flags("nonexistent_file.yaml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read YAML file"));
    }
} 