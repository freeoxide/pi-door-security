use axum::{
    http::StatusCode,
    middleware,
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
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
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
        ));
    }

    // Hash password
    let password_hash = auth::hash_password(&req.password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Create user
    let user = users::ActiveModel {
        username: Set(req.username),
        password_hash: Set(password_hash),
        role: Set(req.role),
        otp_secret: Set(None),
        otp_enabled: Set(false),
    };

    let user = user.insert(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
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
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
        ))?;

    let mut user: users::ActiveModel = user.into();

    if let Some(username) = req.username {
        user.username = Set(username);
    }

    if let Some(password) = req.password {
        let password_hash = auth::hash_password(&password).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
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
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
        ))?;

    let user: users::ActiveModel = user.into();
    user.delete(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
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
        .route_layer(middleware::from_fn(crate::auth::middleware::require_admin));
}
