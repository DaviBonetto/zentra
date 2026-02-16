use super::ProviderConfig;
use crate::stt::GroqAdapter;
use std::env;

pub fn default_providers_from_env() -> Vec<ProviderConfig> {
    let mut providers = Vec::new();

    if let Some(key) = env::var("GROQ_API_KEY").ok().filter(|k| k.starts_with("gsk_")) {
        providers.push(ProviderConfig {
            id: "groq".to_string(),
            priority: 1,
            adapter: Box::new(GroqAdapter::new(key)),
            max_retries: 0,
            timeout_secs: 10,
            confidence_threshold: 0.7,
        });
    }

    providers
}
