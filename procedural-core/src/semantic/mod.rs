use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SemanticPlugin;

impl Plugin for SemanticPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SemanticEffectEvent>().add_systems(
            Update,
            (compute_semantic_interactions, apply_semantic_effects).chain(),
        );
    }
}

/// Semantic tags attached to every game entity.
/// Example: fire monster has tags [("fire", 0.8), ("aggression", 0.9), ("corruption", 0.3)]
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct SemanticTags {
    pub tags: Vec<(String, f32)>,
}

impl SemanticTags {
    pub fn new(tags: Vec<(&str, f32)>) -> Self {
        Self {
            tags: tags.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    /// Get tag value by name, returns 0.0 if not found
    pub fn get(&self, name: &str) -> f32 {
        self.tags
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }

    /// Cosine similarity between two tag vectors
    pub fn similarity(&self, other: &SemanticTags) -> f32 {
        let mut dot = 0.0_f32;
        let mut mag_a = 0.0_f32;
        let mut mag_b = 0.0_f32;

        for (key, val_a) in &self.tags {
            mag_a += val_a * val_a;
            let val_b = other.get(key);
            dot += val_a * val_b;
        }

        for (_, val_b) in &other.tags {
            mag_b += val_b * val_b;
        }

        let magnitude = mag_a.sqrt() * mag_b.sqrt();
        if magnitude < f32::EPSILON {
            return 0.0;
        }

        dot / magnitude
    }

    /// Classify interaction based on similarity threshold
    pub fn interaction_with(&self, other: &SemanticTags) -> SemanticInteraction {
        let sim = self.similarity(other);
        if sim > 0.7 {
            SemanticInteraction::Synergy(sim)
        } else if sim < 0.3 {
            SemanticInteraction::Conflict(sim)
        } else {
            SemanticInteraction::Neutral
        }
    }

    /// Merge another tag set into this one (for environmental blending)
    pub fn blend(&mut self, other: &SemanticTags, weight: f32) {
        for (key, val) in &other.tags {
            let current = self.get(key);
            let blended = current + (val - current) * weight;
            if let Some(entry) = self.tags.iter_mut().find(|(k, _)| k == key) {
                entry.1 = blended;
            } else {
                self.tags.push((key.clone(), val * weight));
            }
        }
    }

    /// Get the dominant tag (highest value)
    pub fn dominant(&self) -> Option<(&str, f32)> {
        self.tags
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.as_str(), *v))
    }
}

/// Semantic interaction result between two entities
#[derive(Debug, Clone)]
pub enum SemanticInteraction {
    Synergy(f32),  // similarity > 0.7
    Neutral,       // 0.3..0.7
    Conflict(f32), // similarity < 0.3
}

/// Semantic effect event — triggered when entities with semantic tags interact
#[derive(Event, Debug)]
pub struct SemanticEffectEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub interaction: SemanticInteractionType,
    pub strength: f32,
}

#[derive(Debug, Clone)]
pub enum SemanticInteractionType {
    /// Fire + Fire = amplified fire damage
    ElementalResonance { element: String },
    /// Fire + Water = steam/neutralization
    ElementalConflict {
        element_a: String,
        element_b: String,
    },
    /// High corruption near low corruption = corruption spread
    CorruptionSpread { amount: f32 },
    /// Healing + Damage = reduced effect
    HealingInterference { reduction: f32 },
}

/// Maximum distance for semantic interactions
const INTERACTION_RANGE: f32 = 10.0;

fn compute_semantic_interactions(
    query: Query<(Entity, &SemanticTags, &Transform)>,
    mut effects: EventWriter<SemanticEffectEvent>,
) {
    let entities: Vec<_> = query.iter().collect();

    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (entity_a, tags_a, transform_a) = entities[i];
            let (entity_b, tags_b, transform_b) = entities[j];

            let distance = transform_a.translation.distance(transform_b.translation);
            if distance > INTERACTION_RANGE {
                continue;
            }

            let similarity = tags_a.similarity(tags_b);
            let proximity_factor = 1.0 - (distance / INTERACTION_RANGE);

            // Check for elemental resonance (same dominant element, high similarity)
            if similarity > 0.7 {
                if let Some((dom_a, _)) = tags_a.dominant() {
                    if let Some((dom_b, _)) = tags_b.dominant() {
                        if dom_a == dom_b {
                            effects.send(SemanticEffectEvent {
                                entity_a,
                                entity_b,
                                interaction: SemanticInteractionType::ElementalResonance {
                                    element: dom_a.to_string(),
                                },
                                strength: similarity * proximity_factor,
                            });
                        }
                    }
                }
            }

            // Check for elemental conflict (fire vs water, etc.)
            if similarity < 0.3 {
                let fire_a = tags_a.get("fire");
                let water_a = tags_a.get("water");
                let fire_b = tags_b.get("fire");
                let water_b = tags_b.get("water");

                if (fire_a > 0.5 && water_b > 0.5) || (water_a > 0.5 && fire_b > 0.5) {
                    effects.send(SemanticEffectEvent {
                        entity_a,
                        entity_b,
                        interaction: SemanticInteractionType::ElementalConflict {
                            element_a: "fire".into(),
                            element_b: "water".into(),
                        },
                        strength: proximity_factor,
                    });
                }
            }

            // Corruption spread
            let corruption_a = tags_a.get("corruption");
            let corruption_b = tags_b.get("corruption");
            if (corruption_a - corruption_b).abs() > 0.3 {
                let spread_amount = (corruption_a - corruption_b).abs() * proximity_factor * 0.01;
                effects.send(SemanticEffectEvent {
                    entity_a,
                    entity_b,
                    interaction: SemanticInteractionType::CorruptionSpread {
                        amount: spread_amount,
                    },
                    strength: proximity_factor,
                });
            }
        }
    }
}

fn apply_semantic_effects(
    mut events: EventReader<SemanticEffectEvent>,
    mut tags_query: Query<&mut SemanticTags>,
) {
    for event in events.read() {
        match &event.interaction {
            SemanticInteractionType::CorruptionSpread { amount } => {
                // Slowly spread corruption from high to low
                let corruption_vals = {
                    let a = tags_query
                        .get(event.entity_a)
                        .map(|t| t.get("corruption"))
                        .unwrap_or(0.0);
                    let b = tags_query
                        .get(event.entity_b)
                        .map(|t| t.get("corruption"))
                        .unwrap_or(0.0);
                    (a, b)
                };

                if corruption_vals.0 > corruption_vals.1 {
                    if let Ok(mut tags) = tags_query.get_mut(event.entity_b) {
                        let current = tags.get("corruption");
                        if let Some(entry) = tags.tags.iter_mut().find(|(k, _)| k == "corruption") {
                            entry.1 = (current + amount).min(1.0);
                        } else {
                            tags.tags.push(("corruption".into(), *amount));
                        }
                    }
                }
            }
            _ => {
                // Other effects are handled by specific systems (combat, etc.)
                trace!(?event, "Semantic effect dispatched");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_identical() {
        let a = SemanticTags::new(vec![("fire", 0.8), ("water", 0.2)]);
        let b = SemanticTags::new(vec![("fire", 0.8), ("water", 0.2)]);
        let sim = a.similarity(&b);
        assert!(
            (sim - 1.0).abs() < 0.01,
            "Identical tags should have similarity ~1.0, got {sim}"
        );
    }

    #[test]
    fn test_similarity_orthogonal() {
        let a = SemanticTags::new(vec![("fire", 1.0)]);
        let b = SemanticTags::new(vec![("water", 1.0)]);
        let sim = a.similarity(&b);
        assert!(
            sim.abs() < 0.01,
            "Orthogonal tags should have similarity ~0.0, got {sim}"
        );
    }

    #[test]
    fn test_get_tag() {
        let tags = SemanticTags::new(vec![("fire", 0.7), ("corruption", 0.3)]);
        assert!((tags.get("fire") - 0.7).abs() < f32::EPSILON);
        assert!((tags.get("missing") - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_interaction_classification() {
        let fire_a = SemanticTags::new(vec![("fire", 0.9), ("aggression", 0.8)]);
        let fire_b = SemanticTags::new(vec![("fire", 0.85), ("aggression", 0.7)]);
        let water = SemanticTags::new(vec![("water", 0.9), ("healing", 0.7)]);

        match fire_a.interaction_with(&fire_b) {
            SemanticInteraction::Synergy(_) => {} // expected
            other => panic!("Expected Synergy, got {:?}", other),
        }

        match fire_a.interaction_with(&water) {
            SemanticInteraction::Conflict(_) => {} // expected
            other => panic!("Expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn test_blend() {
        let mut a = SemanticTags::new(vec![("fire", 0.8)]);
        let b = SemanticTags::new(vec![("fire", 0.2), ("water", 0.6)]);

        a.blend(&b, 0.5);

        // fire: 0.8 + (0.2 - 0.8) * 0.5 = 0.8 - 0.3 = 0.5
        assert!((a.get("fire") - 0.5).abs() < 0.01);
        // water: 0.0 + 0.6 * 0.5 = 0.3
        assert!((a.get("water") - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_dominant() {
        let tags = SemanticTags::new(vec![("fire", 0.3), ("water", 0.9), ("earth", 0.1)]);
        let (name, val) = tags.dominant().unwrap();
        assert_eq!(name, "water");
        assert!((val - 0.9).abs() < f32::EPSILON);
    }
}
