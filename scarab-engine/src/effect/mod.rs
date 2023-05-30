use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    types::physbox::{HasBox, PhysBox},
    ScarabResult,
};

/// Helper structs for applying basic effects and attacks to entities
#[cfg(feature = "effect-helpers")]
pub mod effect_helpers;

#[derive(Debug)]
/// An effect on other entities that the scene should process on the next game tick
pub struct PendingEffect<E> {
    /// An optional source of the effect
    pub source: Option<EffectSource>,
    /// Determines which objects should be targeted by the source
    pub target: Box<dyn EffectTarget<E>>,
    /// Handles the logic of applying the effect
    pub effect: Box<dyn Effect<E>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// A source of an effect
pub struct EffectSource {
    /// The source's registry index
    pub index: usize,
    /// Whether or not the effect should target the source
    pub can_target_source: bool,
}

impl EffectSource {
    /// Checks, for the source parameters, whether the effect should be called on the source
    pub fn should_apply_effect(&self, target_index: usize) -> bool {
        !(!self.can_target_source && target_index == self.index)
    }
}

impl From<(usize, bool)> for EffectSource {
    fn from((index, can_target_source): (usize, bool)) -> Self {
        Self {
            index,
            can_target_source,
        }
    }
}

/// Checks whether an object is targeted by an effect source
pub trait EffectTarget<E>: Debug {
    /// Given a potential target, returns whether it should be targeted by the effect
    fn can_target(&mut self, potential_target: &E) -> bool;
}

impl<E: HasBox> EffectTarget<E> for PhysBox {
    fn can_target(&mut self, potential_target: &E) -> bool {
        self.has_overlap(potential_target.get_box())
    }
}

/// Effects that can target other entities of type `E`
pub trait Effect<E>: Debug {
    /// Apply the main effect to a target entity (i.e. do damage, apply status effects, etc.)
    /// Returns whether or not the effect needs to process on the next tick
    fn apply_effect(&mut self, target: &mut E) -> ScarabResult<bool>;

    /// Apply any necessary updates to the source of the effect
    /// This could be animation states, draining energy or any other necessary effect
    fn update_src(&mut self, src: &mut E) -> ScarabResult<()>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn effect_source_always_targets_when_not_source() {
        let source_index = 0;
        let mut source: EffectSource = (source_index, false).into();

        assert!(source.should_apply_effect(source_index + 1));

        source.can_target_source = true;
        assert!(source.should_apply_effect(source_index + 1));
    }

    #[test]
    fn effect_source_targets_source_only_when_able() {
        let source_index = 0;
        let mut source: EffectSource = (source_index, false).into();

        assert!(!source.should_apply_effect(source_index));

        source.can_target_source = true;
        assert!(source.should_apply_effect(source_index));
    }
}
