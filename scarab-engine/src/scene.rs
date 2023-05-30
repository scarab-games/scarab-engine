use std::fmt::Debug;

use graphics::Context;
use opengl_graphics::GlGraphics;
use piston::RenderArgs;
use serde::{Deserialize, Serialize};

use crate::{
    gameobject::{
        entity::registry::{EntityRegistry, RegisteredDebugEntity, RegisteredEntity},
        field::Field,
    },
    rendering::{debug::DebugView, registry::TextureRegistry, Camera, View},
    types::{
        physbox::{HasBox, HasBoxMut, PhysBox},
        HasSolidity,
    },
    ScarabResult,
};

#[derive(Debug, Serialize, Deserialize)]
/// A wrapper over all things in the app right now
pub struct Scene<E, V> {
    field: Field,
    field_view: V,
    entity_registry: EntityRegistry<E>,
    #[serde(skip)]
    #[serde(default = "Vec::new")]
    pending_attacks: Vec<PendingEffect<E>>,
}

impl<E, V> Scene<E, V>
where
    E: RegisteredEntity + Debug,
    V: View<Viewed = Field>,
{
    /// Initializes a new scene with the given field, field view and no entities
    pub fn new(field: Field, field_view: V) -> Self {
        Self {
            field,
            field_view,
            entity_registry: EntityRegistry::default(),
            pending_attacks: Vec::default(),
        }
    }

    /// Renders everything in the scene
    pub fn render(
        &mut self,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> ScarabResult<()> {
        self.field_view
            .render(&mut self.field, args, &camera, ctx, texture_registry, gl)?;

        for registered_entity in &mut self.entity_registry {
            registered_entity.render(args, camera, ctx, texture_registry, gl)?;
        }
        Ok(())
    }

    #[cfg(feature = "debug-rendering")]
    /// Renders the scene with additional debug info
    pub fn render_with_info<D>(
        &mut self,
        debug_options: &D,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> ScarabResult<()>
    where
        E: RegisteredDebugEntity<DebugOptions = D>,
        V: DebugView<Viewed = Field, DebugOptions = D>,
    {
        self.field_view.render_with_info(
            &mut self.field,
            debug_options,
            args,
            &camera,
            ctx,
            texture_registry,
            gl,
        )?;

        for registered_entity in &mut self.entity_registry {
            registered_entity.render_with_info(
                debug_options,
                args,
                camera,
                ctx,
                texture_registry,
                gl,
            )?;
        }
        Ok(())
    }

    /// Registers a new entity to the scene
    pub fn register_entity(&mut self, to_register: E) -> ScarabResult<()> {
        self.entity_registry.register(to_register)
    }

    /// Gets a reference to the scene's [Field]
    pub fn get_field(&self) -> &Field {
        &self.field
    }

    /// Runs the physics update for all of the scene's entities
    pub fn tick_entities(&mut self, dt: f64) -> ScarabResult<()> {
        let mut args = GameTickArgs {
            field: &self.field,
            pending_attacks: &mut self.pending_attacks,
            dt,
        };
        for (i, registered_entity) in self.entity_registry.iter_mut().enumerate() {
            registered_entity.game_tick(i, &mut args)?;
        }

        self.handle_entity_collisions()?;

        self.process_pending_effects()?;

        Ok(())
    }

    // TODO! Find a way to pin the return type of this to a specific type within the registry
    /// Optionally returns a mutable reference to the scene's player
    pub fn player_mut(&mut self) -> Option<&mut E::Player> {
        self.entity_registry.player_mut()
    }

    fn handle_entity_collisions(&mut self) -> ScarabResult<()> {
        // This is kinda gross, but I don't really know how else to do it
        // we'll see later how necessary it is to change
        for this_index in 0..self.entity_registry.len() {
            if let Some(this_one) = self.entity_registry.get_one(this_index) {
                if !this_one.get_solidity().has_solidity() {
                    continue;
                }

                let this_one_box = *this_one.get_box();

                for other_index in 0..this_index {
                    if this_index == other_index {
                        continue;
                    }
                    if let Some(other_one) = self.entity_registry.get_one_mut(other_index) {
                        if other_one.get_solidity().has_solidity() {
                            other_one
                                .get_box_mut()
                                .shift_to_nonoverlapping(&this_one_box);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn process_pending_effects(&mut self) -> ScarabResult<()> {
        let _ = self.pending_attacks.drain_filter(|effect| {
            let keep_effect = self
                .entity_registry
                .iter_mut()
                .enumerate()
                .filter_map(|(i, e)| {
                    // TODO! remove inefficient retrieval of overlapping entities
                    // Do not attack if it's the source and the source can't be targeted
                    if effect.source.map_or(true, |s| s.should_apply_effect(i))
                        && e.get_box().has_overlap(&effect.target_area)
                    {
                        let res = effect.effect.apply_effect(e).ok();
                        Some(res).flatten()
                    } else {
                        None
                    }
                })
                .any(|b| b);

            effect.source.map(|s| {
                self.entity_registry
                    .get_one_mut(s.index)
                    .map(|source_entity| effect.effect.update_src(source_entity))
                    .or_else(|| {
                        println!(
                            "error processing attack: could not find source entity: {:?}",
                            effect
                        );
                        None
                    });
            });

            // Drain filter *REMOVES* when true
            !keep_effect
        });

        Ok(())
    }
}

#[derive(Debug)]
/// Various arguments used for running game ticks on entities
pub struct GameTickArgs<'a, E> {
    /// The field which the updated entity is on
    pub field: &'a Field,
    /// The current attacks waiting to be processed in the game loop. Add to this to attack another entity
    pub pending_attacks: &'a mut Vec<PendingEffect<E>>,
    /// The change in time for this update
    pub dt: f64,
}

#[derive(Debug)]
/// An effect on other entities that the scene should process on the next game tick
pub struct PendingEffect<E> {
    /// An optional source of the effect
    pub source: Option<EffectSource>,
    /// The attack's target area
    /// TODO: this could be changed into a more generalized "EffectTarget" which could just
    /// get the nearest "n" entities within a range for example
    pub target_area: PhysBox,
    /// Handles the logic of applying the effect
    pub effect: Box<dyn TargetsOthers<E>>,
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
    fn should_apply_effect(&self, target_index: usize) -> bool {
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

/// Effects that can target other entities of type `E`
pub trait TargetsOthers<E>: Debug {
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
