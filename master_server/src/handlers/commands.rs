use axum::{  extract::{Path, Query, State},  http::StatusCode,  middleware,
    routing::{get, post, Router},
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    entities::{prelude::*, commands, user_clients, users},
};

#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub command: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListCommandsQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AckCommandRequest {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub id: Uuid,
    pub client_id: Uuid,
    pub issued_by: Uuid,
    pub ts_issued: String,
    pub command: String,
    pub params: Option<serde_json::Value>,
    pub status: commands::CommandStatus,
    pub ts_updated: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<commands::Model> for CommandResponse {
    fn from(cmd: commands::Model) -> Self {
        Self {
            id: cmd.id,
            client_id: cmd.client_id,
            issued_by: cmd.issued_by,
            ts_issued: cmd.ts_issued.to_rfc3339(),
            command: cmd.command,
            params: cmd.params,
            status: cmd.status,
            ts_updated: cmd.ts_updated.to_rfc3339(),
            error: cmd.error,
        }
    }
}

async fn create_command(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
    Json(req): Json<CreateCommandRequest>,
) -> Result<(StatusCode, Json<CommandResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Check client exists
    Clients::find_by_id(client_id)
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Error".to_string(),
            }),
        ))?;

    // Check access for non-admin
    if auth_user.role != users::UserRole::Admin {
        let assignment = UserClients::find()
            .filter(user_clients::Column::UserId.eq(auth_user.id))
            .filter(user_clients::Column::ClientId.eq(client_id))
            .one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

        if assignment.is_none() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".to_string(),
                }),
            ));
        }
    }

    let now = chrono::Utc::now();
    let command = commands::ActiveModel {
        id: Set(Uuid::new_v4()),
        client_id: Set(client_id),
        issued_by: Set(auth_user.id),
        ts_issued: Set(now.into()),
        command: Set(req.command),
        params: Set(req.params.map(sea_orm::prelude::Json::from)),
        status: Set(commands::CommandStatus::Pending),
        ts_updated: Set(now.into()),
        error: Set(None),
    };

    let command = command.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok((StatusCode::CREATED, Json(command.into())))
}

async fn list_commands(
    State(state): State<AppState>,
    Path(client_id): Path<Uuid>,
    Query(query): Query<ListCommandsQuery>,
) -> Result<Json<Vec<CommandResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let mut q = Commands::find().filter(commands::Column::ClientId.eq(client_id));

    if let Some(status) = query.status {
        let status_enum = match status.as_str() {
            "pending" => commands::CommandStatus::Pending,
            "sent" => commands::CommandStatus::Sent,
            "acked" => commands::CommandStatus::Acked,
            "failed" => commands::CommandStatus::Failed,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Invalid status".to_string(),
                    }),
                ))
            }
        };
        q = q.filter(commands::Column::Status.eq(status_enum));
    }

    let commands = q.all(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(Json(commands.into_iter().map(|c| c.into()).collect()))
}

async fn ack_command(
    State(state): State<AppState>,
    Path((client_id, cmd_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AckCommandRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let command = Commands::find_by_id(cmd_id)
        .filter(commands::Column::ClientId.eq(client_id))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Error".to_string(),
            }),
        ))?;

    let mut command: commands::ActiveModel = command.into();
    command.status = Set(if req.success {
        commands::CommandStatus::Acked
    } else {
        commands::CommandStatus::Failed
    });
    command.error = Set(req.error);
    command.ts_updated = Set(chrono::Utc::now().into());

    command.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/:client_id/commands",
            post(create_command),
        )
        .route("/:client_id/commands", get(list_commands))
        .route("/:client_id/commands/:cmd_id/ack", post(ack_command))
}
