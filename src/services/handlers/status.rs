use actix_web::{get, web, HttpResponse};
use chrono::Local;
use serde::Deserialize;

use crate::bridge::{HostError, RequestFrame, RequestPayload, ResponsePayload};
use crate::error::{ApiError, Result};
use crate::models::CommonError;
use crate::services::response::ApiResponse;
use crate::services::{AppState, JwtToken};

#[get("/status/timestamp")]
pub async fn get_timestamp() -> Result<HttpResponse> {
    let ts = Local::now().timestamp_millis();

    let response = serde_json::json!({
        "ts": ts,
    });
    Ok(HttpResponse::Ok().json(&ApiResponse::normal(response)))
}

#[get("/status/agent")]
pub async fn get_agent_list(app: web::Data<AppState>, token: Option<JwtToken>) -> Result<HttpResponse> {
    let token = token.ok_or_else(|| ApiError::new(CommonError::LoginNeeded))?;
    if !token.is_admin {
        return Err(CommonError::Forbidden.into());
    }

    let agents = &app.agents;
    let response = serde_json::json!({
        "agents": agents.get_client_list().await,
    });
    Ok(HttpResponse::Ok().json(ApiResponse::normal(response)))
}

#[derive(Deserialize)]
pub struct PingRequest {
    msg: Option<String>,
}

#[get("/status/agent/ping")]
pub async fn ping_agent(
    params: web::Query<PingRequest>,
    app: web::Data<AppState>,
) -> Result<HttpResponse> {
    let params = params.into_inner();
    let message = params.msg.unwrap_or_else(|| "Hello world!".to_string());

    let agents = &app.agents;
    let payload = RequestPayload::Ping(message);
    let request = RequestFrame::new(payload);

    let response = agents.request(request).await??;
    if let ResponsePayload::Pong(pong) = response {
        Ok(HttpResponse::Ok().json(ApiResponse::normal(pong)))
    } else {
        Err(ApiError::new(HostError::Mismatched))
    }
}
