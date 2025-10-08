use axum::{  extract::{Path, Query, State},  http::StatusCode,  middleware,
    routing::{delete, get, patch, post, Router},
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    entities::{prelude::*, clients, user_clients, users},
};

#[derive(Debug, Deserialize)]
pub struct CreateClientRequest {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNetworkRequest {
    pub eth0_ip: Option<String>,
    pub wlan0_ip: Option<String>,
    pub service_port: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AssignUserRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RegisterClientRequest {
    pub provision_key: Uuid,
    pub eth0_ip: Option<String>,
    pub wlan0_ip: Option<String>,
    pub service_port: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ClientResponse {
    pub id: Uuid,
    pub label: String,
    pub eth0_ip: Option<String>,
    pub wlan0_ip: Option<String>,
    pub service_port: Option<i32>,
    pub status: clients::ClientStatus,
    pub last_seen_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateClientResponse {
    pub id: Uuid,
    pub provision_key: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RegisterClientResponse {
    pub client_id: Uuid,
    pub api_token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<clients::Model> for ClientResponse {
    fn from(client: clients::Model) -> Self {
        Self {
            id: client.id,
            label: client.label,
            eth0_ip: client.eth0_ip,
            wlan0_ip: client.wlan0_ip,
            service_port: client.service_port,
            status: client.status,
            last_seen_at: client.last_seen_at.map(|dt| dt.to_rfc3339()),
            created_at: client.created_at.to_rfc3339(),
        }
    }
}

async fn create_client(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Json(req): Json<CreateClientRequest>,
) -> Result<(StatusCode, Json<CreateClientResponse>), (StatusCode, Json<ErrorResponse>)> {
    let client_id = Uuid::new_v4();
    let provision_key = Uuid::now_v7();

    let client = clients::ActiveModel {
        id: Set(client_id),
        label: Set(req.label),
        provision_key: Set(provision_key),
        eth0_ip: Set(None),
        wlan0_ip: Set(None),
        service_port: Set(None),
        status: Set(clients::ClientStatus::Unknown),
        last_seen_at: Set(None),
        created_at: Set(chrono::Utc::now().into()),
    };

    client.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create client".to_string(),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateClientResponse {
            id: client_id,
            provision_key,
        }),
    ))
}

async fn list_clients(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<ClientResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let clients = if auth_user.role == users::UserRole::Admin {
        // Admin sees all clients
        Clients::find().all(&state.db).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?
    } else {
        // Users see only assigned clients
        let assignments = UserClients::find()
            .filter(user_clients::Column::UserId.eq(auth_user.id))
            .all(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Database error".to_string(),
                    }),
                )
            })?;

        let client_ids: Vec<Uuid> = assignments.iter().map(|a| a.client_id).collect();

        Clients::find()
            .filter(clients::Column::Id.is_in(client_ids))
            .all(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Database error".to_string(),
                    }),
                )
            })?
    };

    Ok(Json(clients.into_iter().map(|c| c.into()).collect()))
}

async fn get_client(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
) -> Result<Json<ClientResponse>, (StatusCode, Json<ErrorResponse>)> {
    let client = Clients::find_by_id(client_id)
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

    // Check access
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

    Ok(Json(client.into()))
}

async fn update_network(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
    Json(req): Json<UpdateNetworkRequest>,
) -> Result<Json<ClientResponse>, (StatusCode, Json<ErrorResponse>)> {
    let client = Clients::find_by_id(client_id)
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

    let mut client: clients::ActiveModel = client.into();

    if let Some(eth0_ip) = req.eth0_ip {
        client.eth0_ip = Set(Some(eth0_ip));
    }

    if let Some(wlan0_ip) = req.wlan0_ip {
        client.wlan0_ip = Set(Some(wlan0_ip));
    }

    if let Some(service_port) = req.service_port {
        client.service_port = Set(Some(service_port));
    }

    let client = client.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(Json(client.into()))
}

async fn delete_client(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let client = Clients::find_by_id(client_id)
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

    let client: clients::ActiveModel = client.into();
    client.delete(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn assign_user(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
    Json(req): Json<AssignUserRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check if client exists
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

    // Check if user exists
    Users::find_by_id(req.user_id)
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

    // Create assignment
    let assignment = user_clients::ActiveModel {
        user_id: Set(req.user_id),
        client_id: Set(client_id),
    };

    assignment.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn unassign_user(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path((client_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let assignment = UserClients::find()
        .filter(user_clients::Column::UserId.eq(user_id))
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
        })?
        .ok_or((StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Error".to_string(),
            }),
        ))?;

    let assignment: user_clients::ActiveModel = assignment.into();
    assignment.delete(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn register_client(
    State(state): State<AppState>,
    Json(req): Json<RegisterClientRequest>,
) -> Result<Json<RegisterClientResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find client by provision key
    let client = Clients::find()
        .filter(clients::Column::ProvisionKey.eq(req.provision_key))
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

    // Update network info and invalidate provision key
    let mut client: clients::ActiveModel = client.into();
    client.eth0_ip = Set(req.eth0_ip);
    client.wlan0_ip = Set(req.wlan0_ip);
    client.service_port = Set(req.service_port);
    client.provision_key = Set(Uuid::nil()); // Invalidate provision key

    let client = client.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    // Generate client API token (using session system with special user ID)
    let token = hex::encode(rand::random::<[u8; 32]>());

    // In a real implementation, we'd store client tokens separately
    // For MVP, we'll just return a generated token

    Ok(Json(RegisterClientResponse {
        client_id: client.id,
        api_token: token,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_client))
        .route(
            "/",
            post(create_client)
                .get(list_clients),
        )
        .route(
            "/:id",
            get(get_client)
                .delete(delete_client),
        )
        .route(
            "/:id/network",
            patch(update_network),
        )
        .route(
            "/:id/assign",
            post(assign_user),
        )
        .route(
            "/:id/assign/:user_id",
            delete(unassign_user),
        )
}
