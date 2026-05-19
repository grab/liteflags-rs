use crate::dto::*;

#[derive(Debug)]
pub struct FlagStore {
    pub(crate) flags: NamespaceFlagsMap,
}

impl FlagStore {
    pub fn new(flags: NamespaceFlagsMap) -> Self {
        Self { flags }
    }

    pub fn update_flags(&self, new_flags: NamespaceFlagsMap) {
        // Clear existing data
        self.flags.0.clear();
        
        // Copy new data
        for entry in new_flags.0.iter() {
            let (namespace, flags) = entry.pair();
            self.flags.0.insert(namespace.clone(), flags.clone());
        }
    }

    pub fn get_flag(&self, namespace: &str, flag_name: &str) -> Option<FlagDefinition> {
        self.flags.0.get(namespace)?
            .0.get(flag_name)
            .map(|entry| entry.value().clone())
    }

    pub fn get_all_namepsaces(&self) -> Vec<String> {
        self.flags.0.iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn get_namespace_flags(&self, namespace: &str) -> Option<Flags> {
        self.flags.0.get(namespace).map(|entry| entry.value().clone())
    }
} 