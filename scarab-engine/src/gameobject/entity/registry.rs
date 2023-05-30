use graphics::Context;
use opengl_graphics::GlGraphics;
use piston::RenderArgs;
use serde::{Deserialize, Serialize};

use super::{Entity, HasEntity};
use crate::{
    error::RenderResult,
    rendering::{registry::TextureRegistry, Camera},
    scene::GameTickArgs,
    types::{
        physbox::{HasBox, HasBoxMut, PhysBox},
        HasHealth, HasSolidity, HasUuid,
    },
    ScarabResult,
};

// TODO: Eventually meant to be a trait that can be derived for enums whose
// variants impl HasEntity
/// A trait for types that can be a registered entity
/// This is commonly an enum whose variants are tuples containing Entity and some Entity View
pub trait RegisteredEntity
where
    Self: Sized,
{
    /// The type of entity that is the player
    type Player: HasEntity;

    /// A reference to the registered object's inner entity
    fn inner_entity(&self) -> &Entity;

    /// A mutable reference to the registered object's inner entity
    fn inner_entity_mut(&mut self) -> &mut Entity;

    /// If this is a player variant, returns Some(self), otherwise None
    fn maybe_player(&self) -> Option<&Self::Player>;

    /// If this is a player variant, returns Some(self), otherwise None
    fn maybe_player_mut(&mut self) -> Option<&mut Self::Player>;

    /// Runs the game tick update for the entity. By default runs the gametick on the inner entity
    fn game_tick(&mut self, _this_idx: usize, args: &mut GameTickArgs<Self>) -> ScarabResult<()> {
        self.inner_entity_mut()
            .game_tick(args)
            .map_err(|e| e.into())
    }

    /// Controls how the registered object renders the inner entity.
    /// This should usually be done by pairing the registered entity with something that impls [crate::rendering::View]
    fn render(
        &mut self,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()>;
}

impl<E: RegisteredEntity> HasBox for E {
    fn get_box(&self) -> &PhysBox {
        self.inner_entity().get_box()
    }
}

impl<E: RegisteredEntity> HasBoxMut for E {
    fn get_box_mut(&mut self) -> &mut PhysBox {
        self.inner_entity_mut().get_box_mut()
    }
}

impl<E: RegisteredEntity> HasEntity for E {
    fn get_entity(&self) -> &Entity {
        self.inner_entity()
    }

    fn get_entity_mut(&mut self) -> &mut Entity {
        self.inner_entity_mut()
    }
}

impl<E: RegisteredEntity> HasHealth for E {
    fn get_health(&self) -> &crate::types::Health {
        self.inner_entity().get_health()
    }

    fn get_health_mut(&mut self) -> &mut crate::types::Health {
        self.inner_entity_mut().get_health_mut()
    }
}

impl<E: RegisteredEntity> HasSolidity for E {
    fn get_solidity(&self) -> &crate::types::Solidity {
        &self.inner_entity().solidity
    }
}

impl<E: RegisteredEntity> HasUuid for E {
    fn uuid(&self) -> uuid::Uuid {
        self.inner_entity().uuid()
    }
}

#[cfg(feature = "debug-rendering")]
/// A registered entity that can be rendered with debug information
pub trait RegisteredDebugEntity: RegisteredEntity {
    /// The options controlling which debug info to show
    type DebugOptions;

    /// Renders the entity with additional debug info
    fn render_with_info(
        &mut self,
        debug_options: &Self::DebugOptions,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()>;
}

/// The registry of all entities that are active in a scene
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityRegistry<E> {
    inner: Vec<E>,
}

impl<E> Default for EntityRegistry<E> {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<E: RegisteredEntity> EntityRegistry<E> {
    /// Attempts to register a new entity to the scene
    pub fn register(&mut self, to_register: E) -> ScarabResult<()> {
        self.inner.push(to_register);
        Ok(())
    }

    /// Gets a reference to the registered player
    pub fn player(&self) -> Option<&E::Player> {
        self.inner.iter().find_map(E::maybe_player)
    }

    /// Gets a mutable reference to the registered player
    pub fn player_mut(&mut self) -> Option<&mut E::Player> {
        self.inner.iter_mut().find_map(E::maybe_player_mut)
    }

    /// The number of currently registered entities
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Gets a reference to the registered entity at the given index if it exists
    pub fn get_one(&self, i: usize) -> Option<&E> {
        self.inner.get(i)
    }

    /// Gets a mutable reference to the registered entity at the given index if it exists
    pub fn get_one_mut(&mut self, i: usize) -> Option<&mut E> {
        self.inner.get_mut(i)
    }

    /// Iterates across the registered entities
    pub fn iter(&self) -> core::slice::Iter<'_, E> {
        self.inner.iter()
    }

    /// Iterates across mutable references to the registered entities
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, E> {
        self.inner.iter_mut()
    }
}

impl<E> IntoIterator for EntityRegistry<E> {
    type Item = E;
    type IntoIter = std::vec::IntoIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, E> IntoIterator for &'a EntityRegistry<E> {
    type Item = &'a E;
    type IntoIter = core::slice::Iter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, E> IntoIterator for &'a mut EntityRegistry<E> {
    type Item = &'a mut E;
    type IntoIter = core::slice::IterMut<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}
