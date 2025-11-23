use rand::Rng;
use std::time::Duration;
use log::{info, warn};

pub struct CooldownManager {
    base_cooldown: f64,
    current_estimate: f64,
    consecutive_hits: u32,
    success_streak: u32,
}

impl CooldownManager {
    pub fn new(base_cooldown: f64) -> Self {
        Self {
            base_cooldown,
            current_estimate: base_cooldown,
            consecutive_hits: 0,
            success_streak: 0,
        }
    }

    pub fn get_sleep_time(&self) -> Duration {
        let mut rng = rand::thread_rng();
        // Base delay + small random jitter to mimic human behavior
        // If we've been hitting cooldowns, we add a penalty buffer
        let penalty = if self.consecutive_hits > 0 {
            self.consecutive_hits as f64 * 0.5 // Add 0.5s per consecutive hit
        } else {
            0.0
        };

        // Gaussian-like distribution: prefer values slightly above estimate
        let jitter = rng.gen_range(0.1..0.8); // Human variance
        let delay = self.current_estimate + penalty + jitter;

        Duration::from_secs_f64(delay)
    }

    pub fn report_cooldown_hit(&mut self, wait_time: f64, total_cooldown: f64) {
        self.consecutive_hits += 1;
        self.success_streak = 0;

        // If server says total cooldown is X, update our estimate
        if total_cooldown > 0.0 && total_cooldown > self.current_estimate {
            info!("Updating cooldown estimate from {:.2}s to {:.2}s", self.current_estimate, total_cooldown);
            self.current_estimate = total_cooldown;
        } else if wait_time > 0.0 {
             // If we don't get total, infer it might be current + wait
             // But usually wait_time is just the remainder.
             // We just respect the penalty for now.
        }

        warn!("Cooldown hit! Consecutive: {}", self.consecutive_hits);
    }

    pub fn report_success(&mut self) {
        self.success_streak += 1;
        self.consecutive_hits = 0;

        // Slowly decay estimate if we are super successful, to find the edge?
        // Or just stick to the safe estimate.
        // For now, if we are successful for a long time, we might reduce the "penalty" buffer logic implicitly
        // by resetting consecutive_hits.

        // Optional: if success streak is huge, maybe try shaving off 0.1s?
        // This effectively "probes" for the fastest possible rate.
        if self.success_streak > 20 && self.current_estimate > self.base_cooldown {
             self.current_estimate -= 0.05;
             if self.current_estimate < self.base_cooldown {
                 self.current_estimate = self.base_cooldown;
             }
             // info!("Decaying cooldown estimate to {:.2}s", self.current_estimate);
             // Reset streak so we don't decay too fast
             self.success_streak = 0;
        }
    }
}
