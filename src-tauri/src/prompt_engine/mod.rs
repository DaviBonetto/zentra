// prompt_engine/mod.rs - Main Prompt Engine

mod types;
mod profiles;
mod clarity;
mod llm;

pub use types::{EngineError, OptimizationMode, OptimizedPrompt, Profile};

use llm::LLMOrchestrator;
use std::collections::HashMap;

/// Prompt Engine - transforms transcripts into optimized LLM prompts
pub struct PromptEngine {
    profiles: HashMap<String, Profile>,
    llm: LLMOrchestrator,
    mode: OptimizationMode,
}

impl PromptEngine {
    /// Create from environment + config file
    pub fn new() -> Self {
        // Resolve config path relative to executable
        let config_path = Self::resolve_config_path();

        let profiles = match profiles::load_profiles(&config_path) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to load profiles: {}. Using defaults.", e);
                Self::default_profiles()
            }
        };

        let llm = LLMOrchestrator::from_env();

        tracing::info!(
            "PromptEngine initialized: {} profiles, mode=ClarityOnly",
            profiles.len()
        );

        Self {
            profiles,
            llm,
            mode: OptimizationMode::ClarityOnly,
        }
    }

    /// Optimize a transcript using the given profile
    pub async fn optimize(
        &self,
        transcript: &str,
        profile_id: &str,
    ) -> Result<OptimizedPrompt, EngineError> {
        let profile = self
            .profiles
            .get(profile_id)
            .ok_or_else(|| EngineError::ProfileNotFound(profile_id.to_string()))?;

        match self.mode {
            OptimizationMode::ClarityOnly => {
                let cleaned = clarity::transform(transcript);
                let text = self.apply_template(profile, &cleaned);

                Ok(OptimizedPrompt {
                    text,
                    profile_used: profile_id.to_string(),
                    mode: OptimizationMode::ClarityOnly,
                    provider: None,
                    confidence: 1.0,
                })
            }
            OptimizationMode::AIOptimize => {
                // First apply clarity, then send to LLM
                let cleaned = clarity::transform(transcript);
                let prompt = self.build_llm_prompt(profile, &cleaned);

                // Truncate to ~3000 tokens (~12000 chars)
                let truncated = if prompt.len() > 12000 {
                    format!("{}...[TRUNCATED]", &prompt[..12000])
                } else {
                    prompt
                };

                match self.llm.generate(&truncated).await {
                    Ok((text, provider)) => Ok(OptimizedPrompt {
                        text,
                        profile_used: profile_id.to_string(),
                        mode: OptimizationMode::AIOptimize,
                        provider: Some(provider),
                        confidence: 0.85,
                    }),
                    Err(e) => {
                        tracing::warn!("LLM failed, falling back to clarity-only: {:?}", e);
                        // Graceful fallback to clarity-only
                        let text = self.apply_template(profile, &cleaned);
                        Ok(OptimizedPrompt {
                            text,
                            profile_used: profile_id.to_string(),
                            mode: OptimizationMode::ClarityOnly,
                            provider: None,
                            confidence: 0.5,
                        })
                    }
                }
            }
        }
    }

    /// Set the optimization mode
    pub fn set_mode(&mut self, mode: OptimizationMode) {
        tracing::info!("PromptEngine mode changed to: {:?}", mode);
        self.mode = mode;
    }

    /// List available profiles
    pub fn list_profiles(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }

    // --- Private helpers ---

    fn apply_template(&self, profile: &Profile, transcript: &str) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let context = profile
            .context_template
            .replace("{{transcript}}", transcript)
            .replace("{{datetime}}", &now)
            .replace("{{app_context}}", "Voice AI Desktop");

        format!(
            "# GOAL\n{}\n\n# RETURN FORMAT\n{}\n\n# WARNINGS\n{}\n\n# CONTEXT\n{}",
            profile.goal,
            profile.return_format,
            profile
                .warnings
                .iter()
                .map(|w| format!("- {}", w))
                .collect::<Vec<_>>()
                .join("\n"),
            context
        )
    }

    fn build_llm_prompt(&self, profile: &Profile, transcript: &str) -> String {
        let template = self.apply_template(profile, transcript);

        format!(
            "Voce e um assistente de otimizacao de prompts.\n\n\
            O usuario disse o seguinte (transcricao fiel):\n\
            \"{}\"\n\n\
            Organize e estruture o que foi dito de forma clara para ser usado como input para uma IA.\n\
            Mantenha todas as informacoes que o usuario mencionou.\n\
            Nao invente nem adicione informacoes.\n\n\
            Template do profile (mantenha GOAL, RETURN FORMAT e WARNINGS como estao; refine apenas CONTEXT):\n\n{}",
            transcript,
            template,
        )
    }

    fn resolve_config_path() -> String {
        // Try relative to executable first, then fallback paths
        let paths = [
            "config/profiles.json".to_string(),
            "../config/profiles.json".to_string(),
            "src-tauri/config/profiles.json".to_string(),
        ];

        for path in &paths {
            if std::path::Path::new(path).exists() {
                return path.clone();
            }
        }

        // Default (will be caught by load_profiles error handler)
        "config/profiles.json".to_string()
    }

    fn default_profiles() -> HashMap<String, Profile> {
        let mut profiles = HashMap::new();
        profiles.insert(
            "clarity".to_string(),
            Profile {
                id: "clarity".to_string(),
                name: "Clarity Only".to_string(),
                goal: "Melhorar clareza e gramatica da transcricao".to_string(),
                return_format: "Texto limpo e correto".to_string(),
                warnings: vec!["NAO adicionar conteudo extra".to_string()],
                context_template: "{{transcript}}".to_string(),
            },
        );
        profiles
    }
}

impl Default for PromptEngine {
    fn default() -> Self {
        Self::new()
    }
}
