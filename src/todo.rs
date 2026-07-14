// ============================================================================
// todo.rs —— 数据模型 + 服务器函数(server functions)
// ----------------------------------------------------------------------------
// 这个文件是“前后端的桥梁”，包含两部分：
//   1) Todo 结构体：一条待办的数据形状，前后端【共用同一个定义】。
//   2) 5 个带 #[server] 的函数：增删改查逻辑。它们的函数体只在服务器上运行，
//      但可以在客户端代码里【像普通异步函数一样调用】——Leptos 会自动把这次调用
//      变成一次到服务器的 HTTP 请求。这就是“全栈同构”最神奇的地方。
// ============================================================================

// chrono 是处理日期时间的库。DateTime<Utc> 表示“带 UTC 时区的时间点”。
use chrono::{DateTime, Utc};
// Leptos 预导入：这里主要用到 ServerFnError、expect_context 以及 #[server] 宏所需的东西。
use leptos::prelude::*;
// serde 是 Rust 生态里做“序列化/反序列化”的标准库。
// Serialize   = 把 Rust 值 → 字节/JSON（发送时用）
// Deserialize = 把 JSON/字节 → Rust 值（接收时用）
use serde::{Deserialize, Serialize};

// #[derive(...)] 自动为结构体实现一些常用能力(trait)，省得手写。
// 【为什么恰好需要这几个】：
//   Debug      —— 允许用 {:?} 打印，方便调试。
//   Clone      —— 允许 .clone() 复制一份；app.rs 里做“乐观更新”时会复制列表/标题。
//   Serialize/Deserialize —— 关键！服务器函数把 Vec<Todo> 返回给浏览器时，需要先
//     把它序列化成 JSON 通过网络传输，浏览器端再反序列化还原成 Vec<Todo>。
//     没有这两个，Todo 就没法在前后端之间“跨网络传递”。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    // 字段都用 pub 公开。【为什么】：app.rs（不同模块）要直接读 todo.id、todo.title 等，
    // 不是 pub 的话跨模块访问不到。
    pub id: i32,                    // 主键，对应数据库 SERIAL；用 i32 匹配 Postgres 的 int4
    pub title: String,              // 标题
    pub completed: bool,            // 是否完成
    pub created_at: DateTime<Utc>,  // 创建时间（UTC）
}

// -------------------- 服务器函数 1：查询全部待办 --------------------
// #[server] 是本项目最核心的宏。
// 【它到底做了什么、为什么这么强大】：它会自动生成两套东西——
//   - 服务器侧：把下面的函数体注册成一个 HTTP 端点（默认 POST /api/<名字><哈希>）。
//   - 客户端侧：生成一个同名函数，body 被替换成“发一个 HTTP 请求到那个端点、
//     等结果、反序列化返回值”。
// 所以 app.rs 里无论在服务器 SSR 期间还是在浏览器里调用 get_todos()，写法完全一样，
// 但底层行为不同：服务器上是直接查库，浏览器里是发网络请求。你无需关心这个差异。
#[server]
// async + 返回 Result<_, ServerFnError>：
// 【为什么必须是这个签名】：服务器函数天然涉及网络/数据库，一定是异步的；而且随时可能
// 失败（数据库断了、网络错误…），所以返回 Result，用 ServerFnError 统一表示“可能跨网络
// 传递的错误”。ServerFnError 也是可序列化的，能把错误信息带回客户端。
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    // 从上下文里“取出”数据库连接池。
    // 【这行和 main.rs 如何配对】：main.rs 里用 provide_context(pool) 把连接池放进了
    //   上下文；这里 expect_context::<sqlx::PgPool>() 就是把它取回来。<...> 指定要取的类型。
    //   expect_ 版本表示“取不到就 panic”——因为如果连接池没提供，说明是程序配置错误。
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
    // 【map_err + ? 的组合，非常常用，务必理解】：
    //   sqlx 的错误类型和 ServerFnError 不是一回事，不能直接返回。map_err 把 sqlx 错误
    //   转换成 ServerFnError（用错误的文字描述）。末尾的 `?` 是“错误传播运算符”：
    //   如果结果是 Err 就【立刻从本函数返回这个错误】，是 Ok 就取出里面的值继续往下走。
    //   等价于一段 match，但简洁得多。
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 把 Vec<元组> 转换成 Vec<Todo> 并返回。
    Ok(rows
        .into_iter() // 把 Vec 变成迭代器，逐个消费其中的元组
        // map：对每个元组做转换。这里用“解构”直接把元组拆成 4 个变量，
        //      再用它们构造一个 Todo。字段名和变量名相同，可用简写（field init shorthand）。
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
    // 去掉首尾空白后再判断，避免用户输入一堆空格当标题。
    let title = title.trim().to_string();
    // 服务器端也要校验，不能只信任前端。
    // 【为什么服务器必须再校验一遍】：前端校验能被绕过（有人可以直接对 /api 发请求）。
    //   把“标题不能为空”作为服务器规则，才是真正的数据保护。空标题就返回错误。
    if title.is_empty() {
        return Err(ServerFnError::new("Title cannot be empty"));
    }
    let pool = expect_context::<sqlx::PgPool>();
    // INSERT 语句里的 $1 是“占位符/参数绑定”。
    // 【为什么用 $1 而不是把 title 拼进字符串】：这能防止 SQL 注入攻击。用 .bind(title)
    //   把值作为“参数”传给数据库，数据库会把它当纯数据处理，即使标题里含有 SQL 语法
    //   （如 '); DROP TABLE todos;--）也不会被当成命令执行。绝不要用字符串拼接拼 SQL。
    sqlx::query("INSERT INTO todos (title) VALUES ($1)")
        .bind(title) // 把 title 绑定到 $1
        .execute(&pool) // execute 用于会改数据、不需要返回行的语句（INSERT/UPDATE/DELETE）
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    // 返回 Ok(()) 表示“成功，但没有有意义的返回值”。() 是 Rust 的“空元组/单位类型”。
    Ok(())
}

// -------------------- 服务器函数 3：切换完成状态 --------------------
#[server]
pub async fn toggle_todo(id: i32) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    // SET completed = NOT completed：把布尔值取反（完成↔未完成）。
    // 【为什么在 SQL 里取反，而不是先查出来再写回】：一条语句原子完成，既少一次往返，
    //   也避免“读—改—写”之间的竞态问题。
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
        .bind(id)    // → $2
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
