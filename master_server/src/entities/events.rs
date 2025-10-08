use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub client_id: Uuid,
    pub ts: DateTimeWithTimeZone,
    pub level: EventLevel,
    pub kind: String,
    pub message: String,
    pub meta: Option<Json>,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "event_level")]
pub enum EventLevel {
    #[sea_orm(string_value = "info")]
    Info,
    #[sea_orm(string_value = "warn")]
    Warn,
    #[sea_orm(string_value = "error")]
    Error,
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
