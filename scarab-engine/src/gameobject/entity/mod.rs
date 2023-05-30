use graphics::{
    types::{Color, Scalar},
    Context,
};
use opengl_graphics::GlGraphics;
use piston::RenderArgs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::field::{Cell, Field};
use crate::{
    error::RenderResult,
    rendering::{registry::TextureRegistry, Camera, View},
    scene::GameTickArgs,
    types::{
        physbox::{HasBox, PhysBox},
        HasSolidity, Health, Solidity, Velocity, SOLID,
    },
    HasBox, HasBoxMut, HasHealth, HasSolidity, HasUuid, PhysicsError, PhysicsResult, ScarabResult,
};

/// Handles the registration of entities (loading and unloading)
pub mod registry;

/// A trait for game objects that wrap/own an entity
pub trait HasEntity {
    /// Returns a reference to the game object's inner entity
    fn get_entity(&self) -> &Entity;

    /// Returns a mutable reference to the game object's inner entity
    fn get_entity_mut(&mut self) -> &mut Entity;
}

#[derive(Debug, Serialize, Deserialize, HasBox, HasBoxMut, HasHealth, HasUuid, HasSolidity)]
/// The basic structure of any non-static object in a game state
pub struct Entity {
    velocity: Velocity,
    max_velocity: Scalar,
    #[has_box]
    physbox: PhysBox,
    #[has_health]
    health: Health,
    #[has_solidity]
    solidity: Solidity,
    #[has_uuid]
    uuid: Uuid,
}

impl Entity {
    /// Creates an Entity with default settings
    pub fn new() -> ScarabResult<Self> {
        Ok(Self {
            velocity: [0.0, 0.0].into(),
            max_velocity: 1.0,
            physbox: PhysBox::new([0.0, 0.0, 1.0, 1.0].into())?,
            health: Health::new(10.0),
            solidity: SOLID,
            uuid: Uuid::new_v4(),
        })
    }

    /// Returns the entity's unique identifier ([Uuid])
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Sets the entity's velocity, limited by its maximum velocity
    pub fn set_velocity(&mut self, velocity: Velocity) {
        self.velocity = if velocity.magnitude_sq() <= self.max_velocity * self.max_velocity {
            velocity
        } else {
            velocity.normalize() * self.max_velocity
        }
    }

    /// Gets the entity's current velocity
    pub fn get_velocity(&self) -> Velocity {
        self.velocity
    }

    /// Sets the entity's maximum velocity. Must be greater than or equal to 0
    pub fn set_max_velocity(&mut self, max_velocity: Scalar) -> PhysicsResult<()> {
        if max_velocity < 0.0 {
            return Err(PhysicsError::MaxVelocity);
        }
        self.max_velocity = max_velocity;

        Ok(())
    }

    /// Gets the entity's maximum velocity
    pub fn get_max_velocity(&self) -> Scalar {
        self.max_velocity
    }

    /// Get the position of the entity after its next movement assuming no collisions
    pub fn get_projected_box(&self) -> PhysBox {
        let mut physbox = self.physbox.clone();
        physbox.set_pos(*physbox.pos() + self.velocity);
        physbox
    }

    /// Returns a callback function for resolving entity-entity collisions
    pub fn game_tick<E>(&mut self, args: &GameTickArgs<E>) -> PhysicsResult<()> {
        self.try_move(args.field, args.dt)
    }

    /// Attempts to move this entity according to its velocity until it collides
    /// with any cells
    fn try_move(&mut self, field: &Field, dt: f64) -> PhysicsResult<()> {
        if self.velocity == [0.0, 0.0].into() {
            return Ok(());
        }

        // TODO: having to recalculate the current cell every time will get time intensive
        // Should create a new function to take into account the old current cell and its neighbors
        // at the very least only going through those. Even more so, we can add the edges that were
        // crossed.
        let current_cell = field
            .cell_at_pos(*self.physbox.pos())
            .ok_or_else(|| PhysicsError::NoFieldCell(*self.physbox.pos()))?;
        let current_cell_overlaps =
            field.neighbors_of_cell_overlapping_box(current_cell, &self.physbox)?;

        let new_pos = *self.physbox.pos() + self.velocity * dt;
        let mut new_box = self.physbox.clone();
        new_box.set_pos(new_pos);

        // Cell Based collisions
        if !new_box.is_fully_contained_by(&current_cell.get_box()) {
            let mut apply_movement_reductions = |from_this_cell: &Cell| -> PhysicsResult<()> {
                let from_cells_neighbors =
                    field.neighbors_of_cell_overlapping_box(from_this_cell, &new_box)?;

                for (edge, neighbors_on_edge) in from_cells_neighbors.iter_by_edge() {
                    for neighbor in neighbors_on_edge {
                        if (!from_this_cell.get_solidity().exit_edge(edge)
                            || !neighbor.get_solidity().enter_edge(edge.opposite()))
                            && self.velocity.is_reduced_by_edge(edge)
                        {
                            new_box.set_touching_edge(&from_this_cell.get_box(), edge);
                        }
                    }

                    // Don't care about the solidity when we're leaving and not
                    // entering into a new neighbor because this can lead to tricky behavoir
                    // because at present, movement is not defined when the entity is not
                    // fully contained by some number of cells
                    if neighbors_on_edge.len() == 0
                        && self.velocity.is_reduced_by_edge(edge)
                        && from_this_cell.get_box().is_edge_crossed_by(&new_box, edge)
                    {
                        new_box.set_touching_edge(&from_this_cell.get_box(), edge);
                    }
                }

                Ok(())
            };

            apply_movement_reductions(&current_cell)?;

            for overlap in current_cell_overlaps.iter_all() {
                apply_movement_reductions(overlap)?;
            }
        }

        self.physbox = new_box;

        // TODO: switch to a separate "resolve entity collisions step"
        // doing these collated will definite cause problems as the number
        // of entities increases
        Ok(())
    }
}

impl HasEntity for Entity {
    fn get_entity(&self) -> &Entity {
        self
    }

    fn get_entity_mut(&mut self) -> &mut Entity {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Renders an entity by filling its PhysBox with the set color
pub struct EntityView {
    /// The color to render the entity with
    pub color: Color,
}

impl View for EntityView {
    type Viewed = Entity;

    fn render(
        &mut self,
        viewed: &Self::Viewed,
        _args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        _texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()> {
        if let Some((transform, rect)) = camera.box_renderables(&viewed.physbox, ctx) {
            graphics::rectangle(self.color, rect, transform, gl);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::PhysicsError;

    // Doing a lot of square roots with the vector math propogates the floating-point error
    // a bunch, this is just to make sure it's reasonably accurate
    const EPSILON: f64 = 0.000_000_000_1;

    #[test]
    fn set_max_velocity_fails_with_negative() {
        let mut entity = Entity::new().unwrap();

        assert_eq!(
            entity.set_max_velocity(-1.0).unwrap_err(),
            PhysicsError::MaxVelocity
        );
    }

    #[test]
    fn set_velocity_bounded_by_max_velocity_maintains_angle() {
        let mut entity = Entity::new().unwrap();

        entity.set_max_velocity(20.0).unwrap();

        let velocity = [20.0, 20.0].into();
        entity.set_velocity(velocity);
        assert!((400.0 - entity.velocity.magnitude_sq()) <= EPSILON);
        assert!((entity.velocity.angle() - velocity.angle()).abs() < EPSILON);

        let velocity = [-100.0, 20.0].into();
        entity.set_velocity(velocity);
        assert!((400.0 - entity.velocity.magnitude_sq()) <= EPSILON);
        assert!((entity.velocity.angle() - velocity.angle()).abs() < EPSILON);

        let velocity = [10.0, 10.0].into();
        entity.set_velocity(velocity);
        assert_eq!(entity.velocity, velocity);
    }
}
