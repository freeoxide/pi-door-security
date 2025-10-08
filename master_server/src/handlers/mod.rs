pub mod auth;
pub mod users;
pub mod clients;
pub mod commands;
pub mod telemetry;

pub use auth::router as auth_router;
pub use users::router as users_router;
pub use clients::router as clients_router;
pub use commands::router as commands_router;
pub use telemetry::router as telemetry_router;
