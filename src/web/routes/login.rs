use crate::crypt::EncryptContent;
use crate::crypt::pwd;
use crate::crypt::token::generate_web_token;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::user::UserBmc;
use crate::model::user::UserForLogin;
use crate::web::error::{Error, Result};
use crate::web::middleware::{remove_token_cookie, set_token_cookie};
use axum::debug_handler;
use axum::extract::State;
use axum::response::Json as AxumJson;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use tower_cookies::Cookies;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct LoginPayload {
    username: String,
    pwd: String,
}

#[derive(Debug, Deserialize)]
struct CreatePayload {
    username: String,
    pwd: String,
}

#[derive(Debug, Deserialize)]
struct LogoffPayload {
    logoff: bool,
}

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/login", post(api_login_handler))
        .route("/api/logoff", post(api_logoff_handler))
        .route("/api/create_user", post(api_create_user))
        .with_state(mm)
}

async fn api_create_user(
    State(mm): State<ModelManager>,
    Json(payload): Json<CreatePayload>,
) -> Result<AxumJson<Value>> {
    debug!("{:<12} - api_create_user_handler", "HANDLER");
    let CreatePayload {
        username,
        pwd: pwd_clear,
    } = payload;

    UserBmc::create_user(&mm, &username, &pwd_clear).await?;

    let body = AxumJson(json!({
        "result": {
            "success": true
        }
    }));

    Ok(body)
}

async fn api_logoff_handler(
    cookies: Cookies,
    Json(payload): Json<LogoffPayload>,
) -> Result<AxumJson<Value>> {
    debug!("{:<12} - api_logoff_handler", "HANDLER");
    let should_logoff = payload.logoff;

    if should_logoff {
        remove_token_cookie(&cookies)?;
    }

    let body = AxumJson(json!({
        "result": {
            "logged_off": should_logoff
        }
    }));

    Ok(body)
}

#[debug_handler]
async fn api_login_handler(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    Json(payload): Json<LoginPayload>,
) -> Result<AxumJson<Value>> {
    debug!("{:<12} - api_login_handler", "HANDLER");

    let LoginPayload {
        username,
        pwd: pwd_clear,
    } = payload;

    let root_ctx = Ctx::root_ctx();

    let user: UserForLogin = UserBmc::first_by_username(&root_ctx, &mm, &username)
        .await?
        .ok_or(Error::LoginFailUsernameNotFound)?;

    let user_id = user.id;

    let Some(pwd) = user.pwd else {
        return Err(Error::LoginFailUserHasNoPwd { user_id });
    };

    pwd::validate_pwd(
        &EncryptContent {
            salt: user.pwd_salt.to_string(),
            content: pwd_clear.clone(),
        },
        &pwd,
    )
    .map_err(|_| Error::LoginFailPwdNotMatching { user_id })?;

    let token = generate_web_token(&user.username, &user.token_salt.to_string())?;
    let token_str = token.to_string();

    set_token_cookie(&cookies, &user.username, &user.token_salt.to_string())?;

    let body = AxumJson(json!({
        "result": {
            "success": true,
            "id": user_id,
            "username": username,
            "token": token_str
        }
    }));

    Ok(body)
}
