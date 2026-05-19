use crate::custom_functions::CustomFunctions;
use rhai::{Engine, EvalAltResult};

/// Builder for creating configured Rhai engines for flag evaluation
pub struct FlagEvalEngineBuilder {
    engine: Engine,
    enable_custom_functions: bool,
}

impl FlagEvalEngineBuilder {
    /// Create a new engine builder with default settings
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            enable_custom_functions: false,
        }
    }

    /// Enable custom functions (semver, datetime, etc.)
    pub fn with_custom_functions(mut self) -> Self {
        self.enable_custom_functions = true;
        self
    }

    /// Build the final engine with all configured options
    pub fn build(mut self) -> Result<FlagEvalEngine, Box<EvalAltResult>> {
        if self.enable_custom_functions {
            CustomFunctions::register_all(&mut self.engine)?;
        }
        
        Ok(FlagEvalEngine(self.engine))
    }
}

impl Default for FlagEvalEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper around Rhai Engine with custom functions for flag evaluation
#[derive(Debug)]
pub struct FlagEvalEngine(Engine);

impl FlagEvalEngine {
    /// Create a new basic engine (equivalent to the old `Engine::new()`)
    pub fn new() -> Self {
        Self(Engine::new())
    }

    /// Create an engine with all custom functions enabled
    pub fn new_with_custom_functions() -> Result<Self, Box<EvalAltResult>> {
        FlagEvalEngineBuilder::new()
            .with_custom_functions()
            .build()
    }
}

impl Default for FlagEvalEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Allow FlagEvalEngine to be used as a Rhai Engine
impl std::ops::Deref for FlagEvalEngine {
    type Target = Engine;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FlagEvalEngine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Scope;

    #[test]
    fn test_basic_engine_creation() {
        let engine = FlagEvalEngineBuilder::new().build().unwrap();
        
        // Test basic evaluation
        let result: bool = engine.eval("true && false").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_engine_with_custom_functions() {
        let engine = FlagEvalEngineBuilder::new()
            .with_custom_functions()
            .build()
            .unwrap();
        
        // Test semver function
        let result: bool = engine.eval(r#"semver("2.0.0", ">", "1.9.9")"#).unwrap();
        assert!(result);
    }

    #[test]
    fn test_convenience_constructors() {
        // Test basic constructor
        let _engine = FlagEvalEngine::new();
        
        // Test constructor with custom functions
        let engine = FlagEvalEngine::new_with_custom_functions().unwrap();
        let result: bool = engine.eval(r#"semver("2.0.0", ">=", "2.0.0")"#).unwrap();
        assert!(result);
    }

    #[test]
    fn test_engine_with_scope() {
        let engine = FlagEvalEngineBuilder::new()
            .with_custom_functions()
            .build()
            .unwrap();
        
        let mut scope = Scope::new();
        scope.push("app_version", "2.1.0");
        scope.push("min_version", "2.0.0");
        
        let result: bool = engine.eval_with_scope(&mut scope, r#"semver(app_version, ">=", min_version)"#).unwrap();
        assert!(result);
    }
}
