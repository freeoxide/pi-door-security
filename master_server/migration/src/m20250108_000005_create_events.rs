use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create event level enum
        manager
            .create_type(
                Type::create()
                    .as_enum(EventLevel::Enum)
                    .values([EventLevel::Info, EventLevel::Warn, EventLevel::Error])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Events::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Events::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Events::ClientId).uuid().not_null())
                    .col(
                        ColumnDef::new(Events::Ts)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Events::Level)
                            .enumeration(EventLevel::Enum, [
                                EventLevel::Info,
                                EventLevel::Warn,
                                EventLevel::Error,
                            ])
                            .not_null(),
                    )
                    .col(ColumnDef::new(Events::Kind).string().not_null())
                    .col(ColumnDef::new(Events::Message).text().not_null())
                    .col(ColumnDef::new(Events::Meta).json_binary())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_events_client_id")
                            .from(Events::Table, Events::ClientId)
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
                    .name("idx_events_client_id")
                    .table(Events::Table)
                    .col(Events::ClientId)
                    .to_owned(),
            )
            .await?;

        // Create index on ts for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_events_ts")
                    .table(Events::Table)
                    .col(Events::Ts)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Events::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(EventLevel::Enum).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Events {
    Table,
    Id,
    ClientId,
    Ts,
    Level,
    Kind,
    Message,
    Meta,
}

#[derive(DeriveIden)]
enum EventLevel {
    #[sea_orm(iden = "event_level")]
    Enum,
    Info,
    Warn,
    Error,
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
}
