use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Heartbeats::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Heartbeats::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Heartbeats::ClientId).uuid().not_null())
                    .col(
                        ColumnDef::new(Heartbeats::Ts)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Heartbeats::UptimeMs).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_heartbeats_client_id")
                            .from(Heartbeats::Table, Heartbeats::ClientId)
                            .to(Clients::Table, Clients::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on client_id
        manager
            .create_index(
                Index::create()
                    .name("idx_heartbeats_client_id")
                    .table(Heartbeats::Table)
                    .col(Heartbeats::ClientId)
                    .to_owned(),
            )
            .await?;

        // Create index on ts for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_heartbeats_ts")
                    .table(Heartbeats::Table)
                    .col(Heartbeats::Ts)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Heartbeats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Heartbeats {
    Table,
    Id,
    ClientId,
    Ts,
    UptimeMs,
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
}
