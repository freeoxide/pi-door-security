use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{post, Router},
    Json, Extension,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::{self, middleware::AuthUser},
    entities::{prelude::*, users},
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub otp_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct OtpSetupResponse {
    pub otpauth_uri: String,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct OtpVerifyRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct OtpVerifyResponse {
    pub otp_enabled: bool,
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = Users::find()
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
        })?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".to_string(),
            }),
        ))?;

    let valid = auth::verify_password(&req.password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Password verification failed".to_string(),
            }),
        )
    })?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".to_string(),
            }),
        ));
    }

    if user.otp_enabled {
        let otp_code = req.otp_code.ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "OTP code required".to_string(),
            }),
        ))?;

        let otp_secret = user.otp_secret.as_ref().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "OTP secret not found".to_string(),
            }),
        ))?;

        let valid_otp = auth::verify_otp_code(otp_secret, &otp_code).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "OTP verification failed".to_string(),
                }),
            )
        })?;

        if !valid_otp {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid OTP code".to_string(),
                }),
            ));
        }
    }

    let (token, expires_at) = auth::create_session(&state.db, user.id, state.config.token_ttl_hours)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create session".to_string(),
                }),
            )
        })?;

    Ok(Json(LoginResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
    }))
}

async fn logout(
    State(_state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

async fn otp_setup(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<OtpSetupResponse>, (StatusCode, Json<ErrorResponse>)> {
    let secret = auth::generate_otp_secret();
    let uri = auth::get_otp_uri(&secret, &auth_user.username, "Pi Door Security");

    let user = Users::find_by_id(auth_user.id)
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
    user.otp_secret = Set(Some(secret.clone()));
    user.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update user".to_string(),
            }),
        )
    })?;

    Ok(Json(OtpSetupResponse {
        otpauth_uri: uri,
        secret,
    }))
}

async fn otp_verify(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<OtpVerifyRequest>,
) -> Result<Json<OtpVerifyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = Users::find_by_id(auth_user.id)
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

    let otp_secret = user.otp_secret.ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "OTP not set up".to_string(),
        }),
    ))?;

    let valid = auth::verify_otp_code(&otp_secret, &req.code).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "OTP verification failed".to_string(),
            }),
        )
    })?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid OTP code".to_string(),
            }),
        ));
    }

    let mut user: users::ActiveModel = user.into();
    user.otp_enabled = Set(true);
    user.update(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to enable OTP".to_string(),
            }),
        )
    })?;

    Ok(Json(OtpVerifyResponse { otp_enabled: true }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/otp/setup", post(otp_setup))
        .route("/otp/verify", post(otp_verify))
}
