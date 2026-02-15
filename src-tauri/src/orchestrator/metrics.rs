use std::collections::HashMap;

pub struct Metrics {
    success_counts: HashMap<String, u64>,
    failure_counts: HashMap<String, u64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            success_counts: HashMap::new(),
            failure_counts: HashMap::new(),
        }
    }

    pub fn record_success(&mut self, provider_id: &str) {
        *self
            .success_counts
            .entry(provider_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn record_failure(&mut self, provider_id: &str) {
        *self
            .failure_counts
            .entry(provider_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn get_success_count(&self, provider_id: &str) -> u64 {
        *self.success_counts.get(provider_id).unwrap_or(&0)
    }

    pub fn get_failure_count(&self, provider_id: &str) -> u64 {
        *self.failure_counts.get(provider_id).unwrap_or(&0)
    }

    pub fn get_success_rate(&self, provider_id: &str) -> f32 {
        let success = self.get_success_count(provider_id) as f32;
        let total = success + self.get_failure_count(provider_id) as f32;

        if total == 0.0 {
            0.0
        } else {
            success / total
        }
    }
}
