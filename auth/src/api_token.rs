use crate::error::AuthError;
use base64::prelude::*;
use platz_db::{BotToken, Identity, UserToken};
use rand::random;
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use uuid::Uuid;

const API_TOKEN_SECRET_BYTES: usize = 32;

#[derive(Debug)]
pub struct ApiTokenInfo {
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

pub async fn generate_api_token() -> Result<ApiTokenInfo, AuthError> {
    let token_id = Uuid::new_v4();
    let secret = BASE64_URL_SAFE_NO_PAD
        .encode(spawn_blocking(random::<[u8; API_TOKEN_SECRET_BYTES]>).await?);
    let secret_hash = calculate_secret_hash(&secret);
    Ok(ApiTokenInfo {
        secret_hash,
        token_value: format!(
            "{}.{}",
            &BASE64_URL_SAFE_NO_PAD.encode(token_id.as_bytes()),
            secret
        ),
        token_id,
    })
}

pub(crate) async fn validate_api_token(api_token: String) -> Result<Identity, AuthError> {
    let Some((token_id, secret)) = api_token.split_once('.') else {
        return Err(AuthError::ApiTokenAuthenticationError("Invalid token"));
    };

    let token_uuid = Uuid::from_bytes(
        BASE64_URL_SAFE_NO_PAD
            .decode(token_id.as_bytes())
            .ok()
            .and_then(|decoded| decoded.as_slice().try_into().ok())
            .ok_or_else(|| AuthError::ApiTokenAuthenticationError("Invalid token"))?,
    );

    let secret_hash = calculate_secret_hash(secret);
    let user_token = UserToken::find(token_uuid).await?;
    let bot_token = BotToken::find(token_uuid).await?;

    if let Some(user_token) = user_token {
        if user_token.secret_hash == secret_hash {
            return Ok(Identity::User(user_token.user_id));
        }
    } else if let Some(bot_token) = bot_token {
        if bot_token.secret_hash == secret_hash {
            return Ok(Identity::Bot(bot_token.bot_id));
        }
    }

    Err(AuthError::ApiTokenAuthenticationError("Illegal token"))
}
