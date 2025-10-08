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
                    .table(UserClients::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserClients::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserClients::ClientId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(UserClients::UserId)
                            .col(UserClients::ClientId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_clients_user_id")
                            .from(UserClients::Table, UserClients::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_clients_client_id")
                            .from(UserClients::Table, UserClients::ClientId)
                            .to(Clients::Table, Clients::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on client_id for reverse lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_user_clients_client_id")
                    .table(UserClients::Table)
                    .col(UserClients::ClientId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserClients::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserClients {
    Table,
    UserId,
    ClientId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
}
