use graphics::types::Color;
use scarab_engine::{
    gameobject::entity::{
        effect_helpers::{BasicAttack, Cooldown, TryAction},
        Entity,
    },
    rendering::{
        components::progress_bar::{self, InsetPosition},
        debug::DebugView,
        sprite::AnimationStates,
        Camera,
    },
    scene::GameTickArgs,
    types::physbox::HasBox,
    HasBox, HasBoxMut, HasEntity, HasHealth, HasSolidity, HasUuid, ScarabResult,
};
use serde::{Deserialize, Serialize};
use shapes::Point;

use super::{EntityDebug, ExampleEntities};
use crate::debug::DebugOptions;

#[derive(
    Debug, Serialize, Deserialize, HasBox, HasBoxMut, HasEntity, HasHealth, HasSolidity, HasUuid,
)]
pub struct Player {
    #[has_box]
    #[has_entity]
    #[has_health]
    #[has_solidity]
    #[has_uuid]
    pub entity: Entity,
    // TODO! this needs to be a proper struct a tuple is just horrendous
    attack: (TryAction, BasicAttack, f64),
}

impl Player {
    pub fn new(entity: Entity, damage: f64, cooldown: f64) -> Self {
        Self {
            entity,
            attack: (TryAction::default(), BasicAttack::new(damage), cooldown),
        }
    }

    pub fn attack(&mut self) {
        self.attack.0.maybe_set_doing();
    }

    pub fn game_tick(
        &mut self,
        this_idx: usize,
        args: &mut GameTickArgs<ExampleEntities>,
    ) -> ScarabResult<()> {
        self.entity.game_tick(args)?;

        self.attack.0.cooldown.cool(args.dt);

        if self.attack.0.should_do(Cooldown::Cooling(self.attack.2)) {
            let mut target_area = self.get_box().clone();
            let size = self.get_box().size();
            let _ = target_area.set_size([size.w * 2.0, size.h * 2.0].into());
            target_area.set_pos(*self.get_box().pos() - Point::from([size.w, size.h]));
            args.pending_attacks
                .push(self.attack.1.into_pending_effect(this_idx, target_area));
        }

        Ok(())
    }

    pub fn cooldown_fraction(&self) -> f64 {
        f64::from(self.attack.0.cooldown) / self.attack.2
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerAnimations {
    Idle,
    Run,
}

impl AnimationStates for PlayerAnimations {
    type Viewed = Player;

    fn next_state(&self, viewed: &Self::Viewed) -> Option<Self> {
        let next = if viewed.entity.get_velocity().magnitude_sq() == 0.0 {
            Self::Idle
        } else {
            Self::Run
        };

        if next != *self {
            Some(next)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerDebug {
    pub entity_debug: EntityDebug<Player>,
    pub cooldown_color: Color,
}

impl PlayerDebug {
    pub fn new(box_color: Color, health_color: Color, cooldown_color: Color) -> Self {
        Self {
            entity_debug: EntityDebug::new(box_color, health_color),
            cooldown_color,
        }
    }
}

impl DebugView for PlayerDebug {
    type DebugOptions = DebugOptions;
    type Viewed = Player;

    fn render_with_info(
        &mut self,
        viewed: &Self::Viewed,
        debug_options: &Self::DebugOptions,
        args: &piston::RenderArgs,
        camera: &Camera,
        ctx: graphics::Context,
        texture_registry: &scarab_engine::rendering::registry::TextureRegistry,
        gl: &mut opengl_graphics::GlGraphics,
    ) -> scarab_engine::error::RenderResult<()> {
        self.entity_debug.render_with_info(
            viewed,
            debug_options,
            args,
            camera,
            ctx,
            texture_registry,
            gl,
        )?;

        if let Some((transform, rect)) = camera.box_renderables(viewed.get_box(), ctx) {
            if debug_options.attack_cooldowns {
                graphics::rectangle(
                    self.cooldown_color,
                    progress_bar::inset_left_to_right(
                        &rect,
                        1.0,
                        0.3,
                        viewed.cooldown_fraction(),
                        InsetPosition::Normal(0.0),
                    ),
                    transform,
                    gl,
                );
            }
        }

        Ok(())
    }
}
