pub use sea_orm_migration::prelude::*;

mod m20250108_000001_create_users;
mod m20250108_000002_create_clients;
mod m20250108_000003_create_user_clients;
mod m20250108_000004_create_sessions;
mod m20250108_000005_create_events;
mod m20250108_000006_create_commands;
mod m20250108_000007_create_heartbeats;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250108_000001_create_users::Migration),
            Box::new(m20250108_000002_create_clients::Migration),
            Box::new(m20250108_000003_create_user_clients::Migration),
            Box::new(m20250108_000004_create_sessions::Migration),
            Box::new(m20250108_000005_create_events::Migration),
            Box::new(m20250108_000006_create_commands::Migration),
            Box::new(m20250108_000007_create_heartbeats::Migration),
        ]
    }
}
