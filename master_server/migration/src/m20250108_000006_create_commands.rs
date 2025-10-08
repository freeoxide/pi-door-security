use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create command status enum
        manager
            .create_type(
                Type::create()
                    .as_enum(CommandStatus::Enum)
                    .values([
                        CommandStatus::Pending,
                        CommandStatus::Sent,
                        CommandStatus::Acked,
                        CommandStatus::Failed,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Commands::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Commands::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Commands::ClientId).uuid().not_null())
                    .col(ColumnDef::new(Commands::IssuedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(Commands::TsIssued)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Commands::Command).string().not_null())
                    .col(ColumnDef::new(Commands::Params).json_binary())
                    .col(
                        ColumnDef::new(Commands::Status)
                            .enumeration(CommandStatus::Enum, [
                                CommandStatus::Pending,
                                CommandStatus::Sent,
                                CommandStatus::Acked,
                                CommandStatus::Failed,
                            ])
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Commands::TsUpdated)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Commands::Error).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_commands_client_id")
                            .from(Commands::Table, Commands::ClientId)
                            .to(Clients::Table, Clients::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_commands_issued_by")
                            .from(Commands::Table, Commands::IssuedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on client_id
        manager
            .create_index(
                Index::create()
                    .name("idx_commands_client_id")
                    .table(Commands::Table)
                    .col(Commands::ClientId)
                    .to_owned(),
            )
            .await?;

        // Create index on issued_by
        manager
            .create_index(
                Index::create()
                    .name("idx_commands_issued_by")
                    .table(Commands::Table)
                    .col(Commands::IssuedBy)
                    .to_owned(),
            )
            .await?;

        // Create index on status for polling queries
        manager
            .create_index(
                Index::create()
                    .name("idx_commands_status")
                    .table(Commands::Table)
                    .col(Commands::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Commands::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(CommandStatus::Enum).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Commands {
    Table,
    Id,
    ClientId,
    IssuedBy,
    TsIssued,
    Command,
    Params,
    Status,
    TsUpdated,
    Error,
}

#[derive(DeriveIden)]
enum CommandStatus {
    #[sea_orm(iden = "command_status")]
    Enum,
    Pending,
    Sent,
    Acked,
    Failed,
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
