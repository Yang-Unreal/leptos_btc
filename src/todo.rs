use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// 前后端共享核心结构 (移除了第三方 Store 宏)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    let _auth = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();

    // 【配合 O(1) 策略】改为 ASC 升序查询。
    // 旧数据在前，新数据追加至 Vec 尾部 (push 是均摊 O(1))，
    // 再利用前端 CSS flex-col-reverse 天然逆序展示，将完美实现插入效率最大化。
    let rows = sqlx::query_as::<_, (Uuid, String, bool, DateTime<Utc>)>(
        "SELECT id, title, completed, created_at FROM todos ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|(id, title, completed, created_at)| Todo {
            id,
            title,
            completed,
            created_at,
        })
        .collect())
}

#[server]
pub async fn add_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
    let _auth = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("INSERT INTO todos (id, title) VALUES ($1, $2)")
        .bind(id)
        .bind(title)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn toggle_todo(id: Uuid) -> Result<(), ServerFnError> {
    let _auth = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET completed = NOT completed WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn delete_todo(id: Uuid) -> Result<(), ServerFnError> {
    let _auth = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn update_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
    let _auth = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET title = $1 WHERE id = $2")
        .bind(title)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// ── Auth guard ──────────────────────────────────────────
#[cfg(feature = "ssr")]
async fn require_auth() -> Result<crate::auth::User, ServerFnError> {
    use crate::auth::AuthSession;
    use leptos_axum::extract;

    let auth: AuthSession = extract().await?;
    auth.user
        .clone()
        .ok_or_else(|| ServerFnError::new("Unauthorized"))
}
