use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i32,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    let rows = sqlx::query_as::<_, (i32, String, bool, DateTime<Utc>)>(
        "SELECT id, title, completed, created_at FROM todos ORDER BY created_at DESC",
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
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("INSERT INTO todos (title) VALUES ($1)")
        .bind(title)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn toggle_todo(id: i32) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET completed = NOT completed WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn delete_todo(id: i32) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server]
pub async fn update_todo(id: i32, title: String) -> Result<(), ServerFnError> {
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
