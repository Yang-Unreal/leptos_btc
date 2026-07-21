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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i32,                   // 主键，对应数据库 SERIAL；用 i32 匹配 Postgres 的 int4
    pub title: String,             // 标题
    pub completed: bool,           // 是否完成
    pub created_at: DateTime<Utc>, // 创建时间（UTC）
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    // 从上下文里"取出"数据库连接池。
    // 【这行和 main.rs 如何配对】：main.rs 里用 provide_context(pool) 把连接池放进了
    //   上下文；这里 expect_context::<sqlx::PgPool>() 就是把它取回来。<...> 指定要取的类型。
    //   expect_ 版本表示"取不到就 panic"——因为如果连接池没提供，说明是程序配置错误。
    //   注意：这段代码只在服务器上执行，所以能安全地碰数据库。
    let pool = expect_context::<sqlx::PgPool>();
    // 执行查询并把每行映射成一个元组 (i32, String, bool, DateTime<Utc>)。
    // 【为什么用 query_as::<_, (元组类型)>】：query_as 会按列顺序把查询结果转换成指定类型。
    //   这里用元组接收 4 列；`_` 让编译器自行推断数据库驱动类型。
    let rows = sqlx::query_as::<_, (i32, String, bool, DateTime<Utc>)>(
        // ORDER BY created_at DESC：最新创建的排在最前面。
        "SELECT id, title, completed, created_at FROM todos ORDER BY created_at DESC",
    )
    .fetch_all(&pool) // 取回所有匹配的行，得到 Vec<元组>
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 把 Vec<元组> 转换成 Vec<Todo> 并返回。
    // 【Rust 基础语法讲解：Ok() 构造器和返回类型】
    // Ok(rows.into_iter()...collect()) 用 Ok() 把成功值包装成 Result 的 Ok 变体。
    // 函数返回类型是 Result<Vec<Todo>, ServerFnError>，所以返回 Ok(Vec<Todo>) 是匹配的。
    Ok(rows
        .into_iter() // 把 Vec 变成迭代器，逐个消费其中的元组
        .map(|(id, title, completed, created_at)| Todo {
            id,
            title,
            completed,
            created_at,
        })
        .collect()) // 把迭代器重新收集成 Vec<Todo>
}

// -------------------- 服务器函数 2：新增一条待办 --------------------
#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    let title = title.trim().to_string();
    // 服务器端也要校验，不能只信任前端。
    // 【为什么服务器必须再校验一遍】：前端校验能被绕过（有人可以直接对 /api 发请求）。
    //   把"标题不能为空"作为服务器规则，才是真正的数据保护。空标题就返回错误。
    if title.is_empty() {
        // 【Rust 基础语法讲解：return 关键字】
        // return Err(...) 显式地从函数返回一个错误值。
        // return 会立即退出当前函数，后面的代码不会执行。
        // 这里创建了一个新的 ServerFnError，内容是 "Title cannot be empty"。
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    // INSERT 语句里的 $1 是"占位符/参数绑定"。
    // 【为什么用 $1 而不是把 title 拼进字符串】：这能防止 SQL 注入攻击。用 .bind(title)
    //   把值作为"参数"传给数据库，数据库会把它当纯数据处理，即使标题里含有 SQL 语法
    //   （如 '); DROP TABLE todos;--）也不会被当成命令执行。绝不要用字符串拼接拼 SQL。
    sqlx::query("INSERT INTO todos (title) VALUES ($1)")
        .bind(title) // 把 title 绑定到 $1
        .execute(&pool) // execute 用于会改数据、不需要返回行的语句（INSERT/UPDATE/DELETE）
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    // 返回 Ok(()) 表示"成功，但没有有意义的返回值"。() 是 Rust 的"空元组/单位类型"。
    Ok(())
}

// -------------------- 服务器函数 3：切换完成状态 --------------------
#[server]
pub async fn toggle_todo(id: i32) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    // SET completed = NOT completed：把布尔值取反（完成↔未完成）。
    // 【为什么在 SQL 里取反，而不是先查出来再写回】：一条语句原子完成，既少一次往返，
    //   也避免"读—改—写"之间的竞态问题。
    sqlx::query("UPDATE todos SET completed = NOT completed WHERE id = $1")
        .bind(id) // WHERE id = $1，只改这一条
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// -------------------- 服务器函数 4：删除一条待办 --------------------
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

// -------------------- 服务器函数 5：修改标题 --------------------
#[server]
// 接收两个参数：要改哪条(id) 和 新标题(title)。
pub async fn update_todo(id: i32, title: String) -> Result<(), ServerFnError> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    // 这里有【两个占位符】$1、$2。
    // 【为什么要注意 bind 的顺序】：.bind 是按顺序对应 $1、$2 的——第一个 bind(title) 对应
    //   $1（SET title = $1），第二个 bind(id) 对应 $2（WHERE id = $2）。顺序写反会改错数据。
    sqlx::query("UPDATE todos SET title = $1 WHERE id = $2")
        .bind(title) // → $1
        .bind(id) // → $2
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
