use crate::config::{Action, Rules};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        static STARTED_AT: OnceLock<Instant> = OnceLock::new();
        STARTED_AT.get_or_init(Instant::now).elapsed()
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
    next_stretch_at: Duration,
    next_yawn_at: Duration,
    last_auto_at: Option<Duration>,
    local_hour: u8,
    paused: bool,
}

impl Scheduler {
    pub fn new(rules: Rules, clock: &dyn Clock) -> Self {
        let now = clock.now();
        let sneeze_in = rules.sneeze_every_minutes.max(1.0) * 60.0;
        let knead_in = rules.knead_every_minutes.max(1.0) * 60.0;
        let stretch_in = rules.work_break_minutes.max(1.0) * 60.0;
        let yawn_in = rules.night_yawn_minutes.max(1.0) * 60.0;
        Self {
            rules,
            next_sneeze_at: now + Duration::from_secs_f64(sneeze_in),
            next_knead_at: now + Duration::from_secs_f64(knead_in),
            next_stretch_at: now + Duration::from_secs_f64(stretch_in),
            next_yawn_at: now + Duration::from_secs_f64(yawn_in),
            last_auto_at: None,
            local_hour: 12,
            paused: false,
        }
    }

    pub fn tick(&mut self, clock: &dyn Clock) -> Option<Vec<Action>> {
        if self.paused {
            return None;
        }
        let now = clock.now();
        let cooldown = Duration::from_secs_f64(self.rules.action_cooldown_seconds);
        if self.last_auto_at.is_some_and(|last| now.saturating_sub(last) < cooldown) {
            return None;
        }
        let is_night = self.is_night();
        if self.rules.behavior_enabled && self.rules.night_quiet_enabled && is_night {
            if now >= self.next_yawn_at {
                self.next_yawn_at = now + Duration::from_secs_f64(self.rules.night_yawn_minutes * 60.0);
                self.last_auto_at = Some(now);
                return Some(vec![Action::Yawning, Action::Contented]);
            }
            return None;
        }
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
            self.last_auto_at = Some(now);
            Some(Self::sequence_for(Action::Sneezing))
        } else if knead_due {
            self.next_knead_at = now
                + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            self.last_auto_at = Some(now);
            Some(Self::sequence_for(Action::Kneading))
        } else if self.rules.behavior_enabled && now >= self.next_stretch_at {
            self.next_stretch_at = now
                + Duration::from_secs_f64(self.rules.work_break_minutes.max(1.0) * 60.0);
            self.last_auto_at = Some(now);
            Some(vec![Action::Stretching])
        } else {
            None
        }
    }

    pub fn sequence_for(action: Action) -> Vec<Action> {
        match action {
            Action::Sneezing => vec![Action::Sneezing, Action::RubNose],
            Action::Kneading => vec![Action::Kneading, Action::Contented],
            Action::Idle => vec![],
            other => vec![other],
        }
    }

    pub fn set_local_hour(&mut self, hour: u8) {
        self.local_hour = hour.min(23);
    }

    fn is_night(&self) -> bool {
        let start = self.rules.night_start_hour;
        let end = self.rules.night_end_hour;
        if start == end { return false; }
        if start > end { self.local_hour >= start || self.local_hour < end }
        else { self.local_hour >= start && self.local_hour < end }
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
            Action::Idle | Action::RubNose | Action::Contented | Action::HeadTilt
            | Action::Yawning | Action::Stretching => {}
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
        self.next_stretch_at = now
            + Duration::from_secs_f64(self.rules.work_break_minutes.max(1.0) * 60.0);
        self.next_yawn_at = now
            + Duration::from_secs_f64(self.rules.night_yawn_minutes.max(1.0) * 60.0);
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
            ..Rules::default_rules()
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
        assert_eq!(s.tick(&clock), Some(vec![Action::Sneezing, Action::RubNose]));
        // Immediately after, no event (timer was reset).
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn fires_knead_after_knead_interval() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 1.0), &clock);
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(vec![Action::Kneading, Action::Contented]));
    }

    #[test]
    fn sneeze_takes_priority_over_knead_when_both_due() {
        let clock = ManualClock::new(Duration::from_secs(0));
        // Both intervals are 1 minute.
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        clock.advance(120.0); // both are well overdue
        assert_eq!(s.tick(&clock), Some(vec![Action::Sneezing, Action::RubNose]));
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
        assert_eq!(s.tick(&clock), Some(vec![Action::Sneezing, Action::RubNose]));
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
        assert_eq!(s.tick(&clock), Some(vec![Action::Kneading, Action::Contented]));
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

    #[test]
    fn night_mode_suppresses_normal_actions_and_yawns() {
        let clock = ManualClock::new(Duration::ZERO);
        let mut r = rules(1.0, 1.0);
        r.night_yawn_minutes = 1.0;
        let mut s = Scheduler::new(r, &clock);
        s.set_local_hour(23);
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(vec![Action::Yawning, Action::Contented]));
    }

    #[test]
    fn work_break_stretches_during_day() {
        let clock = ManualClock::new(Duration::ZERO);
        let mut r = rules(30.0, 30.0);
        r.work_break_minutes = 1.0;
        let mut s = Scheduler::new(r, &clock);
        s.set_local_hour(12);
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(vec![Action::Stretching]));
    }
}
