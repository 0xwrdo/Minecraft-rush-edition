//! Brewing stand state machine, potion recipe tree, modifiers (Redstone extension,
//! Glowstone amplification, Gunpowder splash), and status effects with durations.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::item::{ItemId, ItemStack};
use crate::survival::SurvivalVitals;
use serde::{Deserialize, Serialize};

pub const BREWING_TIME_TICKS: u16 = 400; // 20 seconds at 20 TPS

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotionType {
    Water,
    Awkward,
    Thick,
    Mundane,
    Swiftness,
    Slowness,
    Strength,
    Weakness,
    Healing,
    Harming,
    Regeneration,
    FireResistance,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectType {
    None,
    Speed,
    Slowness,
    Strength,
    Weakness,
    InstantHealth,
    InstantDamage,
    Regeneration,
    FireResistance,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionMetadata {
    pub potion_type: PotionType,
    pub is_extended: bool,
    pub is_amplified: bool,
    pub is_splash: bool,
}

impl PotionMetadata {
    pub const WATER: Self = Self {
        potion_type: PotionType::Water,
        is_extended: false,
        is_amplified: false,
        is_splash: false,
    };

    pub fn new(potion_type: PotionType) -> Self {
        Self {
            potion_type,
            is_extended: false,
            is_amplified: false,
            is_splash: false,
        }
    }

    /// Encode into 16-bit item damage metadata
    pub fn to_meta_damage(&self) -> u16 {
        let type_id = match self.potion_type {
            PotionType::Water => 0,
            PotionType::Mundane => 1,
            PotionType::Thick => 2,
            PotionType::Awkward => 3,
            PotionType::Swiftness => 4,
            PotionType::Slowness => 5,
            PotionType::Strength => 6,
            PotionType::Weakness => 7,
            PotionType::Healing => 8,
            PotionType::Harming => 9,
            PotionType::Regeneration => 10,
            PotionType::FireResistance => 11,
            PotionType::Poison => 12,
        };

        let mut val = type_id & 0x00FF;
        if self.is_extended {
            val |= 0x0100;
        }
        if self.is_amplified {
            val |= 0x0200;
        }
        if self.is_splash {
            val |= 0x0400;
        }
        val
    }

    /// Decode from 16-bit item damage metadata
    pub fn from_meta_damage(meta: u16) -> Self {
        let type_id = meta & 0x00FF;
        let potion_type = match type_id {
            1 => PotionType::Mundane,
            2 => PotionType::Thick,
            3 => PotionType::Awkward,
            4 => PotionType::Swiftness,
            5 => PotionType::Slowness,
            6 => PotionType::Strength,
            7 => PotionType::Weakness,
            8 => PotionType::Healing,
            9 => PotionType::Harming,
            10 => PotionType::Regeneration,
            11 => PotionType::FireResistance,
            12 => PotionType::Poison,
            _ => PotionType::Water,
        };

        Self {
            potion_type,
            is_extended: (meta & 0x0100) != 0,
            is_amplified: (meta & 0x0200) != 0,
            is_splash: (meta & 0x0400) != 0,
        }
    }

    pub fn effect_type(&self) -> EffectType {
        match self.potion_type {
            PotionType::Water | PotionType::Awkward | PotionType::Thick | PotionType::Mundane => {
                EffectType::None
            }
            PotionType::Swiftness => EffectType::Speed,
            PotionType::Slowness => EffectType::Slowness,
            PotionType::Strength => EffectType::Strength,
            PotionType::Weakness => EffectType::Weakness,
            PotionType::Healing => EffectType::InstantHealth,
            PotionType::Harming => EffectType::InstantDamage,
            PotionType::Regeneration => EffectType::Regeneration,
            PotionType::FireResistance => EffectType::FireResistance,
            PotionType::Poison => EffectType::Poison,
        }
    }

    /// Duration in ticks (at 20 TPS)
    pub fn duration_ticks(&self) -> u32 {
        let base = match self.potion_type {
            PotionType::Swiftness | PotionType::Strength | PotionType::FireResistance => 3600, // 3:00
            PotionType::Slowness | PotionType::Weakness => 1800, // 1:30
            PotionType::Regeneration | PotionType::Poison => 900, // 0:45
            _ => 0,                                              // Instant
        };

        if self.is_extended {
            base * 8 / 3 // 3:00 -> 8:00 (9600 ticks), 1:30 -> 4:00 (4800), 0:45 -> 2:00 (2400)
        } else if self.is_amplified {
            base / 2 // Level II has reduced duration (e.g. 1:30)
        } else {
            base
        }
    }
}

pub struct BrewingStand {
    /// Slot 0: Top ingredient
    pub ingredient: ItemStack,
    /// Slots 1, 2, 3: Bottom potion bottles
    pub bottles: [ItemStack; 3],
    /// Progress counter (0..400)
    pub brew_time: u16,
}

impl BrewingStand {
    pub fn new() -> Self {
        Self {
            ingredient: ItemStack::EMPTY,
            bottles: [ItemStack::EMPTY; 3],
            brew_time: 0,
        }
    }

    pub fn is_brewing(&self) -> bool {
        self.brew_time > 0
    }

    /// Check if current ingredient can react with at least one bottle
    pub fn can_brew(&self) -> bool {
        if self.ingredient.is_empty() {
            return false;
        }

        for bottle in &self.bottles {
            if !bottle.is_empty() && bottle.item_id == ItemId(373) {
                let meta = PotionMetadata::from_meta_damage(bottle.damage);
                if Self::brew_transition(meta, self.ingredient.item_id).is_some() {
                    return true;
                }
            }
        }

        false
    }

    /// Potion recipe transition tree
    pub fn brew_transition(current: PotionMetadata, ingredient: ItemId) -> Option<PotionMetadata> {
        let id = ingredient.0;

        // 1. Modifiers: Redstone (331), Glowstone (348), Gunpowder (289)
        if id == 331 {
            // Redstone: Extends duration (cannot extend already amplified, extended, or instant potions)
            if !current.is_extended && !current.is_amplified && current.duration_ticks() > 0 {
                let mut next = current;
                next.is_extended = true;
                return Some(next);
            }
            if current.potion_type == PotionType::Water {
                return Some(PotionMetadata::new(PotionType::Mundane));
            }
            return None;
        }

        if id == 348 {
            // Glowstone: Amplifies level to Level II (cannot amplify already extended or instant)
            if !current.is_amplified
                && !current.is_extended
                && current.effect_type() != EffectType::None
            {
                let mut next = current;
                next.is_amplified = true;
                return Some(next);
            }
            if current.potion_type == PotionType::Water {
                return Some(PotionMetadata::new(PotionType::Thick));
            }
            return None;
        }

        if id == 289 {
            // Gunpowder: Converts into splash potion
            if !current.is_splash {
                let mut next = current;
                next.is_splash = true;
                return Some(next);
            }
            return None;
        }

        // 2. Nether Wart (372) -> Awkward Potion
        if id == 372 && current.potion_type == PotionType::Water {
            return Some(PotionMetadata::new(PotionType::Awkward));
        }

        // 3. Fermented Spider Eye (376) -> Corruptions / Inversions
        if id == 376 {
            let next_type = match current.potion_type {
                PotionType::Water => Some(PotionType::Weakness),
                PotionType::Swiftness | PotionType::FireResistance => Some(PotionType::Slowness),
                PotionType::Healing | PotionType::Poison => Some(PotionType::Harming),
                PotionType::Strength | PotionType::Regeneration => Some(PotionType::Weakness),
                _ => None,
            };
            return next_type.map(|t| PotionMetadata {
                potion_type: t,
                is_extended: current.is_extended,
                is_amplified: current.is_amplified,
                is_splash: current.is_splash,
            });
        }

        // 4. Primary ingredients onto Awkward Potion
        if current.potion_type == PotionType::Awkward {
            let next_type = match id {
                353 => Some(PotionType::Swiftness),      // Sugar
                382 => Some(PotionType::Healing),        // Glistering Melon
                375 => Some(PotionType::Poison),         // Spider Eye
                370 => Some(PotionType::Regeneration),   // Ghast Tear
                378 => Some(PotionType::FireResistance), // Magma Cream
                377 => Some(PotionType::Strength),       // Blaze Powder
                _ => None,
            };
            return next_type.map(|t| PotionMetadata {
                potion_type: t,
                is_extended: false,
                is_amplified: false,
                is_splash: current.is_splash,
            });
        }

        None
    }

    /// Simulate one 20 TPS tick of brewing stand
    pub fn tick(&mut self) {
        if !self.can_brew() {
            self.brew_time = 0;
            return;
        }

        self.brew_time += 1;
        if self.brew_time >= BREWING_TIME_TICKS {
            self.brew_time = 0;
            self.complete_brew();
        }
    }

    fn complete_brew(&mut self) {
        let ing_id = self.ingredient.item_id;

        for bottle in &mut self.bottles {
            if !bottle.is_empty() && bottle.item_id == ItemId(373) {
                let meta = PotionMetadata::from_meta_damage(bottle.damage);
                if let Some(next) = Self::brew_transition(meta, ing_id) {
                    bottle.damage = next.to_meta_damage();
                }
            }
        }

        self.ingredient.count -= 1;
        if self.ingredient.count == 0 {
            self.ingredient = ItemStack::EMPTY;
        }
    }
}

impl Default for BrewingStand {
    fn default() -> Self {
        Self::new()
    }
}

/// Active status effect with duration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveEffect {
    pub effect: EffectType,
    pub level: u8, // 1 for Level I, 2 for Level II
    pub remaining_ticks: u32,
}

#[derive(Debug, Clone)]
pub struct StatusEffectManager {
    pub effects: Vec<ActiveEffect>,
}

impl StatusEffectManager {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn add_effect(&mut self, effect: EffectType, level: u8, duration: u32) {
        if effect == EffectType::None {
            return;
        }

        // Refresh or upgrade existing
        if let Some(existing) = self.effects.iter_mut().find(|e| e.effect == effect) {
            if level >= existing.level {
                existing.level = level;
                existing.remaining_ticks = duration;
            }
        } else {
            self.effects.push(ActiveEffect {
                effect,
                level,
                remaining_ticks: duration,
            });
        }
    }

    pub fn has_effect(&self, effect: EffectType) -> bool {
        self.effects
            .iter()
            .any(|e| e.effect == effect && e.remaining_ticks > 0)
    }

    pub fn get_level(&self, effect: EffectType) -> u8 {
        self.effects
            .iter()
            .find(|e| e.effect == effect && e.remaining_ticks > 0)
            .map(|e| e.level)
            .unwrap_or(0)
    }

    /// Simulate one 20 TPS tick of active status effects on survival vitals
    pub fn tick(&mut self, vitals: &mut SurvivalVitals) {
        for effect in &mut self.effects {
            if effect.remaining_ticks == 0 {
                continue;
            }

            effect.remaining_ticks -= 1;

            match effect.effect {
                EffectType::Poison => {
                    // Poison deals 1 damage every 25 ticks, but cannot kill (stops at 1 HP)
                    if effect.remaining_ticks % 25 == 0 && vitals.health > 1.0 {
                        vitals.health = (vitals.health - 1.0).max(1.0);
                    }
                }
                EffectType::Regeneration => {
                    // Regeneration heals 1 HP every 25 (Level II) or 50 (Level I) ticks
                    let interval = if effect.level >= 2 { 25 } else { 50 };
                    if effect.remaining_ticks % interval == 0
                        && vitals.health < SurvivalVitals::MAX_HEALTH
                    {
                        vitals.health = (vitals.health + 1.0).min(SurvivalVitals::MAX_HEALTH);
                    }
                }
                _ => {}
            }
        }

        // Clean up expired effects
        self.effects.retain(|e| e.remaining_ticks > 0);
    }

    /// Drink a potion and apply its instant or timed status effect
    pub fn apply_potion(&mut self, potion: PotionMetadata, vitals: &mut SurvivalVitals) {
        let level = if potion.is_amplified { 2 } else { 1 };

        match potion.effect_type() {
            EffectType::InstantHealth => {
                let heal_amount = if level == 2 { 8.0 } else { 4.0 };
                vitals.health = (vitals.health + heal_amount).min(SurvivalVitals::MAX_HEALTH);
            }
            EffectType::InstantDamage => {
                let harm_amount = if level == 2 { 12.0 } else { 6.0 };
                vitals.apply_damage(harm_amount, 0);
            }
            other => {
                self.add_effect(other, level, potion.duration_ticks());
            }
        }
    }
}

impl Default for StatusEffectManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_potion_tree_progression_and_modifiers() {
        let mut stand = BrewingStand::new();

        // 3 Water Bottles in bottom slots
        for slot in &mut stand.bottles {
            *slot = ItemStack {
                item_id: ItemId(373),
                count: 1,
                damage: PotionMetadata::WATER.to_meta_damage(),
            };
        }

        // 1. Brew with Nether Wart (372) -> Awkward Potion
        stand.ingredient = ItemStack::new(ItemId(372), 1);
        assert!(stand.can_brew());

        for _ in 0..BREWING_TIME_TICKS {
            stand.tick();
        }

        assert_eq!(stand.ingredient, ItemStack::EMPTY);
        let meta = PotionMetadata::from_meta_damage(stand.bottles[0].damage);
        assert_eq!(meta.potion_type, PotionType::Awkward);

        // 2. Brew Awkward with Sugar (353) -> Swiftness (Speed)
        stand.ingredient = ItemStack::new(ItemId(353), 1);
        for _ in 0..BREWING_TIME_TICKS {
            stand.tick();
        }

        let meta = PotionMetadata::from_meta_damage(stand.bottles[0].damage);
        assert_eq!(meta.potion_type, PotionType::Swiftness);
        assert_eq!(meta.duration_ticks(), 3600); // 3:00

        // 3. Extend with Redstone (331) -> Extended Swiftness
        stand.ingredient = ItemStack::new(ItemId(331), 1);
        for _ in 0..BREWING_TIME_TICKS {
            stand.tick();
        }

        let meta = PotionMetadata::from_meta_damage(stand.bottles[0].damage);
        assert!(meta.is_extended);
        assert_eq!(meta.duration_ticks(), 9600); // 8:00
    }

    #[test]
    fn test_poison_status_effect_damage() {
        let mut vitals = SurvivalVitals::new(crate::survival::GameMode::Survival);
        vitals.health = 10.0;

        let mut mgr = StatusEffectManager::new();
        // Add 50 ticks of Poison (should tick twice: tick 25 and tick 50)
        mgr.add_effect(EffectType::Poison, 1, 50);

        for _ in 0..50 {
            mgr.tick(&mut vitals);
        }

        // Dealt 2 damage
        assert_eq!(vitals.health, 8.0);
    }
}
