use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub otp_secret: Option<String>,
    pub otp_enabled: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_role")]
pub enum UserRole {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "user")]
    User,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sessions::Entity")]
    Sessions,
    #[sea_orm(has_many = "super::user_clients::Entity")]
    UserClients,
    #[sea_orm(has_many = "super::commands::Entity")]
    Commands,
}

impl Related<super::sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl Related<super::user_clients::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserClients.def()
    }
}

impl Related<super::commands::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Commands.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
