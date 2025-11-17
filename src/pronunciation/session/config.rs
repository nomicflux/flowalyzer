use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub sample_rate: u32,
    pub chunk_duration_ms: u32,
    pub latency_budget_ms: u32,
    pub latency_range: RangeInclusive<u32>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            chunk_duration_ms: 100,
            latency_budget_ms: 200,
            latency_range: 100..=200,
        }
    }
}
