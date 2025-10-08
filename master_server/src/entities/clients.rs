use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "clients")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub label: String,
    #[sea_orm(unique)]
    pub provision_key: Uuid,
    pub eth0_ip: Option<String>,
    pub wlan0_ip: Option<String>,
    pub service_port: Option<i32>,
    pub status: ClientStatus,
    pub last_seen_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "client_status")]
pub enum ClientStatus {
    #[sea_orm(string_value = "unknown")]
    Unknown,
    #[sea_orm(string_value = "online")]
    Online,
    #[sea_orm(string_value = "offline")]
    Offline,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_clients::Entity")]
    UserClients,
    #[sea_orm(has_many = "super::events::Entity")]
    Events,
    #[sea_orm(has_many = "super::commands::Entity")]
    Commands,
    #[sea_orm(has_many = "super::heartbeats::Entity")]
    Heartbeats,
}

impl Related<super::user_clients::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserClients.def()
    }
}

impl Related<super::events::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Events.def()
    }
}

impl Related<super::commands::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Commands.def()
    }
}

impl Related<super::heartbeats::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Heartbeats.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
