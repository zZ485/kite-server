use actix_web::{get, web, HttpResponse};

use crate::error::Result;
use crate::models::notice::Notice;
use crate::services::response::ApiResponse;
use crate::services::AppState;

#[get("/notice")]
pub async fn get_notices(app: web::Data<AppState>) -> Result<HttpResponse> {
    let notices = Notice::get(&app.pool).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::normal(notices)))
}
