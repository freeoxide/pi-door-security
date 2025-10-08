use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{app::AppState, entities::{prelude::*, users}};

/// User context extracted from authentication
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: uuid::Uuid,
    pub username: String,
    pub role: users::UserRole,
}

/// Extract bearer token from Authorization header
fn extract_bearer_token(req: &Request) -> Option<String> {
    let auth_header = req.headers().get("authorization")?.to_str().ok()?;
    let token = auth_header.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

/// Middleware to require authentication
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req).ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = crate::auth::verify_session(&state.db, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Fetch user details
    let user = Users::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_user = AuthUser {
        id: user.id,
        username: user.username,
        role: user.role,
    };

    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}

/// Middleware to require admin role
pub async fn require_admin(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req).ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = crate::auth::verify_session(&state.db, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = Users::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.role != users::UserRole::Admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let auth_user = AuthUser {
        id: user.id,
        username: user.username,
        role: user.role,
    };

    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}
