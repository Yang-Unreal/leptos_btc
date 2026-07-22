// ============================================================================
// todo.rs —— 数据模型 + 服务器函数(server functions)
// ----------------------------------------------------------------------------
// 这个文件是"前后端的桥梁"，包含两部分：
//   1) Todo 结构体：一条待办的数据形状，前后端【共用同一个定义】。
//   2) 5 个带 #[server] 的函数：增删改查逻辑。它们的函数体只在服务器上运行，
//      但可以在客户端代码里【像普通异步函数一样调用】——Leptos 会自动把这次调用
//      变成一次到服务器的 HTTP 请求。这就是"全栈同构"最神奇的地方。
// ============================================================================

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    let rows = sqlx::query_as::<_, (Uuid, String, bool, DateTime<Utc>)>(
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

// -------------------- 服务器函数 2：新增一条待办 --------------------
#[server]
pub async fn add_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
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

// -------------------- 服务器函数 3：切换完成状态 --------------------
#[server]
pub async fn toggle_todo(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("UPDATE todos SET completed = NOT completed WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// -------------------- 服务器函数 4：删除一条待办 --------------------
#[server]
pub async fn delete_todo(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// -------------------- 服务器函数 5：修改标题 --------------------
#[server]
pub async fn update_todo(id: Uuid, title: String) -> Result<(), ServerFnError> {
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
