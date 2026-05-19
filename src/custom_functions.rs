use rhai::EvalAltResult;
use versions::Versioning;

/// Container for all custom functions that can be registered with the Rhai engine
pub struct CustomFunctions;

impl CustomFunctions {
    /// Register all custom functions with the Rhai engine
    pub fn register_all(engine: &mut rhai::Engine) -> Result<(), Box<EvalAltResult>> {
        Self::register_semver_functions(engine)?;
        Ok(())
    }

    /// Register semantic version comparison functions
    fn register_semver_functions(engine: &mut rhai::Engine) -> Result<(), Box<EvalAltResult>> {
        // Register with explicit return type to handle Result properly
        engine.register_fn("semver", |v1: &str, op: &str, v2: &str| -> Result<bool, Box<EvalAltResult>> {
            semver_compare_checked(v1, op, v2)
                .map_err(|e| e.into())
        });
        Ok(())
    }
}

// =============================================================================
// Semantic Version Functions
// =============================================================================

/// Compare two semantic versions using the specified operator
/// Returns Result for proper error handling, and a bool wrapper for Rhai
/// 
/// # Arguments
/// * `version1` - First version string (e.g., "2.1.0")
/// * `operator` - Comparison operator: ">", ">=", "<", "<=", "==", "!="
/// * `version2` - Second version string (e.g., "2.0.0")
/// 
/// # Examples
/// * `semver("2.1.0", ">", "2.0.0")` returns `Ok(true)`
/// * `semver("1.9.9", ">=", "2.0.0")` returns `Ok(false)`
/// * `semver("2.0.0", "==", "2.0.0")` returns `Ok(true)`
/// 
/// # Errors
/// Returns `Err` for invalid versions or unsupported operators
pub fn semver_compare_checked(version1: &str, operator: &str, version2: &str) -> Result<bool, String> {
    let v1 = Versioning::new(version1)
        .ok_or_else(|| format!("Invalid version format: '{}'", version1))?;
    
    let v2 = Versioning::new(version2)
        .ok_or_else(|| format!("Invalid version format: '{}'", version2))?;
    
    let result = match operator {
        ">" => v1 > v2,
        ">=" => v1 >= v2,
        "<" => v1 < v2,
        "<=" => v1 <= v2,
        "==" => v1 == v2,
        "!=" => v1 != v2,
        _ => {
            return Err(format!(
                "Unsupported semver operator: '{}'. Supported: >, >=, <, <=, ==, !=",
                operator
            ));
        }
    };
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparisons() {
        // Greater than
        assert!(semver_compare_checked("2.0.0", ">", "1.9.9").unwrap());
        assert!(!semver_compare_checked("1.9.9", ">", "2.0.0").unwrap());
        
        // Greater than or equal
        assert!(semver_compare_checked("2.0.0", ">=", "2.0.0").unwrap());
        assert!(semver_compare_checked("2.0.1", ">=", "2.0.0").unwrap());
        assert!(!semver_compare_checked("1.9.9", ">=", "2.0.0").unwrap());
        
        // Less than
        assert!(semver_compare_checked("1.9.9", "<", "2.0.0").unwrap());
        assert!(!semver_compare_checked("2.0.0", "<", "1.9.9").unwrap());
        
        // Less than or equal
        assert!(semver_compare_checked("1.9.9", "<=", "2.0.0").unwrap());
        assert!(semver_compare_checked("2.0.0", "<=", "2.0.0").unwrap());
        assert!(!semver_compare_checked("2.0.1", "<=", "2.0.0").unwrap());
        
        // Equal
        assert!(semver_compare_checked("2.0.0", "==", "2.0.0").unwrap());
        assert!(!semver_compare_checked("2.0.0", "==", "2.0.1").unwrap());
        
        // Not equal
        assert!(semver_compare_checked("2.0.0", "!=", "2.0.1").unwrap());
        assert!(!semver_compare_checked("2.0.0", "!=", "2.0.0").unwrap());
        
        // 4-part versions
        assert!(semver_compare_checked("2.1.1.1", ">", "2.1.1.0").unwrap());
        assert!(semver_compare_checked("2.2.0", ">", "2.1.1.1").unwrap());
        assert!(semver_compare_checked("1.2.3.4", "==", "1.2.3.4").unwrap());
        
        // Mixed format comparisons
        assert!(semver_compare_checked("2.0.0", ">", "1.9.9.9").unwrap());
        assert!(semver_compare_checked("1.9", "<", "2.0.0.0").unwrap());
        
        // Invalid operator
        assert!(semver_compare_checked("2.0.0", "~", "1.9.9").is_err());
    }

    #[test]
    fn test_invalid_operators() {
        // Test invalid operators return errors
        assert!(semver_compare_checked("2.0.0", "~>", "1.0.0").is_err());
        assert!(semver_compare_checked("2.0.0", "^", "1.0.0").is_err());
        assert!(semver_compare_checked("2.0.0", "*", "1.0.0").is_err());
    }
}
