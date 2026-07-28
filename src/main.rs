#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_btc::app::{shell, App};
    use leptos_btc::auth::AuthBackend;
    use sqlx::postgres::PgPoolOptions;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::PostgresStore;

    let conf = get_configuration(Some("Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("could not connect to Postgres");

    // ── 建表 ──────────────────────────────────────────────

    // users 认证表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id            UUID PRIMARY KEY,
            username      TEXT NOT NULL UNIQUE,
            email         TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&pool)
    .await
    .expect("could not create users table");

    // todos 业务表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (
            id          UUID PRIMARY KEY,
            title       TEXT NOT NULL,
            completed   BOOLEAN NOT NULL DEFAULT FALSE,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            user_id     UUID NOT NULL REFERENCES users(id)
        )",
    )
    .execute(&pool)
    .await
    .expect("could not create todos table");

    // ── Session 存储（PostgreSQL 持久化）─────────────────
    let session_store = PostgresStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to migrate session store");

    // 生产环境请将 with_secure(false) 改为 with_secure(true)
    let session_secure: bool = std::env::var("SESSION_SECURE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false);

    let session_layer = SessionManagerLayer::new(session_store).with_secure(session_secure);

    // ── Auth 中间件 ───────────────────────────────────────
    let auth_backend = AuthBackend { pool: pool.clone() };
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    let routes = generate_route_list(App);

    // ── 路由注册 ──────────────────────────────────────────
    // leptos_routes_with_context 同时注册 SSR 页面路由和 server function 端点。
    // additional_context 闭包在每个请求的 SSR 渲染前同步执行，
    // 此处仅注入数据库连接池，供后续 Server Function 通过 expect_context 获取。
    // 认证状态由 App 组件内 get_current_user() 异步提取后，
    // 以 AuthContext provide_context 到子组件树。
    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let pool = pool.clone();
                move || {
                    provide_context(pool.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(auth_layer)
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
