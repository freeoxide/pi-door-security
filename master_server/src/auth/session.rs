use anyhow::Result;
use chrono::{Duration, Utc};
use rand::Rng;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::{prelude::*, sessions};

/// Generate a secure random token
fn generate_token() -> String {
    let random_bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(random_bytes)
}

/// Create a new session for a user
pub async fn create_session(
    db: &DatabaseConnection,
    user_id: Uuid,
    ttl_hours: i64,
) -> Result<(String, chrono::DateTime<Utc>)> {
    let token = generate_token();
    let now = Utc::now();
    let expires_at = now + Duration::hours(ttl_hours);

    let session = sessions::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        token: Set(token.clone()),
        expires_at: Set(expires_at.into()),
        created_at: Set(now.into()),
        revoked_at: Set(None),
    };

    session.insert(db).await?;

    Ok((token, expires_at))
}

/// Verify a session token and return the user_id if valid
pub async fn verify_session(db: &DatabaseConnection, token: &str) -> Result<Option<Uuid>> {
    let session = Sessions::find()
        .filter(sessions::Column::Token.eq(token))
        .filter(sessions::Column::RevokedAt.is_null())
        .one(db)
        .await?;

    if let Some(session) = session {
        let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
        if session.expires_at > now {
            return Ok(Some(session.user_id));
        }
    }

    Ok(None)
}

/// Revoke a session token
pub async fn revoke_session(db: &DatabaseConnection, token: &str) -> Result<()> {
    let session = Sessions::find()
        .filter(sessions::Column::Token.eq(token))
        .one(db)
        .await?;

    if let Some(session) = session {
        let mut session: sessions::ActiveModel = session.into();
        session.revoked_at = Set(Some(Utc::now().into()));
        session.update(db).await?;
    }

    Ok(())
}
