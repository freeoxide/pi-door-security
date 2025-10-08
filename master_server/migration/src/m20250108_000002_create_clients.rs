use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create status enum
        manager
            .create_type(
                Type::create()
                    .as_enum(ClientStatus::Enum)
                    .values([ClientStatus::Unknown, ClientStatus::Online, ClientStatus::Offline])
                    .to_owned(),
            )
            .await?;

        // Create clients table
        manager
            .create_table(
                Table::create()
                    .table(Clients::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Clients::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Clients::Label)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Clients::ProvisionKey)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Clients::Eth0Ip).string())
                    .col(ColumnDef::new(Clients::Wlan0Ip).string())
                    .col(ColumnDef::new(Clients::ServicePort).integer())
                    .col(
                        ColumnDef::new(Clients::Status)
                            .enumeration(ClientStatus::Enum, [
                                ClientStatus::Unknown,
                                ClientStatus::Online,
                                ClientStatus::Offline,
                            ])
                            .not_null()
                            .default("unknown"),
                    )
                    .col(ColumnDef::new(Clients::LastSeenAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Clients::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on status
        manager
            .create_index(
                Index::create()
                    .name("idx_clients_status")
                    .table(Clients::Table)
                    .col(Clients::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Clients::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(ClientStatus::Enum).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
    Label,
    ProvisionKey,
    Eth0Ip,
    Wlan0Ip,
    ServicePort,
    Status,
    LastSeenAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ClientStatus {
    #[sea_orm(iden = "client_status")]
    Enum,
    Unknown,
    Online,
    Offline,
}
