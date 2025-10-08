use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create role enum
        manager
            .create_type(
                Type::create()
                    .as_enum(UserRole::Enum)
                    .values([UserRole::Admin, UserRole::User])
                    .to_owned(),
            )
            .await?;

        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::Role)
                            .enumeration(UserRole::Enum, [UserRole::Admin, UserRole::User])
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::OtpSecret).string())
                    .col(
                        ColumnDef::new(Users::OtpEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on username
        manager
            .create_index(
                Index::create()
                    .name("idx_users_username")
                    .table(Users::Table)
                    .col(Users::Username)
                    .to_owned(),
            )
            .await?;

        // Create index on role
        manager
            .create_index(
                Index::create()
                    .name("idx_users_role")
                    .table(Users::Table)
                    .col(Users::Role)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(UserRole::Enum).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
    Role,
    OtpSecret,
    OtpEnabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserRole {
    #[sea_orm(iden = "user_role")]
    Enum,
    Admin,
    User,
}
