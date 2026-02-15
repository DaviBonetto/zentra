use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open { tripped_at: Instant },
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u8,
    last_failure_time: Option<Instant>,
    trip_threshold: u8,
    trip_window: Duration,
    cooldown: Duration,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: None,
            trip_threshold: 3,
            trip_window: Duration::from_secs(300),
            cooldown: Duration::from_secs(600),
        }
    }

    pub fn is_request_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open { tripped_at } => {
                if tripped_at.elapsed() >= self.cooldown {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_failure_time = None;
        self.state = CircuitState::Closed;
    }

    pub fn record_failure(&mut self) {
        let now = Instant::now();

        if let Some(last_fail) = self.last_failure_time {
            if now.duration_since(last_fail) > self.trip_window {
                self.failure_count = 1;
            } else {
                self.failure_count = self.failure_count.saturating_add(1);
            }
        } else {
            self.failure_count = 1;
        }

        self.last_failure_time = Some(now);

        if self.failure_count >= self.trip_threshold {
            self.state = CircuitState::Open { tripped_at: now };
            tracing::warn!("Circuit breaker tripped, failure_count={}", self.failure_count);
        }
    }
}
