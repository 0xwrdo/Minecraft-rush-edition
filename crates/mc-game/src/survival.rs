//! Survival vitals: Health, Hunger, Saturation, Exhaustion, Armor calculations,
//! Death, Respawn, and Hardcore mode.
//!
//! Has NO graphics dependencies and runs entirely headless.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Hardcore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone)]
pub struct SurvivalVitals {
    /// Health (0.0..=20.0, 20 is 10 full hearts)
    pub health: f32,
    /// Hunger food level (0..=20, 20 is 10 drumsticks)
    pub food_level: u8,
    /// Food saturation (0.0..=food_level)
    pub food_saturation: f32,
    /// Exhaustion level (0.0..=4.0)
    pub food_exhaustion: f32,

    pub game_mode: GameMode,
    pub difficulty: Difficulty,
    pub is_dead: bool,

    /// Air supply in ticks (300 ticks = 15 seconds, 10 bubbles)
    pub air: u16,
    pub drown_timer: u16,

    /// 80 ticks = 4.0 seconds per regen / starvation cycle
    pub regen_timer: u16,
    pub starvation_timer: u16,
}

impl SurvivalVitals {
    pub const MAX_HEALTH: f32 = 20.0;
    pub const MAX_FOOD: u8 = 20;
    pub const MAX_AIR: u16 = 300;
    pub const INITIAL_SATURATION: f32 = 5.0;
    pub const EXHAUSTION_THRESHOLD: f32 = 4.0;
    pub const TICK_CYCLE: u16 = 80;

    pub fn new(game_mode: GameMode) -> Self {
        Self {
            health: Self::MAX_HEALTH,
            food_level: Self::MAX_FOOD,
            food_saturation: Self::INITIAL_SATURATION,
            food_exhaustion: 0.0,
            game_mode,
            difficulty: if game_mode == GameMode::Hardcore {
                Difficulty::Hard
            } else {
                Difficulty::Normal
            },
            is_dead: false,
            air: Self::MAX_AIR,
            drown_timer: 0,
            regen_timer: 0,
            starvation_timer: 0,
        }
    }

    pub fn is_hardcore(&self) -> bool {
        self.game_mode == GameMode::Hardcore
    }

    /// Add food exhaustion. When exhaustion reaches 4.0, reduces saturation or food level.
    pub fn add_exhaustion(&mut self, amount: f32) {
        if self.game_mode == GameMode::Creative {
            return;
        }

        self.food_exhaustion += amount;
        while self.food_exhaustion >= Self::EXHAUSTION_THRESHOLD {
            self.food_exhaustion -= Self::EXHAUSTION_THRESHOLD;

            if self.food_saturation > 0.0 {
                self.food_saturation = (self.food_saturation - 1.0).max(0.0);
            } else if self.food_level > 0 {
                self.food_level -= 1;
            }
        }
    }

    /// Apply damage taking armor defense points (0..20) into account.
    /// In Minecraft 1.0, each armor defense point gives 4% damage reduction (up to 80%).
    /// Returns the net damage dealt to health.
    pub fn apply_damage(&mut self, raw_damage: f32, armor_points: u8) -> f32 {
        if self.game_mode == GameMode::Creative || self.is_dead {
            return 0.0;
        }

        let clamped_armor = armor_points.min(20);
        let reduction = (clamped_armor as f32) * 0.04;
        let final_damage = (raw_damage * (1.0 - reduction)).max(0.0);

        self.health = (self.health - final_damage).max(0.0);
        self.add_exhaustion(0.3); // Taking damage adds 0.3 exhaustion

        if self.health <= 0.0 {
            self.health = 0.0;
            self.is_dead = true;
        }

        final_damage
    }

    /// Eat food item. Returns false if player is too full to eat.
    pub fn eat_food(&mut self, food_points: u8, saturation_ratio: f32) -> bool {
        if self.food_level >= Self::MAX_FOOD {
            return false;
        }

        self.food_level = (self.food_level + food_points).min(Self::MAX_FOOD);
        // Saturation formula: food_points * saturation_ratio * 2.0, capped at current food level
        let added_saturation = (food_points as f32) * saturation_ratio * 2.0;
        self.food_saturation =
            (self.food_saturation + added_saturation).min(self.food_level as f32);

        true
    }

    /// Update survival vitals each tick (natural regeneration and starvation)
    pub fn tick(&mut self) {
        if self.is_dead || self.game_mode == GameMode::Creative {
            return;
        }

        // Peaceful difficulty: continuous health recovery and no hunger drain
        if self.difficulty == Difficulty::Peaceful {
            if self.food_level < Self::MAX_FOOD {
                self.food_level = Self::MAX_FOOD;
            }
            if self.health < Self::MAX_HEALTH {
                self.health = (self.health + 0.05).min(Self::MAX_HEALTH);
            }
            return;
        }

        // Natural regeneration: when food >= 18 and health < 20, heal 1 HP every 80 ticks
        // // TODO(verify): Exact food threshold in MC 1.0 (18 vs 20)
        if self.food_level >= 18 && self.health < Self::MAX_HEALTH {
            self.regen_timer += 1;
            if self.regen_timer >= Self::TICK_CYCLE {
                self.regen_timer = 0;
                self.health = (self.health + 1.0).min(Self::MAX_HEALTH);
                self.add_exhaustion(3.0); // Healing costs 3.0 exhaustion in MC 1.0
            }
        } else {
            self.regen_timer = 0;
        }

        // Starvation: when food == 0, take 1 damage every 80 ticks
        if self.food_level == 0 {
            self.starvation_timer += 1;
            if self.starvation_timer >= Self::TICK_CYCLE {
                self.starvation_timer = 0;

                let min_health_allowed = match self.difficulty {
                    Difficulty::Peaceful => Self::MAX_HEALTH,
                    Difficulty::Easy => 10.0,
                    Difficulty::Normal => 1.0,
                    Difficulty::Hard => 0.0, // Can starve to death
                };

                if self.is_hardcore() || self.difficulty == Difficulty::Hard {
                    self.health = (self.health - 1.0).max(0.0);
                    if self.health <= 0.0 {
                        self.is_dead = true;
                    }
                } else if self.health > min_health_allowed {
                    self.health = (self.health - 1.0).max(min_health_allowed);
                }
            }
        } else {
            self.starvation_timer = 0;
        }
    }

    /// Update environmental hazards (air/oxygen and drowning damage in water)
    pub fn tick_environment(&mut self, in_water: bool) {
        if self.is_dead || self.game_mode == GameMode::Creative {
            return;
        }

        if in_water {
            if self.air > 0 {
                self.air -= 1;
            } else {
                self.drown_timer += 1;
                if self.drown_timer >= 20 {
                    self.drown_timer = 0;
                    self.apply_damage(2.0, 0); // 2 drowning damage (1 heart) per second
                }
            }
        } else {
            self.air = Self::MAX_AIR;
            self.drown_timer = 0;
        }
    }

    /// Attempt to respawn the player. Returns false in Hardcore mode (permanent death).
    pub fn respawn(&mut self) -> bool {
        if self.is_hardcore() {
            // Hardcore mode: death is permanent!
            return false;
        }

        self.health = Self::MAX_HEALTH;
        self.food_level = Self::MAX_FOOD;
        self.food_saturation = Self::INITIAL_SATURATION;
        self.food_exhaustion = 0.0;
        self.air = Self::MAX_AIR;
        self.drown_timer = 0;
        self.is_dead = false;
        self.regen_timer = 0;
        self.starvation_timer = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_armor_damage_mitigation() {
        let mut vitals = SurvivalVitals::new(GameMode::Survival);

        // 20 armor defense points gives 80% reduction: 10 raw damage becomes 2 net damage
        let net = vitals.apply_damage(10.0, 20);
        assert!((net - 2.0).abs() < 1e-4);
        assert_eq!(vitals.health, 18.0);
    }

    #[test]
    fn test_eating_and_saturation() {
        let mut vitals = SurvivalVitals::new(GameMode::Survival);
        vitals.food_level = 10;
        vitals.food_saturation = 0.0;

        // Bread: 5 food points, 0.6 saturation ratio => +6.0 saturation
        assert!(vitals.eat_food(5, 0.6));
        assert_eq!(vitals.food_level, 15);
        assert!((vitals.food_saturation - 6.0).abs() < 1e-4);
    }

    #[test]
    fn test_exhaustion_depletes_saturation_then_food() {
        let mut vitals = SurvivalVitals::new(GameMode::Survival);
        vitals.food_saturation = 1.0;
        vitals.food_level = 20;

        // Add 4.0 exhaustion: burns 1 saturation
        vitals.add_exhaustion(4.0);
        assert_eq!(vitals.food_saturation, 0.0);
        assert_eq!(vitals.food_level, 20);

        // Add 4.0 more exhaustion: burns 1 food level
        vitals.add_exhaustion(4.0);
        assert_eq!(vitals.food_saturation, 0.0);
        assert_eq!(vitals.food_level, 19);
    }

    #[test]
    fn test_hardcore_mode_permanent_death() {
        let mut vitals = SurvivalVitals::new(GameMode::Hardcore);
        assert!(vitals.is_hardcore());

        vitals.apply_damage(100.0, 0);
        assert!(vitals.is_dead);
        assert_eq!(vitals.health, 0.0);

        // Respawn must fail in Hardcore
        assert!(!vitals.respawn());
    }

    #[test]
    fn test_natural_regeneration_consumes_exhaustion() {
        let mut vitals = SurvivalVitals::new(GameMode::Survival);
        vitals.health = 15.0;
        vitals.food_level = 20;

        for _ in 0..80 {
            vitals.tick();
        }

        assert_eq!(vitals.health, 16.0);
        assert_eq!(vitals.food_exhaustion, 3.0);
    }

    #[test]
    fn test_underwater_air_depletion_and_drowning() {
        let mut vitals = SurvivalVitals::new(GameMode::Survival);
        assert_eq!(vitals.air, 300);

        // 100 ticks in water
        for _ in 0..100 {
            vitals.tick_environment(true);
        }
        assert_eq!(vitals.air, 200);

        // Surfacing resets air
        vitals.tick_environment(false);
        assert_eq!(vitals.air, 300);

        // Fully exhaust air: 300 ticks in water
        for _ in 0..300 {
            vitals.tick_environment(true);
        }
        assert_eq!(vitals.air, 0);

        // 20 ticks out of air triggers 2 drowning damage
        for _ in 0..20 {
            vitals.tick_environment(true);
        }
        assert_eq!(vitals.health, 18.0);
    }
}
