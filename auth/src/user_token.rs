use crate::error::AuthError;
use base64::prelude::*;
use platz_db::UserToken;
use rand::random;
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use uuid::Uuid;

const USER_TOKEN_SECRET_BYTES: usize = 32;

#[derive(Debug)]
pub struct UserTokenInfo {
    pub token_id: Uuid,
    pub secret_hash: String,
    pub token_value: String,
}

fn calculate_secret_hash(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    let secret_sha256 = hasher.finalize();
    BASE64_URL_SAFE_NO_PAD.encode(secret_sha256)
}

pub async fn generate_user_token() -> Result<UserTokenInfo, AuthError> {
    let token_id = Uuid::new_v4();
    let secret = BASE64_URL_SAFE_NO_PAD
        .encode(spawn_blocking(random::<[u8; USER_TOKEN_SECRET_BYTES]>).await?);
    let secret_hash = calculate_secret_hash(&secret);
    Ok(UserTokenInfo {
        secret_hash,
        token_value: format!(
            "{}.{}",
            &BASE64_URL_SAFE_NO_PAD.encode(token_id.as_bytes()),
            secret
        ),
        token_id,
    })
}

pub(crate) async fn validate_user_token(user_token: String) -> Result<UserToken, AuthError> {
    let Some((token_id, secret)) = user_token.split_once('.') else { return Err(AuthError::UserTokenAuthenticationError("Invalid token".to_string())); };
    let token_uuid = Uuid::from_bytes(
        BASE64_URL_SAFE_NO_PAD
            .decode(token_id.as_bytes())
            .ok()
            .and_then(|decoded| decoded.as_slice().try_into().ok())
            .ok_or_else(|| AuthError::UserTokenAuthenticationError("Invalid token".to_string()))?,
    );
    let user_token = UserToken::find(token_uuid).await?;
    let secret_hash = calculate_secret_hash(secret);
    if secret_hash != user_token.secret_hash {
        return Err(AuthError::UserTokenAuthenticationError(
            "Illegal token".into(),
        ));
    }
    Ok(user_token)
}
