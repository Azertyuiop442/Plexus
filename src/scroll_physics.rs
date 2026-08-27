
use std::time::{Duration, Instant};

pub const ACCEL_WINDOW_MS: u64 = 250;

pub const SCROLL_CAP_DIVISOR: u32 = 2;

pub const ACCEL_SATURATION: u32 = 7;

pub const DECAY_TAU_MS: f64 = 120.0;

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    last_event: Option<Instant>,
    momentum: f64,
    last_direction: i32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            last_event: None,
            momentum: 0.0,
            last_direction: 0,
        }
    }

    pub fn apply(&mut self, delta: i32, viewport_h: u16) -> i32 {
        let now = Instant::now();
        let dir = delta.signum();
        if dir == 0 {
            return 0;
        }

        let flipped = self.last_direction != 0 && self.last_direction != dir;
        let stale = self
            .last_event
            .map(|t| now.duration_since(t) >= Duration::from_millis(ACCEL_WINDOW_MS))
            .unwrap_or(true);
        if flipped || stale {
            self.momentum = 0.0;
        } else if let Some(last) = self.last_event {
            let dt_ms = now.duration_since(last).as_secs_f64() * 1000.0;
            if dt_ms > 0.0 {
                self.momentum *= (-dt_ms / DECAY_TAU_MS).exp();
            }
        }
        self.momentum = (self.momentum + 1.0).min(ACCEL_SATURATION as f64);
        self.last_direction = dir;
        self.last_event = Some(now);

        let t = (self.momentum - 1.0) / (ACCEL_SATURATION as f64 - 1.0);
        let accel = 1.0 + smoothstep(t.clamp(0.0, 1.0)) * (ACCEL_SATURATION as f64 - 1.0) * 0.5;
        let scaled = (delta as f64 * accel).round() as i32;

        let cap = (viewport_h as u32 / SCROLL_CAP_DIVISOR).max(1) as i32;
        dir * scaled.abs().min(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_event_scrolls_one_line() {
        let mut s = ScrollState::new();
        assert_eq!(s.apply(3, 24), 3);
    }

    #[test]
    fn rapid_burst_accelerates_and_caps() {
        let mut s = ScrollState::new();
        assert_eq!(s.apply(3, 24), 3);
        assert!(s.apply(3, 24) >= 3, "second event must not shrink");
        let mut last = 3;
        let mut saturated = false;
        for _ in 0..10 {
            let d = s.apply(3, 24);
            assert!(d >= last, "burst must be monotonic, got {d} after {last}");
            last = d;
            if d == 12 {
                saturated = true;
                break;
            }
        }
        assert!(saturated, "burst must reach the half-viewport cap 12");
    }

    #[test]
    fn direction_flip_resets_momentum() {
        let mut s = ScrollState::new();
        s.apply(3, 24);
        s.apply(3, 24);
        assert_eq!(s.apply(-3, 24), -3);
        assert!(s.apply(-3, 24) < 0, "flip restarts acceleration");
    }

    #[test]
    fn inactivity_expires_momentum() {
        let mut s = ScrollState::new();
        s.apply(3, 24);
        s.apply(3, 24);
        s.last_event = Some(Instant::now() - Duration::from_millis(ACCEL_WINDOW_MS + 10));
        assert_eq!(s.apply(3, 24), 3);
    }

    #[test]
    fn momentum_decays_exponentially_between_events() {
        let mut s = ScrollState::new();
        s.apply(3, 24);
        s.apply(3, 24);
        let boosted = s.momentum;
        assert!(boosted > 1.5, "burst builds momentum ({boosted})");
        s.last_event = Some(Instant::now() - Duration::from_millis(DECAY_TAU_MS as u64));
        s.apply(3, 24);
        assert!(
            s.momentum < boosted,
            "slow wheel must decay momentum ({:?})",
            s.momentum
        );
    }

    #[test]
    fn cap_respects_small_viewports() {
        let mut s = ScrollState::new();
        for _ in 0..6 {
            let d = s.apply(3, 6);
            assert!(d <= 3, "delta {d} exceeded half-viewport cap");
        }
    }
}

