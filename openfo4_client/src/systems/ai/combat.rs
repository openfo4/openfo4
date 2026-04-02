use crate::systems::ai::AiPackage;

#[derive(Copy, Clone, Debug)]
pub struct DefaultCombatPackageConfig {
    /// The larger this factor, the larger the chance that the actor prefers offensive actions.
    ///
    /// More offensive actors will:
    /// - Attack more often
    /// - Use power attacks more often
    /// - More more aggressively
    pub offensive_factor: f32,
    /// More defensive actors will:
    /// - Block more often
    /// - Gun bash more often
    pub defensive_factor: f32,
    /// Chance that the actor will hit the target.
    ///
    /// This factor is combined with weapon the accuracy.
    pub gun_accuracy_factor: f32,
    /// Chance that the actor will attack staggered targets.
    pub melee_attack_staggered_target: f32,
    /// Chance that the actor will power attack staggered targets.
    pub melee_power_attack_staggered_target: f32,
    /// Chance that the actor will power attack a blocking target.
    pub melee_power_attack_blocking_target: f32,
    /// Chance that the actor will gun bash.
    pub bash: f32,
    /// Chance that the actor will bash staggered targets.
    pub bash_staggered_target: f32,
    /// Chance that the actor will bash a target currently attacking.
    pub bash_attacking_target: f32,
    /// Chance that the actor will bash a target currently power attacking.
    pub bash_power_attacking_target: f32,
    /// Chance that the actor will circle around the target in either direction.
    pub dueling_circle: f32,
    /// Chance that the actor will fall back.
    pub dueling_fallback: f32,
}

#[derive(Clone, Debug)]
pub struct DefaultCombatPackage {
    config: DefaultCombatPackageConfig,
}

impl DefaultCombatPackage {
    fn update(&mut self) {}
}
