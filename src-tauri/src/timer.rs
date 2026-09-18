use crate::config::{Action, Rules};
use std::sync::Mutex;
use std::time::Duration;

pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        // Use monotonic time. std::time::Instant is monotonic but can't be exported as a Duration from epoch;
        // for production use we need the elapsed-since-start time. Using SystemTime is not monotonic.
        // The pragmatic choice: track an epoch inside Scheduler using SystemClock::now as a Duration
        // from UNIX_EPOCH is fine because we only compare within a single run.
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
    }
}

pub struct ManualClock {
    now: Mutex<Duration>,
}

impl ManualClock {
    pub fn new(start: Duration) -> Self {
        Self { now: Mutex::new(start) }
    }
    pub fn advance(&self, secs: f64) {
        let mut n = self.now.lock().unwrap();
        *n += Duration::from_secs_f64(secs);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

pub struct Scheduler {
    rules: Rules,
    next_sneeze_at: Duration,
    next_knead_at: Duration,
    paused: bool,
}

impl Scheduler {
    pub fn new(rules: Rules, clock: &dyn Clock) -> Self {
        let now = clock.now();
        let sneeze_in = rules.sneeze_every_minutes.max(1.0) * 60.0;
        let knead_in = rules.knead_every_minutes.max(1.0) * 60.0;
        Self {
            rules,
            next_sneeze_at: now + Duration::from_secs_f64(sneeze_in),
            next_knead_at: now + Duration::from_secs_f64(knead_in),
            paused: false,
        }
    }

    pub fn tick(&mut self, clock: &dyn Clock) -> Option<Action> {
        if self.paused {
            return None;
        }
        let now = clock.now();
        let sneeze_due = now >= self.next_sneeze_at;
        let knead_due = now >= self.next_knead_at;
        if sneeze_due {
            self.next_sneeze_at = now
                + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
            // When sneeze wins priority, also advance any other due timer so we
            // don't fire a second action immediately on the next tick.
            if knead_due {
                self.next_knead_at = now
                    + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            }
            Some(Action::Sneezing)
        } else if knead_due {
            self.next_knead_at = now
                + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            Some(Action::Kneading)
        } else {
            None
        }
    }

    pub fn trigger(&mut self, action: Action, clock: &dyn Clock) {
        if action == Action::Idle {
            return;
        }
        let now = clock.now();
        match action {
            Action::Sneezing => {
                self.next_sneeze_at = now
                    + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
            }
            Action::Kneading => {
                self.next_knead_at = now
                    + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            }
            Action::Idle => {}
        }
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn reload_rules(&mut self, rules: Rules, clock: &dyn Clock) {
        self.rules = rules;
        let now = clock.now();
        // After reloading, push next events out by at least one interval so the user
        // gets a fresh window after editing the rules.
        self.next_sneeze_at = now
            + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
        self.next_knead_at = now
            + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rules;

    fn rules(min_sneeze: f64, min_knead: f64) -> Rules {
        Rules {
            sneeze_every_minutes: min_sneeze,
            knead_every_minutes: min_knead,
            single_click: "kneading".to_string(),
            double_click: "sneezing".to_string(),
        }
    }

    #[test]
    fn new_scheduler_does_not_fire_immediately() {
        let clock = ManualClock::new(Duration::from_secs(1_000_000));
        let s = Scheduler::new(rules(30.0, 5.0), &clock);
        let mut s = s;
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn fires_sneeze_after_sneeze_interval() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        // After exactly 60s (1 minute) we should get a sneeze.
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
        // Immediately after, no event (timer was reset).
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn fires_knead_after_knead_interval() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 1.0), &clock);
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(Action::Kneading));
    }

    #[test]
    fn sneeze_takes_priority_over_knead_when_both_due() {
        let clock = ManualClock::new(Duration::from_secs(0));
        // Both intervals are 1 minute.
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        clock.advance(120.0); // both are well overdue
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
    }

    #[test]
    fn paused_blocks_all_ticks() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        s.set_paused(true);
        clock.advance(3600.0);
        assert!(s.tick(&clock).is_none());
        s.set_paused(false);
        clock.advance(3600.0);
        // Should now fire
        assert!(s.tick(&clock).is_some());
    }

    #[test]
    fn manual_trigger_pushes_next_event_out() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        // Manually trigger sneeze at t=10s; next sneeze should be t=10+30*60.
        clock.advance(10.0);
        s.trigger(Action::Sneezing, &clock);
        // At t=11s, no sneeze
        clock.advance(1.0);
        assert!(s.tick(&clock).is_none());
        // Jump to 30*60+10 = 1810s total
        clock.advance(30.0 * 60.0);
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
    }

    #[test]
    fn trigger_idle_is_a_noop() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        clock.advance(10.0);
        s.trigger(Action::Idle, &clock);
        // Schedule unchanged: at 11s no event
        clock.advance(1.0);
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn reload_rules_resets_schedule() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        clock.advance(60.0);
        // Without reload, knead should fire at t=60s (5 min interval set in test setup is wrong — actually 5 min from t=0 → t=300s)
        // Reload with 1-min knead interval.
        s.reload_rules(rules(30.0, 1.0), &clock);
        clock.advance(60.0); // total t=120s
        assert_eq!(s.tick(&clock), Some(Action::Kneading));
    }

    #[test]
    fn minimum_interval_is_one_minute() {
        let clock = ManualClock::new(Duration::from_secs(0));
        // 0 minutes should be treated as 1 minute floor, matching the spec.
        let mut s = Scheduler::new(rules(0.0, 0.0), &clock);
        clock.advance(60.0); // exactly 1 minute
        // Should fire (whichever has priority)
        assert!(s.tick(&clock).is_some());
    }
}