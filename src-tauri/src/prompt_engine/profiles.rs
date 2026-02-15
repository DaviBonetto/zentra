// prompt_engine/profiles.rs — Profile loading and validation

use std::collections::HashMap;
use super::types::{EngineError, Profile, ProfilesConfig};

/// Load profiles from a JSON file path
pub fn load_profiles(path: &str) -> Result<HashMap<String, Profile>, EngineError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| EngineError::ConfigError(format!("Failed to read {}: {}", path, e)))?;

    let config: ProfilesConfig = serde_json::from_str(&content)
        .map_err(|e| EngineError::ConfigError(format!("Invalid JSON in {}: {}", path, e)))?;

    let mut profiles = HashMap::new();
    for profile in config.profiles {
        // Validate required fields
        if profile.id.is_empty() || profile.goal.is_empty() {
            return Err(EngineError::ConfigError(format!(
                "Profile missing required fields: id='{}', goal='{}'",
                profile.id, profile.goal
            )));
        }
        profiles.insert(profile.id.clone(), profile);
    }

    tracing::info!("Loaded {} profiles from {}", profiles.len(), path);
    Ok(profiles)
}
