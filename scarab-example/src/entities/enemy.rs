use scarab_engine::{
    effect::{
        effect_helpers::{FollowBox, TargetFirstPlayer},
        PendingEffect,
    },
    gameobject::entity::Entity,
    scene::GameTickArgs,
    HasBox, HasBoxMut, HasEntity, HasHealth, HasSolidity, HasUuid, ScarabResult,
};
use serde::{Deserialize, Serialize};

use super::ExampleEntities;

#[derive(
    Debug, Serialize, Deserialize, HasBox, HasBoxMut, HasEntity, HasHealth, HasSolidity, HasUuid,
)]
pub struct Enemy {
    #[has_box]
    #[has_entity]
    #[has_health]
    #[has_solidity]
    #[has_uuid]
    pub entity: Entity,
}

impl Enemy {
    pub fn game_tick(
        &mut self,
        this_idx: usize,
        args: &mut GameTickArgs<ExampleEntities>,
    ) -> ScarabResult<()> {
        self.entity.game_tick(args)?;

        args.pending_effects.push(PendingEffect {
            source: Some((this_idx, false).into()),
            target: Box::new(TargetFirstPlayer::default()),
            effect: Box::new(FollowBox::default()),
        });

        Ok(())
    }
}
