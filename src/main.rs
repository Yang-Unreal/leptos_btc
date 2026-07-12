#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    // Load DATABASE_URL (and any other vars) from a local .env file when present.
    // No-op if the environment variables are already set (e.g. in Dokploy).
    let _ = dotenvy::dotenv();

    use axum::Router;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::routing::post;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
    use leptos_btc::app::*;
    use sqlx::postgres::PgPoolOptions;

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    // --- Postgres connection pool + run migrations ---
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("could not connect to Postgres");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (
            id          SERIAL PRIMARY KEY,
            title       TEXT NOT NULL,
            completed   BOOLEAN NOT NULL DEFAULT FALSE,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&pool)
    .await
    .expect("could not create todos table");

    let routes = generate_route_list(App);

    let api_pool = pool.clone();
    let app = Router::new()
        // Server functions (the CRUD endpoints) are served under /api.
        .route(
            "/api/{tail..}",
            post(move |req: Request<Body>| {
                let pool = api_pool.clone();
                async move {
                    handle_server_fns_with_context(move || provide_context(pool.clone()), req)
                        .await
                }
            }),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                // Runs inside the reactive owner for every request, so the pool
                // is available to server functions triggered during SSR (Resources)
                // as well as to /api server-function calls.
                let ssr_pool = pool.clone();
                move || provide_context(ssr_pool.clone())
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
