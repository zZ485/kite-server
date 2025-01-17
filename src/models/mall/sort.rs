use sqlx::PgPool;

use crate::error::Result;
use crate::models::mall::Sorts;

pub async fn get_goods_sorts(db: &PgPool) -> Result<Vec<Sorts>> {
    let sorts = sqlx::query_as("SELECT id, title FROM mall.sorts ORDER BY priority;")
        .fetch_all(db)
        .await?;
    Ok(sorts)
}
