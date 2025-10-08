use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "heartbeats")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub client_id: Uuid,
    pub ts: DateTimeWithTimeZone,
    pub uptime_ms: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clients::Entity",
        from = "Column::ClientId",
        to = "super::clients::Column::Id"
    )]
    Clients,
}

impl Related<super::clients::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clients.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
