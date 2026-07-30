use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subtask {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
}

// 前后端共享核心结构 (增加了 subtasks 嵌套)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub subtasks: Vec<Subtask>,
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    let user = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();

    // 1. 查询所有主任务
    let todos_rows = sqlx::query_as::<_, (Uuid, String, bool, DateTime<Utc>, Uuid)>(
        "SELECT id, title, completed, created_at, user_id FROM todos WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 2. 查询所有子任务
    let subtasks_rows = sqlx::query_as::<_, (Uuid, Uuid, String, bool, DateTime<Utc>, Uuid)>(
        "SELECT id, todo_id, title, completed, created_at, user_id FROM subtasks WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 3. 在 Rust 内存中组装（避免低效的 N+1 查询）
    let mut todos = todos_rows
        .into_iter()
        .map(|(id, title, completed, created_at, user_id)| Todo {
            id,
            title,
            completed,
            created_at,
            user_id,
            subtasks: vec![],
        })
        .collect::<Vec<_>>();

    for (id, todo_id, title, completed, created_at, user_id) in subtasks_rows {
        if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
            todo.subtasks.push(Subtask {
                id,
                todo_id,
                title,
                completed,
                created_at,
                user_id,
            });
        }
    }

    Ok(todos)
}

#[server]
pub async fn add_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("INSERT INTO todos (id, title, user_id) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(title)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn toggle_todo(id: Uuid) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET completed = NOT completed WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn delete_todo(id: Uuid) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    // 由于在 main.rs 建表时加了 ON DELETE CASCADE，此处的删除会自动清理相关的 subtasks
    sqlx::query("DELETE FROM todos WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn update_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET title = $1 WHERE id = $2 AND user_id = $3")
        .bind(title)
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// ── 子任务 API ────────────────────────────────────────

#[server]
pub async fn add_subtask(id: Uuid, todo_id: Uuid, title: String) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Subtask title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("INSERT INTO subtasks (id, todo_id, title, user_id) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(todo_id)
        .bind(title)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn toggle_subtask(id: Uuid) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE subtasks SET completed = NOT completed WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn delete_subtask(id: Uuid) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("DELETE FROM subtasks WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn update_subtask(id: Uuid, title: String) -> Result<(), ServerFnError> {
    let user = require_auth().await?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Subtask title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE subtasks SET title = $1 WHERE id = $2 AND user_id = $3")
        .bind(title)
        .bind(id)
        .bind(user.id)
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
