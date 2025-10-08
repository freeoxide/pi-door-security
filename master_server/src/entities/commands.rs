use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "commands")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub client_id: Uuid,
    pub issued_by: Uuid,
    pub ts_issued: DateTimeWithTimeZone,
    pub command: String,
    pub params: Option<Json>,
    pub status: CommandStatus,
    pub ts_updated: DateTimeWithTimeZone,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "command_status")]
pub enum CommandStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "sent")]
    Sent,
    #[sea_orm(string_value = "acked")]
    Acked,
    #[sea_orm(string_value = "failed")]
    Failed,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clients::Entity",
        from = "Column::ClientId",
        to = "super::clients::Column::Id"
    )]
    Clients,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::IssuedBy",
        to = "super::users::Column::Id"
    )]
    Users,
}

impl Related<super::clients::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clients.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
