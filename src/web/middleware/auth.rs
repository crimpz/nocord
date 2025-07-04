use super::set_token_cookie;
use crate::crypt::token::{Token, validate_web_token};
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::user::{UserBmc, UserForAuth};
use crate::web::error::{Error, Result};
use crate::web::middleware::AUTH_TOKEN;
use async_trait::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::Request;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tower_cookies::{Cookie, Cookies};
use tracing::debug;

type CtxExtResult = core::result::Result<Ctx, CtxExtError>;

#[derive(Clone, Serialize, Debug)]
pub enum CtxExtError {
    TokenNotInCookie,
    TokenWrongFormat,

    UserNotFound,
    ModelAccessError(String),
    FailValidate,
    CannotSetTokenCookie,

    CtxNotInRequestExt,
    CtxCreateFail(String),
}

pub async fn mw_ctx_require(
    ctx: Result<Ctx>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response> {
    debug!("{:<12} - mw_ctx_require - {ctx:?}", "MIDDLEWARE");
    ctx?;
    let response = next.run(req).await;
    Ok(response)
}

pub async fn mw_ctx_resolve(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response> {
    debug!("{:<12} - mw_ctx_resolve", "MIDDLEWARE");

    let ctx_ext_result = _ctx_resolve(State(mm), &cookies).await;
    debug!("mw_ctx_resolve - ctx result: {:?}", ctx_ext_result);

    if ctx_ext_result.is_err() && !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie)) {
        cookies.remove(Cookie::from(AUTH_TOKEN))
    }

    req.extensions_mut().insert(ctx_ext_result);
    Ok(next.run(req).await)
}

async fn _ctx_resolve(mm: State<ModelManager>, cookies: &Cookies) -> CtxExtResult {
    debug!("ctx_resolve - cookies: {:?}", cookies);
    let token = cookies
        .get(AUTH_TOKEN)
        .map(|c| c.value().to_string())
        .ok_or(CtxExtError::TokenNotInCookie)?;

    let token: Token = token.parse().map_err(|_| CtxExtError::TokenWrongFormat)?;

    debug!("ctx_resolve - token parse result: {:?}", token);

    let user: UserForAuth = UserBmc::first_by_username(&Ctx::root_ctx(), &mm, &token.ident)
        .await
        .map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?
        .ok_or(CtxExtError::UserNotFound)?;

    validate_web_token(&token, &user.token_salt.to_string())
        .map_err(|_| CtxExtError::FailValidate)?;

    let exp_dt: DateTime<Utc> = token.exp.parse().map_err(|_| CtxExtError::FailValidate)?;
    let now = Utc::now();

    let time_remaining = exp_dt - now;

    if time_remaining < Duration::seconds(30) {
        set_token_cookie(cookies, &user.username, &user.token_salt.to_string())
            .map_err(|_| CtxExtError::CannotSetTokenCookie)?;
    }

    debug!("ctx_resolver completed");
    Ctx::new(user.id).map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
        debug!("{:<12} - Ctx", "EXTRACTOR");

        parts
            .extensions
            .get::<CtxExtResult>()
            .ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
            .clone()
            .map_err(Error::CtxExt)
    }
}
