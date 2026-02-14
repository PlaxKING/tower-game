//! Monte-Carlo Balance Simulation
//!
//! Simulates 100k+ player builds to detect dominant strategies,
//! verify weapon balance, and ensure no single build trivializes content.
//! Uses rayon for parallel execution across CPU cores.
//!
//! From opensourcestack.txt Category 12:
//! "Monte-Carlo 100k+ builds before updating grammars"

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::combat::weapons::WeaponType;

/// A simulated player build for balance testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedBuild {
    pub weapon: WeaponType,
    pub level: u32,
    pub stat_allocation: StatAllocation,
    pub playstyle: Playstyle,
    pub element_affinity: String,
}

/// Stat point allocation for a build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatAllocation {
    pub strength: f32,  // melee damage
    pub agility: f32,   // speed, dodge
    pub vitality: f32,  // hp, defense
    pub intellect: f32, // semantic power
    pub endurance: f32, // resource pools
}

impl StatAllocation {
    /// Generate from hash bits
    fn from_hash(hash: u64, total_points: f32) -> Self {
        let bits = [
            (hash & 0xFF) as f32,
            ((hash >> 8) & 0xFF) as f32,
            ((hash >> 16) & 0xFF) as f32,
            ((hash >> 24) & 0xFF) as f32,
            ((hash >> 32) & 0xFF) as f32,
        ];
        let sum: f32 = bits.iter().sum();
        let norm = total_points / sum.max(1.0);

        Self {
            strength: bits[0] * norm,
            agility: bits[1] * norm,
            vitality: bits[2] * norm,
            intellect: bits[3] * norm,
            endurance: bits[4] * norm,
        }
    }

    #[allow(dead_code)]
    fn total(&self) -> f32 {
        self.strength + self.agility + self.vitality + self.intellect + self.endurance
    }
}

/// Player behavior archetype
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Playstyle {
    Aggressive, // maximize damage, low defense
    Defensive,  // maximize survivability
    Balanced,   // even distribution
    HitAndRun,  // high agility, burst damage
    Semantic,   // intellect focus, status effects
}

impl Playstyle {
    fn from_hash(hash: u64) -> Self {
        match hash % 5 {
            0 => Self::Aggressive,
            1 => Self::Defensive,
            2 => Self::Balanced,
            3 => Self::HitAndRun,
            _ => Self::Semantic,
        }
    }
}

/// Performance metrics for a single build simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPerformance {
    pub build: SimulatedBuild,
    pub dps: f32,                 // damage per second
    pub effective_hp: f32,        // hp * mitigation
    pub clear_speed: f32,         // floors per minute estimate
    pub survivability: f32,       // 0-1 chance of surviving a floor
    pub resource_efficiency: f32, // damage per resource point
    pub composite_score: f32,     // weighted overall score
}

/// Results of a balance simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceReport {
    pub total_builds: u64,
    pub avg_score: f32,
    pub std_deviation: f32,
    pub min_score: f32,
    pub max_score: f32,
    pub score_range_ratio: f32,                 // max/min — ideally < 2.0
    pub weapon_scores: Vec<(String, f32, f32)>, // weapon, avg, std
    pub playstyle_scores: Vec<(String, f32, f32)>,
    pub dominant_builds: Vec<BuildPerformance>, // top 5
    pub weakest_builds: Vec<BuildPerformance>,  // bottom 5
    pub balance_grade: BalanceGrade,
}

/// Overall balance assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceGrade {
    Excellent, // range ratio < 1.5
    Good,      // range ratio < 2.0
    Fair,      // range ratio < 3.0
    Poor,      // range ratio < 5.0
    Critical,  // range ratio >= 5.0
}

/// Configuration for a simulation run
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub build_count: u64,
    pub floor_level: u32,
    pub base_seed: u64,
    pub stat_points: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            build_count: 10_000,
            floor_level: 10,
            base_seed: 42,
            stat_points: 50.0,
        }
    }
}

/// Generate a deterministic build from a hash
fn generate_build(hash: u64, level: u32, stat_points: f32) -> SimulatedBuild {
    let weapon = match hash % 6 {
        0 => WeaponType::Sword,
        1 => WeaponType::Greatsword,
        2 => WeaponType::DualDaggers,
        3 => WeaponType::Spear,
        4 => WeaponType::Gauntlets,
        _ => WeaponType::Staff,
    };

    let element = match (hash >> 40) % 6 {
        0 => "fire",
        1 => "water",
        2 => "earth",
        3 => "wind",
        4 => "void",
        _ => "neutral",
    };

    SimulatedBuild {
        weapon,
        level,
        stat_allocation: StatAllocation::from_hash(hash >> 8, stat_points),
        playstyle: Playstyle::from_hash(hash >> 48),
        element_affinity: element.to_string(),
    }
}

/// Simulate combat performance for a build
fn simulate_build(build: &SimulatedBuild, floor_level: u32) -> BuildPerformance {
    let stats = &build.stat_allocation;

    // Base damage from weapon type
    let weapon_base_dps = match build.weapon {
        WeaponType::Sword => 30.0,
        WeaponType::Greatsword => 45.0,
        WeaponType::DualDaggers => 25.0,
        WeaponType::Spear => 35.0,
        WeaponType::Gauntlets => 28.0,
        WeaponType::Staff => 20.0,
    };

    // Attack speed modifier
    let attack_speed = match build.weapon {
        WeaponType::Sword => 1.0,
        WeaponType::Greatsword => 0.6,
        WeaponType::DualDaggers => 1.6,
        WeaponType::Spear => 0.8,
        WeaponType::Gauntlets => 1.4,
        WeaponType::Staff => 0.7,
    };

    // DPS calculation
    let str_bonus = 1.0 + stats.strength * 0.02;
    let agi_bonus = 1.0 + stats.agility * 0.005; // minor speed boost
    let int_bonus = if build.weapon == WeaponType::Staff {
        1.0 + stats.intellect * 0.025
    } else {
        1.0
    };
    let dps = weapon_base_dps * attack_speed * str_bonus * agi_bonus * int_bonus;

    // Effective HP
    let base_hp = 100.0 + stats.vitality * 10.0;
    let armor = stats.vitality * 0.5 + stats.endurance * 0.3;
    let dodge_chance = (stats.agility * 0.005).min(0.4);
    let mitigation = 1.0 / (1.0 - dodge_chance) * (1.0 + armor * 0.01);
    let effective_hp = base_hp * mitigation;

    // Playstyle modifiers
    let (dps_mod, ehp_mod) = match build.playstyle {
        Playstyle::Aggressive => (1.15, 0.85),
        Playstyle::Defensive => (0.85, 1.2),
        Playstyle::Balanced => (1.0, 1.0),
        Playstyle::HitAndRun => (1.1, 0.95),
        Playstyle::Semantic => (0.9, 1.05),
    };

    let final_dps = dps * dps_mod;
    let final_ehp = effective_hp * ehp_mod;

    // Floor difficulty scaling
    let floor_difficulty = 1.0 + floor_level as f32 * 0.05;
    let monster_dps = 15.0 * floor_difficulty;
    let monster_hp = 200.0 * floor_difficulty;

    // Clear speed: time to kill average monster
    let time_to_kill = monster_hp / final_dps.max(1.0);
    let clear_speed = 60.0 / time_to_kill.max(1.0); // monsters per minute

    // Survivability: chance of surviving a floor's worth of damage
    let survival_time = final_ehp / monster_dps.max(1.0);
    let survivability = (survival_time / 60.0).min(1.0); // 60s = guaranteed survive

    // Resource efficiency
    let resource_pool = stats.endurance * 5.0 + 50.0;
    let resource_efficiency = final_dps / (resource_pool.max(1.0) * 0.1);

    // Composite score (weighted)
    let composite = final_dps * 0.3
        + final_ehp * 0.01 * 0.25
        + clear_speed * 0.25
        + survivability * 100.0 * 0.1
        + resource_efficiency * 0.1;

    BuildPerformance {
        build: build.clone(),
        dps: final_dps,
        effective_hp: final_ehp,
        clear_speed,
        survivability,
        resource_efficiency,
        composite_score: composite,
    }
}

/// Run Monte-Carlo balance simulation with rayon parallelism
pub fn run_balance_simulation(config: &SimConfig) -> BalanceReport {
    // Generate build hashes deterministically
    let hashes: Vec<u64> = (0..config.build_count)
        .map(|i| {
            let mut hasher = Sha3_256::new();
            hasher.update(config.base_seed.to_le_bytes());
            hasher.update(i.to_le_bytes());
            let result = hasher.finalize();
            u64::from_le_bytes(result[0..8].try_into().unwrap())
        })
        .collect();

    // Parallel simulation with rayon
    let results: Vec<BuildPerformance> = hashes
        .par_iter()
        .map(|hash| {
            let build = generate_build(*hash, config.floor_level, config.stat_points);
            simulate_build(&build, config.floor_level)
        })
        .collect();

    analyze_results(&results, config.build_count)
}

/// Analyze simulation results into a balance report
fn analyze_results(results: &[BuildPerformance], total: u64) -> BalanceReport {
    if results.is_empty() {
        return BalanceReport {
            total_builds: 0,
            avg_score: 0.0,
            std_deviation: 0.0,
            min_score: 0.0,
            max_score: 0.0,
            score_range_ratio: 1.0,
            weapon_scores: vec![],
            playstyle_scores: vec![],
            dominant_builds: vec![],
            weakest_builds: vec![],
            balance_grade: BalanceGrade::Good,
        };
    }

    let scores: Vec<f32> = results.iter().map(|r| r.composite_score).collect();
    let avg = scores.iter().sum::<f32>() / scores.len() as f32;
    let variance = scores.iter().map(|s| (s - avg).powi(2)).sum::<f32>() / scores.len() as f32;
    let std_dev = variance.sqrt();
    let min = scores.iter().cloned().fold(f32::MAX, f32::min);
    let max = scores.iter().cloned().fold(f32::MIN, f32::max);
    let range_ratio = if min > 0.001 { max / min } else { 999.0 };

    // Per-weapon analysis
    let weapons = [
        WeaponType::Sword,
        WeaponType::Greatsword,
        WeaponType::DualDaggers,
        WeaponType::Spear,
        WeaponType::Gauntlets,
        WeaponType::Staff,
    ];
    let weapon_scores: Vec<(String, f32, f32)> = weapons
        .iter()
        .map(|w| {
            let weapon_results: Vec<f32> = results
                .iter()
                .filter(|r| r.build.weapon == *w)
                .map(|r| r.composite_score)
                .collect();
            let w_avg = if weapon_results.is_empty() {
                0.0
            } else {
                weapon_results.iter().sum::<f32>() / weapon_results.len() as f32
            };
            let w_var = if weapon_results.is_empty() {
                0.0
            } else {
                weapon_results
                    .iter()
                    .map(|s| (s - w_avg).powi(2))
                    .sum::<f32>()
                    / weapon_results.len() as f32
            };
            (format!("{:?}", w), w_avg, w_var.sqrt())
        })
        .collect();

    // Per-playstyle analysis
    let styles = [
        Playstyle::Aggressive,
        Playstyle::Defensive,
        Playstyle::Balanced,
        Playstyle::HitAndRun,
        Playstyle::Semantic,
    ];
    let playstyle_scores: Vec<(String, f32, f32)> = styles
        .iter()
        .map(|s| {
            let style_results: Vec<f32> = results
                .iter()
                .filter(|r| r.build.playstyle == *s)
                .map(|r| r.composite_score)
                .collect();
            let s_avg = if style_results.is_empty() {
                0.0
            } else {
                style_results.iter().sum::<f32>() / style_results.len() as f32
            };
            let s_var = if style_results.is_empty() {
                0.0
            } else {
                style_results
                    .iter()
                    .map(|x| (x - s_avg).powi(2))
                    .sum::<f32>()
                    / style_results.len() as f32
            };
            (format!("{:?}", s), s_avg, s_var.sqrt())
        })
        .collect();

    // Top 5 and bottom 5
    let mut sorted = results.to_vec();
    sorted.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());
    let dominant = sorted.iter().take(5).cloned().collect();
    let weakest = sorted.iter().rev().take(5).cloned().collect();

    let grade = if range_ratio < 1.5 {
        BalanceGrade::Excellent
    } else if range_ratio < 2.0 {
        BalanceGrade::Good
    } else if range_ratio < 3.0 {
        BalanceGrade::Fair
    } else if range_ratio < 5.0 {
        BalanceGrade::Poor
    } else {
        BalanceGrade::Critical
    };

    BalanceReport {
        total_builds: total,
        avg_score: avg,
        std_deviation: std_dev,
        min_score: min,
        max_score: max,
        score_range_ratio: range_ratio,
        weapon_scores,
        playstyle_scores,
        dominant_builds: dominant,
        weakest_builds: weakest,
        balance_grade: grade,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_sim_small() {
        let config = SimConfig {
            build_count: 100,
            floor_level: 10,
            base_seed: 42,
            stat_points: 50.0,
        };
        let report = run_balance_simulation(&config);
        assert_eq!(report.total_builds, 100);
        assert!(report.avg_score > 0.0);
        assert!(report.max_score >= report.min_score);
        assert_eq!(report.dominant_builds.len(), 5);
        assert_eq!(report.weakest_builds.len(), 5);
    }

    #[test]
    fn test_balance_sim_1k() {
        let config = SimConfig {
            build_count: 1_000,
            floor_level: 20,
            base_seed: 12345,
            stat_points: 80.0,
        };
        let report = run_balance_simulation(&config);
        assert_eq!(report.total_builds, 1_000);
        assert!(
            report.std_deviation > 0.0,
            "Should have variance across builds"
        );
    }

    #[test]
    fn test_all_weapons_represented() {
        let config = SimConfig {
            build_count: 600,
            ..Default::default()
        };
        let report = run_balance_simulation(&config);
        assert_eq!(
            report.weapon_scores.len(),
            6,
            "All 6 weapons should be represented"
        );
        for (name, avg, _) in &report.weapon_scores {
            assert!(avg > &0.0, "Weapon {} should have positive score", name);
        }
    }

    #[test]
    fn test_all_playstyles_represented() {
        let config = SimConfig {
            build_count: 500,
            ..Default::default()
        };
        let report = run_balance_simulation(&config);
        assert_eq!(
            report.playstyle_scores.len(),
            5,
            "All 5 playstyles should be represented"
        );
    }

    #[test]
    fn test_deterministic_results() {
        let config = SimConfig {
            build_count: 50,
            ..Default::default()
        };
        let r1 = run_balance_simulation(&config);
        let r2 = run_balance_simulation(&config);
        assert!(
            (r1.avg_score - r2.avg_score).abs() < 0.001,
            "Same seed should give same results"
        );
    }

    #[test]
    fn test_balance_grade() {
        let config = SimConfig {
            build_count: 500,
            ..Default::default()
        };
        let report = run_balance_simulation(&config);
        // Random builds will have wide variance; verify grading works
        assert!(
            report.balance_grade != BalanceGrade::Critical,
            "Random builds shouldn't be Critical, got ratio: {:.2}",
            report.score_range_ratio
        );
    }

    #[test]
    fn test_stat_allocation_from_hash() {
        let stats = StatAllocation::from_hash(12345, 50.0);
        let total = stats.total();
        assert!(
            (total - 50.0).abs() < 0.1,
            "Stats should sum to target: got {}",
            total
        );
    }

    #[test]
    fn test_simulate_build_produces_valid_metrics() {
        let build = SimulatedBuild {
            weapon: WeaponType::Sword,
            level: 10,
            stat_allocation: StatAllocation::from_hash(42, 50.0),
            playstyle: Playstyle::Balanced,
            element_affinity: "fire".into(),
        };
        let perf = simulate_build(&build, 10);
        assert!(perf.dps > 0.0);
        assert!(perf.effective_hp > 0.0);
        assert!(perf.clear_speed > 0.0);
        assert!(perf.survivability >= 0.0 && perf.survivability <= 1.0);
        assert!(perf.composite_score > 0.0);
    }

    #[test]
    fn test_floor_level_affects_difficulty() {
        let build = SimulatedBuild {
            weapon: WeaponType::Sword,
            level: 10,
            stat_allocation: StatAllocation::from_hash(42, 50.0),
            playstyle: Playstyle::Balanced,
            element_affinity: "fire".into(),
        };
        let perf_low = simulate_build(&build, 1);
        let perf_high = simulate_build(&build, 50);
        assert!(
            perf_low.survivability > perf_high.survivability,
            "Higher floors should be harder to survive"
        );
    }

    #[test]
    fn test_report_serialization() {
        let config = SimConfig {
            build_count: 10,
            ..Default::default()
        };
        let report = run_balance_simulation(&config);
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.is_empty());
        let restored: BalanceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_builds, 10);
    }
}
