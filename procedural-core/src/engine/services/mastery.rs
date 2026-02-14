use std::collections::HashMap;

use crate::engine::config::EngineConfig;
use crate::engine::helpers::tier_to_u32;
use crate::engine::messages::{
    DomainProfileMsg, MasteryProfileMsg, MasteryProgressResultMsg, SpecInfoMsg, SynergyInfoMsg,
};
use crate::mastery::{MasteryDomain, MasteryProfile};
use crate::specialization::SpecializationProfile;

/// MasteryService — skill mastery and progression tracking
pub struct MasteryService {
    pub(crate) profiles: HashMap<u64, MasteryProfile>,
    pub(crate) spec_profiles: HashMap<u64, SpecializationProfile>,
}

impl MasteryService {
    pub fn new(_config: &EngineConfig) -> Self {
        Self {
            profiles: HashMap::new(),
            spec_profiles: HashMap::new(),
        }
    }

    pub fn track_progress(
        &mut self,
        player_id: u64,
        domain: &str,
        xp_amount: f32,
    ) -> MasteryProgressResultMsg {
        let profile = self.profiles.entry(player_id).or_default();

        let mastery_domain = match domain {
            "sword" => MasteryDomain::SwordMastery,
            "greatsword" => MasteryDomain::GreatswordMastery,
            "dual_daggers" | "dagger" => MasteryDomain::DaggerMastery,
            "spear" => MasteryDomain::SpearMastery,
            "gauntlets" | "gauntlet" => MasteryDomain::GauntletMastery,
            "staff" => MasteryDomain::StaffMastery,
            "dodge" => MasteryDomain::DodgeMastery,
            "parry" => MasteryDomain::ParryMastery,
            "block" => MasteryDomain::BlockMastery,
            "aerial" => MasteryDomain::AerialMastery,
            "alchemy" => MasteryDomain::Alchemy,
            "smithing" | "blacksmithing" => MasteryDomain::Blacksmithing,
            "enchanting" => MasteryDomain::Enchanting,
            "tailoring" => MasteryDomain::Tailoring,
            "cooking" => MasteryDomain::Cooking,
            "trading" => MasteryDomain::Trading,
            "exploration" => MasteryDomain::Exploration,
            "mining" => MasteryDomain::Mining,
            "herbalism" => MasteryDomain::Herbalism,
            "salvaging" => MasteryDomain::Salvaging,
            "semantic" => MasteryDomain::SemanticAttunement,
            _ => {
                return MasteryProgressResultMsg {
                    domain: domain.into(),
                    new_tier: 0,
                    new_xp: 0.0,
                    xp_to_next: 0.0,
                    tier_up: false,
                    newly_unlocked: vec![],
                }
            }
        };

        let _old_tier = profile.tier(mastery_domain);
        let tier_up = profile.gain_xp(mastery_domain, xp_amount as u64);
        let new_tier = profile.tier(mastery_domain);

        let (xp_current, xp_required) =
            if let Some(progress) = profile.masteries.get(&mastery_domain) {
                let next_threshold = new_tier
                    .next_tier()
                    .map(|t| t.xp_threshold())
                    .unwrap_or(progress.xp);
                (progress.xp as f32, next_threshold as f32)
            } else {
                (0.0, 100.0)
            };

        MasteryProgressResultMsg {
            domain: domain.into(),
            new_tier: tier_to_u32(new_tier),
            new_xp: xp_current,
            xp_to_next: xp_required,
            tier_up,
            newly_unlocked: if tier_up {
                vec![format!("Tier {:?} unlocked!", new_tier)]
            } else {
                vec![]
            },
        }
    }

    pub fn get_mastery_profile(&self, player_id: u64) -> MasteryProfileMsg {
        let default_profile = MasteryProfile::new();
        let profile = self.profiles.get(&player_id).unwrap_or(&default_profile);

        let all_domains = [
            MasteryDomain::SwordMastery,
            MasteryDomain::GreatswordMastery,
            MasteryDomain::DaggerMastery,
            MasteryDomain::SpearMastery,
            MasteryDomain::GauntletMastery,
            MasteryDomain::StaffMastery,
            MasteryDomain::ParryMastery,
            MasteryDomain::DodgeMastery,
            MasteryDomain::BlockMastery,
            MasteryDomain::AerialMastery,
            MasteryDomain::Blacksmithing,
            MasteryDomain::Alchemy,
            MasteryDomain::Enchanting,
            MasteryDomain::Tailoring,
            MasteryDomain::Cooking,
            MasteryDomain::Mining,
            MasteryDomain::Herbalism,
            MasteryDomain::Salvaging,
            MasteryDomain::Trading,
            MasteryDomain::Exploration,
            MasteryDomain::SemanticAttunement,
        ];

        let domains: Vec<DomainProfileMsg> = all_domains
            .iter()
            .map(|&d| {
                let tier = profile.tier(d);
                let (xp, xp_req) = if let Some(progress) = profile.masteries.get(&d) {
                    let next = tier
                        .next_tier()
                        .map(|t| t.xp_threshold())
                        .unwrap_or(progress.xp);
                    (progress.xp as f32, next as f32)
                } else {
                    (0.0, 100.0)
                };
                DomainProfileMsg {
                    domain_name: format!("{:?}", d),
                    tier: tier_to_u32(tier),
                    xp_current: xp,
                    xp_required: xp_req,
                }
            })
            .collect();

        let spec_profile = self.spec_profiles.get(&player_id);
        let specializations: Vec<SpecInfoMsg> = spec_profile
            .map(|sp| {
                sp.chosen_branches
                    .iter()
                    .map(|(domain, branch_id)| SpecInfoMsg {
                        branch_id: branch_id.clone(),
                        domain: format!("{:?}", domain),
                        combat_role: sp
                            .primary_role
                            .map(|r| format!("{:?}", r))
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let synergies: Vec<SynergyInfoMsg> = spec_profile
            .map(|sp| {
                let branch_ids: Vec<String> = sp.chosen_branches.values().cloned().collect();
                crate::specialization::find_active_synergies(&branch_ids)
                    .into_iter()
                    .map(|s| SynergyInfoMsg {
                        synergy_name: s.name.clone(),
                        required_branches: vec![s.branch_a.clone(), s.branch_b.clone()],
                        bonus_description: s.description.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let role = spec_profile
            .and_then(|sp| sp.primary_role)
            .map(|r| format!("{:?}", r))
            .unwrap_or_else(|| "None".into());

        MasteryProfileMsg {
            domains,
            specializations,
            active_synergies: synergies,
            primary_combat_role: role,
        }
    }
}
