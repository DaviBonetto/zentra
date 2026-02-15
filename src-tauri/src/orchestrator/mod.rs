use crate::audio::AudioBuffer;
use crate::stt::{STTAdapter, STTError, Transcript};
use std::collections::HashMap;
use std::time::Duration;

use self::circuit_breaker::CircuitBreaker;
use self::metrics::Metrics;
use self::provider_registry::default_providers_from_env;
use self::retry::RetryPolicy;

pub mod circuit_breaker;
pub mod metrics;
pub mod provider_registry;
pub mod retry;

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("All providers failed")]
    AllProvidersFailed(Vec<(String, STTError)>),

    #[error("No providers available")]
    NoProvidersAvailable,
}

pub struct ProviderConfig {
    pub id: String,
    pub priority: u8,
    pub adapter: Box<dyn STTAdapter + Send + Sync>,
    pub max_retries: u8,
    pub timeout_secs: u64,
    pub confidence_threshold: f32,
}

pub struct FailoverOrchestrator {
    providers: Vec<ProviderConfig>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    metrics: Metrics,
}

impl FailoverOrchestrator {
    pub fn new(mut providers: Vec<ProviderConfig>) -> Self {
        providers.sort_by_key(|p| p.priority);

        let mut circuit_breakers = HashMap::new();
        for provider in &providers {
            circuit_breakers.insert(provider.id.clone(), CircuitBreaker::new());
        }

        Self {
            providers,
            circuit_breakers,
            metrics: Metrics::new(),
        }
    }

    pub fn from_env() -> Self {
        let providers = default_providers_from_env();
        Self::new(providers)
    }

    pub async fn transcribe(
        &mut self,
        audio: &AudioBuffer,
    ) -> Result<Transcript, OrchestratorError> {
        if self.providers.is_empty() {
            return Err(OrchestratorError::NoProvidersAvailable);
        }

        let mut all_errors = Vec::new();

        for provider in &self.providers {
            let allowed = {
                let cb = self
                    .circuit_breakers
                    .get_mut(&provider.id)
                    .expect("Circuit breaker missing");
                cb.is_request_allowed()
            };

            if !allowed {
                tracing::warn!(
                    "Provider {} skipped: circuit breaker open",
                    provider.id
                );
                all_errors.push((
                    provider.id.clone(),
                    STTError::ProviderError("Circuit breaker open".to_string()),
                ));
                continue;
            }

            tracing::info!(
                "Attempting provider: {} (priority {})",
                provider.id,
                provider.priority
            );

            let retry_policy = RetryPolicy::new(provider.max_retries);
            let mut attempt = 0u8;

            loop {
                match self.try_provider(provider, audio).await {
                    Ok(transcript) => {
                        if transcript.confidence >= provider.confidence_threshold {
                            tracing::info!(
                                "Provider {} succeeded: confidence={:.2}, text_len={}",
                                provider.id,
                                transcript.confidence,
                                transcript.text.len()
                            );

                            if let Some(cb) = self.circuit_breakers.get_mut(&provider.id) {
                                cb.record_success();
                            }
                            self.metrics.record_success(&provider.id);
                            return Ok(transcript);
                        }

                        tracing::warn!(
                            "Provider {} returned low confidence: {:.2} < {:.2}",
                            provider.id,
                            transcript.confidence,
                            provider.confidence_threshold
                        );

                        if let Some(cb) = self.circuit_breakers.get_mut(&provider.id) {
                            cb.record_failure();
                        }
                        self.metrics.record_failure(&provider.id);
                        all_errors.push((
                            provider.id.clone(),
                            STTError::ProviderError("Low confidence".to_string()),
                        ));
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Provider {} attempt {}/{} failed: {:?}",
                            provider.id,
                            attempt + 1,
                            provider.max_retries + 1,
                            e
                        );

                        if retry_policy.should_retry(attempt, &e) {
                            retry_policy.wait_before_retry(attempt).await;
                            attempt += 1;
                            continue;
                        }

                        if let Some(cb) = self.circuit_breakers.get_mut(&provider.id) {
                            cb.record_failure();
                        }
                        self.metrics.record_failure(&provider.id);
                        all_errors.push((provider.id.clone(), e));
                        break;
                    }
                }
            }
        }

        tracing::error!("All providers failed: {:?}", all_errors);
        Err(OrchestratorError::AllProvidersFailed(all_errors))
    }

    pub fn get_metrics(&self) -> &Metrics {
        &self.metrics
    }

    async fn try_provider(
        &self,
        provider: &ProviderConfig,
        audio: &AudioBuffer,
    ) -> Result<Transcript, STTError> {
        let timeout = Duration::from_secs(provider.timeout_secs);

        match tokio::time::timeout(timeout, provider.adapter.transcribe(audio)).await {
            Ok(result) => result,
            Err(_) => Err(STTError::TimeoutError),
        }
    }
}
