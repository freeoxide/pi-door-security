use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post, Router},
    Extension, Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::{self, middleware::AuthUser},
    entities::{prelude::*, users},
};

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: users::UserRole,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub role: Option<users::UserRole>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: users::UserRole,
    pub otp_enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<users::Model> for UserResponse {
    fn from(user: users::Model) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            otp_enabled: user.otp_enabled,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

async fn create_user(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Check if username already exists
    let existing = Users::find()
        .filter(users::Column::Username.eq(&req.username))
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Username already exists".to_string(),
            }),
        ));
    }

    // Hash password
    let password_hash = auth::hash_password(&req.password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Password hashing failed".to_string(),
            }),
        )
    })?;

    // Create user
    let user = users::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(req.username),
        password_hash: Set(password_hash),
        role: Set(req.role),
        otp_secret: Set(None),
        otp_enabled: Set(false),
        created_at: Set(Utc::now().into()),
    };

    let user = user.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create user".to_string(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(user.into())))
}

async fn list_users(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let users = Users::find().all(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Database error".to_string(),
            }),
        )
    })?;

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

async fn update_user(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = Users::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".to_string(),
            }),
        ))?;

    let mut user: users::ActiveModel = user.into();

    if let Some(username) = req.username {
        user.username = Set(username);
    }

    if let Some(password) = req.password {
        let password_hash = auth::hash_password(&password).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Password hashing failed".to_string(),
                }),
            )
        })?;
        user.password_hash = Set(password_hash);
    }

    if let Some(role) = req.role {
        user.role = Set(role);
    }

    let user = user.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update user".to_string(),
            }),
        )
    })?;

    Ok(Json(user.into()))
}

async fn delete_user(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user = Users::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".to_string(),
            }),
        ))?;

    let user: users::ActiveModel = user.into();
    user.delete(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to delete user".to_string(),
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_user))
        .route("/", get(list_users))
        .route("/:id", patch(update_user))
        .route("/:id", delete(delete_user))
        
}
