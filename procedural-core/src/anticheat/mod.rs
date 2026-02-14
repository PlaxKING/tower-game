//! Anti-Cheat Pattern Detection
//!
//! Analyzes player behavior windows to detect:
//! - Speed hacks (impossible movement deltas)
//! - Damage hacks (exceeding theoretical maximum)
//! - Bot patterns (inhuman input regularity)
//! - Exploit abuse (repeated impossible actions)
//!
//! From opensourcestack.txt Category 11:
//! "petgraph for behavior analysis through windowed functions"

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Types of suspicious behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationType {
    SpeedHack,         // moving faster than possible
    DamageHack,        // dealing more damage than theoretical max
    TeleportSuspicion, // position jump without valid reason
    BotPattern,        // inhuman input regularity
    ExploitAbuse,      // repeated impossible actions
    ResourceHack,      // gaining resources impossibly fast
    TimingAnomaly,     // actions faster than humanly possible
}

/// Severity of a violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,      // could be lag/desync
    Medium,   // suspicious but not conclusive
    High,     // very likely cheating
    Critical, // definitely cheating
}

/// A recorded violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub timestamp: u64,
    pub details: String,
    pub value: f32,     // measured value
    pub threshold: f32, // expected maximum
}

/// Player action for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAction {
    pub timestamp_ms: u64,
    pub action_type: ActionType,
    pub position: [f32; 3],
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Move,
    Attack,
    Dodge,
    Interact,
    PickupLoot,
    CraftItem,
    UseAbility,
}

/// Penalty recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenaltyAction {
    None,
    Warning,       // send warning to player
    SoftThrottle,  // reduce speed/damage slightly
    ShadowPenalty, // "dead drops" — loot is worthless
    TempBan,       // temporary match ban
    FlagForReview, // human review needed
}

/// Per-player behavior analyzer
#[derive(Debug, Clone)]
pub struct PlayerAnalyzer {
    pub player_id: String,
    pub action_history: VecDeque<PlayerAction>,
    pub violations: Vec<Violation>,
    pub trust_score: f32, // 0.0 (banned) - 1.0 (trusted)
    pub window_size: usize,
    // Thresholds
    pub max_speed: f32,
    pub max_damage_per_hit: f32,
    pub min_action_interval_ms: u64,
    pub teleport_threshold: f32,
}

impl PlayerAnalyzer {
    pub fn new(player_id: &str) -> Self {
        Self {
            player_id: player_id.to_string(),
            action_history: VecDeque::new(),
            violations: Vec::new(),
            trust_score: 1.0,
            window_size: 100,
            // Game-tuned thresholds
            max_speed: 20.0, // units per tick (matches Nakama match handler)
            max_damage_per_hit: 200.0, // matches Nakama damage cap
            min_action_interval_ms: 50, // ~20 actions/sec max
            teleport_threshold: 500.0, // units (matches RemotePlayer teleport threshold)
        }
    }

    /// Record a player action and check for violations
    pub fn record_action(&mut self, action: PlayerAction) -> Vec<Violation> {
        let mut new_violations = Vec::new();

        // Check against recent history
        if let Some(prev) = self.action_history.back() {
            // Speed check
            if action.action_type == ActionType::Move {
                let dx = action.position[0] - prev.position[0];
                let dy = action.position[1] - prev.position[1];
                let dz = action.position[2] - prev.position[2];
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                let dt_ms = action.timestamp_ms.saturating_sub(prev.timestamp_ms);

                if dt_ms > 0 {
                    let speed = distance / (dt_ms as f32 / 1000.0);

                    if distance > self.teleport_threshold {
                        new_violations.push(Violation {
                            violation_type: ViolationType::TeleportSuspicion,
                            severity: ViolationSeverity::Medium,
                            timestamp: action.timestamp_ms,
                            details: format!("Teleport: {:.1} units in {}ms", distance, dt_ms),
                            value: distance,
                            threshold: self.teleport_threshold,
                        });
                    } else if speed > self.max_speed * 3.0 {
                        new_violations.push(Violation {
                            violation_type: ViolationType::SpeedHack,
                            severity: ViolationSeverity::High,
                            timestamp: action.timestamp_ms,
                            details: format!("Speed: {:.1} (max: {:.1})", speed, self.max_speed),
                            value: speed,
                            threshold: self.max_speed,
                        });
                    } else if speed > self.max_speed * 1.5 {
                        new_violations.push(Violation {
                            violation_type: ViolationType::SpeedHack,
                            severity: ViolationSeverity::Low,
                            timestamp: action.timestamp_ms,
                            details: format!("Elevated speed: {:.1}", speed),
                            value: speed,
                            threshold: self.max_speed,
                        });
                    }
                }
            }

            // Damage check
            if action.action_type == ActionType::Attack && action.value > self.max_damage_per_hit {
                let severity = if action.value > self.max_damage_per_hit * 2.0 {
                    ViolationSeverity::Critical
                } else {
                    ViolationSeverity::High
                };
                new_violations.push(Violation {
                    violation_type: ViolationType::DamageHack,
                    severity,
                    timestamp: action.timestamp_ms,
                    details: format!(
                        "Damage: {:.1} (max: {:.1})",
                        action.value, self.max_damage_per_hit
                    ),
                    value: action.value,
                    threshold: self.max_damage_per_hit,
                });
            }

            // Timing check (inhuman speed)
            let interval = action.timestamp_ms.saturating_sub(prev.timestamp_ms);
            if interval > 0
                && interval < self.min_action_interval_ms
                && action.action_type == prev.action_type
            {
                new_violations.push(Violation {
                    violation_type: ViolationType::TimingAnomaly,
                    severity: ViolationSeverity::Medium,
                    timestamp: action.timestamp_ms,
                    details: format!(
                        "Action interval: {}ms (min: {}ms)",
                        interval, self.min_action_interval_ms
                    ),
                    value: interval as f32,
                    threshold: self.min_action_interval_ms as f32,
                });
            }
        }

        // Bot pattern detection (check regularity over window)
        if self.action_history.len() >= 10 {
            if let Some(bot_violation) = self.check_bot_pattern() {
                new_violations.push(bot_violation);
            }
        }

        // Update trust score
        for v in &new_violations {
            let penalty = match v.severity {
                ViolationSeverity::Low => 0.02,
                ViolationSeverity::Medium => 0.05,
                ViolationSeverity::High => 0.15,
                ViolationSeverity::Critical => 0.30,
            };
            self.trust_score = (self.trust_score - penalty).max(0.0);
        }

        // Store
        self.violations.extend(new_violations.clone());
        self.action_history.push_back(action);
        while self.action_history.len() > self.window_size {
            self.action_history.pop_front();
        }

        new_violations
    }

    /// Detect bot-like input regularity
    fn check_bot_pattern(&self) -> Option<Violation> {
        if self.action_history.len() < 10 {
            return None;
        }

        // Calculate inter-action timing variance
        let intervals: Vec<f64> = self
            .action_history
            .iter()
            .zip(self.action_history.iter().skip(1))
            .map(|(a, b)| (b.timestamp_ms - a.timestamp_ms) as f64)
            .collect();

        if intervals.is_empty() {
            return None;
        }

        let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
        if avg < 1.0 {
            return None;
        }

        let variance =
            intervals.iter().map(|i| (i - avg).powi(2)).sum::<f64>() / intervals.len() as f64;
        let cv = variance.sqrt() / avg; // coefficient of variation

        // Humans have CV > 0.15 typically; bots have CV < 0.05
        if cv < 0.03 {
            Some(Violation {
                violation_type: ViolationType::BotPattern,
                severity: ViolationSeverity::High,
                timestamp: self
                    .action_history
                    .back()
                    .map(|a| a.timestamp_ms)
                    .unwrap_or(0),
                details: format!("Input regularity CV: {:.4} (bot threshold: 0.03)", cv),
                value: cv as f32,
                threshold: 0.03,
            })
        } else if cv < 0.08 {
            Some(Violation {
                violation_type: ViolationType::BotPattern,
                severity: ViolationSeverity::Low,
                timestamp: self
                    .action_history
                    .back()
                    .map(|a| a.timestamp_ms)
                    .unwrap_or(0),
                details: format!("Suspiciously regular input CV: {:.4}", cv),
                value: cv as f32,
                threshold: 0.08,
            })
        } else {
            None
        }
    }

    /// Get recommended penalty based on violation history
    pub fn recommended_penalty(&self) -> PenaltyAction {
        if self.trust_score <= 0.0 {
            PenaltyAction::TempBan
        } else if self.trust_score < 0.3 {
            PenaltyAction::ShadowPenalty
        } else if self.trust_score < 0.5 {
            PenaltyAction::SoftThrottle
        } else if self.trust_score < 0.7 {
            PenaltyAction::Warning
        } else if self
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical)
        {
            PenaltyAction::FlagForReview
        } else {
            PenaltyAction::None
        }
    }

    /// Slowly recover trust over time (called periodically)
    pub fn recover_trust(&mut self, amount: f32) {
        self.trust_score = (self.trust_score + amount).min(1.0);
    }

    /// Count violations of a specific type
    pub fn violation_count(&self, vtype: ViolationType) -> usize {
        self.violations
            .iter()
            .filter(|v| v.violation_type == vtype)
            .count()
    }

    /// Recent violations (last N)
    pub fn recent_violations(&self, count: usize) -> &[Violation] {
        let start = self.violations.len().saturating_sub(count);
        &self.violations[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn move_action(ts: u64, x: f32, z: f32) -> PlayerAction {
        PlayerAction {
            timestamp_ms: ts,
            action_type: ActionType::Move,
            position: [x, 0.0, z],
            value: 0.0,
        }
    }

    fn attack_action(ts: u64, damage: f32) -> PlayerAction {
        PlayerAction {
            timestamp_ms: ts,
            action_type: ActionType::Attack,
            position: [0.0, 0.0, 0.0],
            value: damage,
        }
    }

    #[test]
    fn test_normal_movement_no_violations() {
        let mut analyzer = PlayerAnalyzer::new("player1");
        let v1 = analyzer.record_action(move_action(0, 0.0, 0.0));
        let v2 = analyzer.record_action(move_action(100, 1.0, 1.0)); // ~14.1 units/sec
        assert!(v1.is_empty());
        assert!(v2.is_empty());
    }

    #[test]
    fn test_speed_hack_detection() {
        let mut analyzer = PlayerAnalyzer::new("cheater1");
        analyzer.record_action(move_action(0, 0.0, 0.0));
        let violations = analyzer.record_action(move_action(100, 100.0, 100.0)); // 1414 units/sec
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::SpeedHack));
    }

    #[test]
    fn test_teleport_detection() {
        let mut analyzer = PlayerAnalyzer::new("player2");
        analyzer.record_action(move_action(0, 0.0, 0.0));
        let violations = analyzer.record_action(move_action(1000, 600.0, 0.0)); // 600 units jump
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::TeleportSuspicion));
    }

    #[test]
    fn test_damage_hack_detection() {
        let mut analyzer = PlayerAnalyzer::new("cheater2");
        analyzer.record_action(attack_action(0, 50.0)); // normal
        let violations = analyzer.record_action(attack_action(500, 500.0)); // way over cap
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::DamageHack));
    }

    #[test]
    fn test_damage_hack_critical() {
        let mut analyzer = PlayerAnalyzer::new("cheater3");
        analyzer.record_action(attack_action(0, 50.0));
        let violations = analyzer.record_action(attack_action(500, 999.0)); // 5x over cap
        let critical = violations
            .iter()
            .find(|v| v.violation_type == ViolationType::DamageHack);
        assert!(critical.is_some());
        assert_eq!(critical.unwrap().severity, ViolationSeverity::Critical);
    }

    #[test]
    fn test_timing_anomaly() {
        let mut analyzer = PlayerAnalyzer::new("bot1");
        analyzer.record_action(attack_action(0, 50.0));
        let violations = analyzer.record_action(attack_action(10, 50.0)); // 10ms between attacks
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::TimingAnomaly));
    }

    #[test]
    fn test_bot_pattern_detection() {
        let mut analyzer = PlayerAnalyzer::new("bot2");
        // Generate perfectly regular inputs (bot-like)
        for i in 0..20 {
            let ts = i * 100; // exactly 100ms apart
            analyzer.record_action(move_action(ts, i as f32, 0.0));
        }
        let bot_violations = analyzer
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::BotPattern)
            .count();
        assert!(
            bot_violations > 0,
            "Perfectly regular input should trigger bot detection"
        );
    }

    #[test]
    fn test_trust_score_degradation() {
        let mut analyzer = PlayerAnalyzer::new("cheater4");
        assert!((analyzer.trust_score - 1.0).abs() < f32::EPSILON);

        analyzer.record_action(move_action(0, 0.0, 0.0));
        analyzer.record_action(move_action(100, 100.0, 100.0)); // speed hack
        assert!(
            analyzer.trust_score < 1.0,
            "Trust should decrease after violation"
        );
    }

    #[test]
    fn test_trust_recovery() {
        let mut analyzer = PlayerAnalyzer::new("player3");
        analyzer.trust_score = 0.5;
        analyzer.recover_trust(0.1);
        assert!((analyzer.trust_score - 0.6).abs() < 0.01);

        analyzer.recover_trust(1.0);
        assert!(
            (analyzer.trust_score - 1.0).abs() < f32::EPSILON,
            "Trust shouldn't exceed 1.0"
        );
    }

    #[test]
    fn test_penalty_recommendations() {
        let mut analyzer = PlayerAnalyzer::new("player4");
        assert_eq!(analyzer.recommended_penalty(), PenaltyAction::None);

        analyzer.trust_score = 0.6;
        assert_eq!(analyzer.recommended_penalty(), PenaltyAction::Warning);

        analyzer.trust_score = 0.4;
        assert_eq!(analyzer.recommended_penalty(), PenaltyAction::SoftThrottle);

        analyzer.trust_score = 0.2;
        assert_eq!(analyzer.recommended_penalty(), PenaltyAction::ShadowPenalty);

        analyzer.trust_score = 0.0;
        assert_eq!(analyzer.recommended_penalty(), PenaltyAction::TempBan);
    }

    #[test]
    fn test_violation_count() {
        let mut analyzer = PlayerAnalyzer::new("cheater5");
        analyzer.record_action(move_action(0, 0.0, 0.0));
        analyzer.record_action(move_action(50, 100.0, 0.0));
        analyzer.record_action(move_action(100, 200.0, 0.0));
        assert!(analyzer.violation_count(ViolationType::SpeedHack) > 0);
    }

    #[test]
    fn test_window_size_limiting() {
        let mut analyzer = PlayerAnalyzer::new("player5");
        analyzer.window_size = 10;
        for i in 0..20 {
            analyzer.record_action(move_action(i * 200, i as f32, 0.0));
        }
        assert!(
            analyzer.action_history.len() <= 10,
            "History should be capped at window_size"
        );
    }

    #[test]
    fn test_human_like_input_no_bot() {
        let mut analyzer = PlayerAnalyzer::new("human1");
        // Generate human-like irregular inputs
        let intervals = [
            95, 112, 87, 150, 103, 78, 200, 133, 91, 167, 88, 145, 99, 122, 76, 189, 108, 94, 156,
            113,
        ];
        let mut ts = 0u64;
        for interval in &intervals {
            ts += interval;
            analyzer.record_action(move_action(ts, (ts / 100) as f32, 0.0));
        }
        let bot_violations = analyzer
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::BotPattern)
            .count();
        assert_eq!(
            bot_violations, 0,
            "Human-like input should not trigger bot detection"
        );
    }
}
