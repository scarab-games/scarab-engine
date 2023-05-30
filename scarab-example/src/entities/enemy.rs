use scarab_engine::{
    gameobject::entity::Entity, HasBox, HasBoxMut, HasEntity, HasHealth, HasSolidity, HasUuid,
};
use serde::{Deserialize, Serialize};

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
