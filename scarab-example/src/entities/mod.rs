use core::marker::PhantomData;

use derivative::Derivative;
use graphics::{types::Color, Context};
use opengl_graphics::GlGraphics;
use piston::RenderArgs;
use scarab_engine::{
    error::RenderResult,
    gameobject::entity::{
        registry::{RegisteredDebugEntity, RegisteredEntity},
        Entity, HasEntity,
    },
    rendering::{
        components::progress_bar::{self, InsetPosition},
        debug::{DebugView, StandardAndDebugView},
        registry::TextureRegistry,
        sprite::{AnimationStateMachine, StaticAnimation},
        Camera, View,
    },
    scene::GameTickArgs,
    types::{physbox::HasBox, HasHealth},
    ScarabResult,
};
use serde::{Deserialize, Serialize};

pub use self::{
    enemy::Enemy,
    player::{Player, PlayerAnimations, PlayerDebug},
};
use crate::debug::DebugOptions;

mod enemy;
mod player;

#[derive(Debug, Serialize, Deserialize)]
pub enum ExampleEntities {
    Player(
        (
            Player,
            StandardAndDebugView<AnimationStateMachine<PlayerAnimations>, PlayerDebug>,
        ),
    ),
    Enemy(
        (
            Enemy,
            StandardAndDebugView<AnimationStateMachine<StaticAnimation<Enemy>>, EntityDebug<Enemy>>,
        ),
    ),
}

impl RegisteredEntity for ExampleEntities {
    type Player = Player;

    fn inner_entity(&self) -> &Entity {
        match self {
            Self::Player((player, _view)) => &player.get_entity(),
            Self::Enemy((enemy, _view)) => &enemy.get_entity(),
        }
    }

    fn inner_entity_mut(&mut self) -> &mut Entity {
        match self {
            Self::Player((player, _view)) => &mut player.entity,
            Self::Enemy((enemy, _view)) => &mut enemy.entity,
        }
    }

    fn render(
        &mut self,
        args: &RenderArgs,
        camera: &Camera,
        ctx: graphics::Context,
        texture_registry: &TextureRegistry,
        gl: &mut opengl_graphics::GlGraphics,
    ) -> RenderResult<()> {
        match self {
            Self::Player((player, view)) => {
                view.render(player, args, camera, ctx, texture_registry, gl)
            }
            Self::Enemy((enemy, view)) => {
                view.render(enemy, args, camera, ctx, texture_registry, gl)
            }
        }
    }

    fn maybe_player(&self) -> Option<&<ExampleEntities as RegisteredEntity>::Player> {
        match self {
            Self::Player((p, _)) => Some(p),
            _ => None,
        }
    }

    fn maybe_player_mut(&mut self) -> Option<&mut <ExampleEntities as RegisteredEntity>::Player> {
        match self {
            Self::Player((p, _)) => Some(p),
            _ => None,
        }
    }

    fn game_tick(&mut self, this_idx: usize, args: &mut GameTickArgs<Self>) -> ScarabResult<()> {
        match self {
            ExampleEntities::Player((player, _)) => player.game_tick(this_idx, args),
            ExampleEntities::Enemy((enemy, _)) => {
                enemy.entity.game_tick(args).map_err(|e| e.into())
            }
        }
    }
}

impl RegisteredDebugEntity for ExampleEntities {
    type DebugOptions = DebugOptions;

    fn render_with_info(
        &mut self,
        debug_options: &Self::DebugOptions,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()> {
        match self {
            Self::Player((player, view)) => view.render_with_info(
                player,
                debug_options,
                args,
                camera,
                ctx,
                texture_registry,
                gl,
            ),
            Self::Enemy((enemy, view)) => view.render_with_info(
                enemy,
                debug_options,
                args,
                camera,
                ctx,
                texture_registry,
                gl,
            ),
        }
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone, Debug)]
pub struct EntityDebug<E> {
    pub box_color: Color,
    pub health_color: Color,
    #[derivative(Debug = "ignore")]
    phantom: PhantomData<E>,
}

impl<E> EntityDebug<E> {
    pub fn new(box_color: Color, health_color: Color) -> Self {
        Self {
            box_color,
            health_color,
            phantom: PhantomData::default(),
        }
    }
}

impl<E> DebugView for EntityDebug<E>
where
    E: HasEntity + HasHealth,
{
    type Viewed = E;
    type DebugOptions = DebugOptions;

    fn render_with_info(
        &mut self,
        viewed: &Self::Viewed,
        debug_options: &Self::DebugOptions,
        _args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        _texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()> {
        if let Some((transform, rect)) = camera.box_renderables(viewed.get_entity().get_box(), ctx)
        {
            if debug_options.entity_collision_boxes {
                graphics::rectangle(self.box_color, rect, transform, gl);
            }

            if debug_options.entity_health {
                graphics::rectangle(
                    self.health_color,
                    progress_bar::inset_left_to_right(
                        &rect,
                        1.0,
                        0.3,
                        viewed.get_health().fraction(),
                        InsetPosition::Inverse(0.0),
                    ),
                    transform,
                    gl,
                );
            }
        }

        Ok(())
    }
}
