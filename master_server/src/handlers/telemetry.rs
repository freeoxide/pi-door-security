use axum::{  extract::{Path, Query, State},  http::StatusCode,  middleware,
    routing::{get, post, Router},
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    entities::{prelude::*, clients, events, heartbeats, user_clients, users},
};

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub uptime_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct EventRequest {
    pub level: events::EventLevel,
    pub kind: String,
    pub message: String,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    pub since: Option<String>,
    pub level: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: i64,
    pub client_id: Uuid,
    pub ts: String,
    pub level: events::EventLevel,
    pub kind: String,
    pub message: String,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ClientStatusResponse {
    pub status: clients::ClientStatus,
    pub last_seen_at: Option<String>,
    pub service_port: Option<i32>,
    pub eth0_ip: Option<String>,
    pub wlan0_ip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<events::Model> for EventResponse {
    fn from(event: events::Model) -> Self {
        Self {
            id: event.id,
            client_id: event.client_id,
            ts: event.ts.to_rfc3339(),
            level: event.level,
            kind: event.kind,
            message: event.message,
            meta: event.meta,
        }
    }
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(client_id): Path<Uuid>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Update client status
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

    let now = chrono::Utc::now();
    let mut client: clients::ActiveModel = client.into();
    client.status = Set(clients::ClientStatus::Online);
    client.last_seen_at = Set(Some(now.into()));
    client.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    // Record heartbeat
    let heartbeat = heartbeats::ActiveModel {
        id: Set(0),
        client_id: Set(client_id),
        ts: Set(now.into()),
        uptime_ms: Set(req.uptime_ms),
    };

    heartbeat.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_event(
    State(state): State<AppState>,
    Path(client_id): Path<Uuid>,
    Json(req): Json<EventRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let event = events::ActiveModel {
        id: Set(0),
        client_id: Set(client_id),
        ts: Set(chrono::Utc::now().into()),
        level: Set(req.level),
        kind: Set(req.kind),
        message: Set(req.message),
        meta: Set(req.meta.map(sea_orm::prelude::Json::from)),
    };

    event.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(StatusCode::ACCEPTED)
}

async fn list_events(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<ErrorResponse>)> {
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
            return Err((StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Error".to_string(),
                    }),
                ));
        }
    }

    let mut q = Events::find()
        .filter(events::Column::ClientId.eq(client_id))
        .order_by_desc(events::Column::Ts);

    if let Some(since) = query.since {
        if let Ok(since_dt) = chrono::DateTime::parse_from_rfc3339(&since) {
            q = q.filter(events::Column::Ts.gt(since_dt));
        }
    }

    if let Some(level) = query.level {
        let level_enum = match level.as_str() {
            "info" => events::EventLevel::Info,
            "warn" => events::EventLevel::Warn,
            "error" => events::EventLevel::Error,
            _ => {
                return Err((StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Error".to_string(),
                    }),
                ))
            }
        };
        q = q.filter(events::Column::Level.eq(level_enum));
    }

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    let events = q.all(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error".to_string(),
                }),
            )
        })?;

    Ok(Json(events.into_iter().map(|e| e.into()).collect()))
}

async fn get_status(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(client_id): Path<Uuid>,
) -> Result<Json<ClientStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
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
            return Err((StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Error".to_string(),
                    }),
                ));
        }
    }

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

    Ok(Json(ClientStatusResponse {
        status: client.status,
        last_seen_at: client.last_seen_at.map(|dt| dt.to_rfc3339()),
        service_port: client.service_port,
        eth0_ip: client.eth0_ip,
        wlan0_ip: client.wlan0_ip,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:client_id/heartbeat", post(heartbeat))
        .route("/:client_id/events", post(create_event))
        .route(
            "/:client_id/events",
            get(list_events),
        )
        .route(
            "/:client_id/status",
            get(get_status),
        )
}
