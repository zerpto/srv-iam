pub use super::_entities::membership_roles::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use uuid::Uuid;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert && self.id.is_not_set() {
            self.id = ActiveValue::Set(Uuid::now_v7());
        }
        Ok(self)
    }
}
