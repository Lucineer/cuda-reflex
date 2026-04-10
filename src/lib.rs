/*!
# cuda-reflex

Biological reflex arcs for agents.

When you touch a hot stove, your hand pulls back before your brain
even registers pain. That's a reflex. No deliberation. No consideration.
Stimulus → spinal cord → response. ~50ms.

Agents need the same. Some situations are too dangerous for deliberation.
An agent that thinks before dodging is a dead agent.

Reflexes are:
- Fast (no deliberation overhead)
- Hardcoded (not learned, evolved)
- Overridable (deliberation can suppress reflexes)
- Composable (multiple reflexes can fire simultaneously)
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reflex stimulus — what triggers the reflex
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Stimulus {
    Temperature(f64),        // degrees C
    Proximity(f64),          // distance to obstacle
    Velocity(f64),           // rate of approach
    Impact(f64),             // collision force
    SignalStrength(f64),     // radio/sensor signal
    EnergyLevel(f64),        // battery/ATP fraction
    MemoryPressure(f64),     // working memory usage
    ErrorRate(f64),          // recent failure rate
    Custom(String),          // domain-specific stimulus
}

/// Reflex response — what the reflex does
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Stop,
    Retreat(f64),            // retreat at speed
    Dodge { direction: (f64, f64) },
    Shield,                  // activate protective mode
    Alert,                   // send alarm
    Throttle(f64),           // reduce speed to fraction
    Halt,                    // emergency stop
    EvasiveManeuver,         // complex avoidance
    Broadcast(String),       // send message to fleet
    SelfPreserve,            // enter self-preservation mode
    Custom(String),
}

/// Priority levels — higher reflexes override lower
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReflexPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    Survival = 4,           // cannot be overridden
}

/// A single reflex arc
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reflex {
    pub name: String,
    pub stimulus: Stimulus,
    pub threshold: f64,      // activation threshold
    pub response: Response,
    pub priority: ReflexPriority,
    pub enabled: bool,
    pub fire_count: u32,
    pub last_fired: u64,
    pub cooldown_ms: u64,
    pub suppressible: bool,  // can deliberation override this?
}

impl Reflex {
    pub fn new(name: &str, stimulus: Stimulus, threshold: f64, response: Response, priority: ReflexPriority) -> Self {
        Reflex { name: name.to_string(), stimulus, threshold, response, priority, enabled: true, fire_count: 0, last_fired: 0, cooldown_ms: 0, suppressible: priority != ReflexPriority::Survival }
    }

    /// Check if this reflex should fire
    pub fn should_fire(&self, stimulus: &Stimulus, now: u64) -> bool {
        if !self.enabled { return false; }
        if now - self.last_fired < self.cooldown_ms { return false; }
        self.matches(stimulus)
    }

    fn matches(&self, stimulus: &Stimulus) -> bool {
        match (&self.stimulus, stimulus) {
            (Stimulus::Temperature(t), Stimulus::Temperature(v)) => *v > *t,
            (Stimulus::Proximity(p), Stimulus::Proximity(v)) => *v < *p,
            (Stimulus::Velocity(vel), Stimulus::Velocity(v)) => *v > *vel,
            (Stimulus::Impact(f), Stimulus::Impact(v)) => *v > *f,
            (Stimulus::SignalStrength(s), Stimulus::SignalStrength(v)) => *v < *s,
            (Stimulus::EnergyLevel(e), Stimulus::EnergyLevel(v)) => *v < *e,
            (Stimulus::MemoryPressure(m), Stimulus::MemoryPressure(v)) => *v > *m,
            (Stimulus::ErrorRate(r), Stimulus::ErrorRate(v)) => *v > *r,
            (Stimulus::Custom(a), Stimulus::Custom(b)) => a == b,
            _ => false,
        }
    }
}

/// The reflex system — manages all reflex arcs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflexSystem {
    pub reflexes: Vec<Reflex>,
    pub suppressed: Vec<String>,
    pub log: Vec<ReflexLogEntry>,
    pub max_log: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflexLogEntry {
    pub name: String,
    pub stimulus: String,
    pub response: String,
    pub timestamp: u64,
    pub override: bool,  // was deliberation active?
}

impl ReflexSystem {
    pub fn new() -> Self { ReflexSystem { reflexes: vec![], suppressed: vec![], log: vec![], max_log: 100 } }

    pub fn add(&mut self, reflex: Reflex) {
        self.reflexes.push(reflex);
    }

    /// Add built-in safety reflexes
    pub fn add_defaults(&mut self) {
        // Hot stove — temperature > 60C
        self.add(Reflex::new("thermal_retreat", Stimulus::Temperature(60.0), 0.5, Response::Retreat(1.0), ReflexPriority::Survival));

        // Proximity — too close to obstacle
        self.add(Reflex::new("proximity_dodge", Stimulus::Proximity(0.5), 0.5, Response::Dodge { direction: (1.0, 0.0) }, ReflexPriority::High));

        // High velocity approach
        self.add(Reflex::new("collision_avoid", Stimulus::Velocity(2.0), 0.5, Response::Halt, ReflexPriority::Critical));

        // Impact
        self.add(Reflex::new("impact_shield", Stimulus::Impact(0.8), 0.5, Response::Shield, ReflexPriority::Critical));

        // Low energy
        self.add(Reflex::new("low_energy_throttle", Stimulus::EnergyLevel(0.1), 0.5, Response::Throttle(0.2), ReflexPriority::High));

        // Critical energy — self-preserve
        self.add(Reflex::new("critical_energy", Stimulus::EnergyLevel(0.05), 0.5, Response::SelfPreserve, ReflexPriority::Survival));

        // Memory pressure
        self.add(Reflex::new("memory_flush", Stimulus::MemoryPressure(0.9), 0.5, Response::Throttle(0.5), ReflexPriority::Normal));

        // High error rate
        self.add(Reflex::new("error_throttle", Stimulus::ErrorRate(0.5), 0.5, Response::Throttle(0.3), ReflexPriority::Normal));
    }

    /// Process stimulus, return highest-priority response
    pub fn process(&mut self, stimulus: Stimulus, deliberation_active: bool) -> Option<Response> {
        let now = now();
        let mut candidates: Vec<&Reflex> = self.reflexes.iter()
            .filter(|r| r.should_fire(&stimulus, now))
            .filter(|r| !self.suppressed.contains(&r.name))
            .filter(|r| !deliberation_active || !r.suppressible) // only unsuppressible fire during deliberation
            .collect();

        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

        if let Some(reflex) = candidates.first() {
            let name = reflex.name.clone();
            let response = reflex.response.clone();

            // Log
            self.log.push(ReflexLogEntry {
                name: name.clone(),
                stimulus: format!("{:?}", stimulus),
                response: format!("{:?}", response),
                timestamp: now,
                override: deliberation_active,
            });
            if self.log.len() > self.max_log { self.log.remove(0); }

            Some(response)
        } else {
            None
        }
    }

    /// Suppress a reflex by name
    pub fn suppress(&mut self, name: &str) {
        if !self.suppressed.contains(&name.to_string()) {
            self.suppressed.push(name.to_string());
        }
    }

    /// Unsuppress a reflex
    pub fn unsuppress(&mut self, name: &str) {
        self.suppressed.retain(|s| s != name);
    }

    /// Enable/disable by name
    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(r) = self.reflexes.iter_mut().find(|r| r.name == name) {
            r.enabled = enabled;
        }
    }

    /// Statistics
    pub fn stats(&self) -> ReflexStats {
        let total_fires: u32 = self.reflexes.iter().map(|r| r.fire_count).sum();
        let active = self.reflexes.iter().filter(|r| r.enabled).count();
        let suppressed = self.suppressed.len();
        ReflexStats { total_reflexes: self.reflexes.len(), active, suppressed, total_fires, log_entries: self.log.len() }
    }
}

#[derive(Clone, Debug)]
pub struct ReflexStats {
    pub total_reflexes: usize,
    pub active: usize,
    pub suppressed: usize,
    pub total_fires: u32,
    pub log_entries: usize,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflex_creation() {
        let r = Reflex::new("test", Stimulus::Temperature(50.0), 0.5, Response::Stop, ReflexPriority::High);
        assert!(r.enabled);
    }

    #[test]
    fn test_reflex_match_temperature() {
        let r = Reflex::new("hot", Stimulus::Temperature(60.0), 0.5, Response::Retreat(1.0), ReflexPriority::Survival);
        assert!(r.matches(&Stimulus::Temperature(70.0))); // hotter than threshold
        assert!(!r.matches(&Stimulus::Temperature(40.0))); // cooler
    }

    #[test]
    fn test_reflex_match_proximity() {
        let r = Reflex::new("close", Stimulus::Proximity(1.0), 0.5, Response::Dodge { direction: (0.0, 1.0) }, ReflexPriority::High);
        assert!(r.matches(&Stimulus::Proximity(0.5))); // closer than threshold
        assert!(!r.matches(&Stimulus::Proximity(2.0))); // farther
    }

    #[test]
    fn test_reflex_match_energy() {
        let r = Reflex::new("low", Stimulus::EnergyLevel(0.1), 0.5, Response::Throttle(0.2), ReflexPriority::High);
        assert!(r.matches(&Stimulus::EnergyLevel(0.05))); // below threshold
        assert!(!r.matches(&Stimulus::EnergyLevel(0.5))); // above
    }

    #[test]
    fn test_system_process() {
        let mut sys = ReflexSystem::new();
        sys.add_defaults();
        let resp = sys.process(Stimulus::Temperature(80.0), false);
        assert!(resp.is_some());
    }

    #[test]
    fn test_system_priority() {
        let mut sys = ReflexSystem::new();
        sys.add(Reflex::new("low", Stimulus::EnergyLevel(0.5), 0.5, Response::Throttle(0.5), ReflexPriority::Low));
        sys.add(Reflex::new("critical", Stimulus::EnergyLevel(0.03), 0.5, Response::SelfPreserve, ReflexPriority::Survival));
        let resp = sys.process(Stimulus::EnergyLevel(0.03), false);
        assert!(resp.is_some());
        // Should pick SelfPreserve (survival) over Throttle (low)
    }

    #[test]
    fn test_suppression() {
        let mut sys = ReflexSystem::new();
        sys.add(Reflex::new("test", Stimulus::Temperature(50.0), 0.5, Response::Stop, ReflexPriority::Normal));
        sys.suppress("test");
        let resp = sys.process(Stimulus::Temperature(80.0), false);
        assert!(resp.is_none());
    }

    #[test]
    fn test_deliberation_override() {
        let mut sys = ReflexSystem::new();
        sys.add(Reflex::new("suppressible", Stimulus::Temperature(50.0), 0.5, Response::Stop, ReflexPriority::Normal));
        sys.add(Reflex::new("survival", Stimulus::Temperature(50.0), 0.5, Response::Halt, ReflexPriority::Survival));
        // During deliberation, only unsuppressible fires
        let resp = sys.process(Stimulus::Temperature(80.0), true);
        assert!(resp.is_some());
    }

    #[test]
    fn test_enable_disable() {
        let mut sys = ReflexSystem::new();
        sys.add(Reflex::new("test", Stimulus::Temperature(50.0), 0.5, Response::Stop, ReflexPriority::High));
        sys.set_enabled("test", false);
        let resp = sys.process(Stimulus::Temperature(80.0), false);
        assert!(resp.is_none());
    }

    #[test]
    fn test_stats() {
        let mut sys = ReflexSystem::new();
        sys.add_defaults();
        let stats = sys.stats();
        assert_eq!(stats.total_reflexes, 8);
        assert_eq!(stats.active, 8);
    }

    #[test]
    fn test_no_match_different_type() {
        let r = Reflex::new("temp", Stimulus::Temperature(50.0), 0.5, Response::Stop, ReflexPriority::Normal);
        assert!(!r.matches(&Stimulus::Proximity(0.1))); // different stimulus type
    }

    #[test]
    fn test_custom_stimulus() {
        let r = Reflex::new("custom", Stimulus::Custom("alert".into()), 0.5, Response::Alert, ReflexPriority::High);
        assert!(r.matches(&Stimulus::Custom("alert".into())));
        assert!(!r.matches(&Stimulus::Custom("other".into())));
    }
}
