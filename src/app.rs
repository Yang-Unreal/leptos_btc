use crate::auth::*;
use crate::todo::*;
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use leptos_router::path;
use uuid::Uuid;

// ============================================================================
// 平台兼容的时间获取
// ============================================================================
// 【问题背景】
// std::time::SystemTime::now() 在 WASM 平台未实现，直接调用会 panic。
// chrono::Utc::now() 底层依赖 SystemTime::now()，在浏览器中同样会崩溃。
//
// 【解决方案】
// WASM 环境下通过 js_sys::Date::new_0().get_time() 获取 JS 时间戳（毫秒），
// 再转换为 chrono::DateTime<Utc>，绕过 Rust std 的未实现平台限制。

#[cfg(target_arch = "wasm32")]
fn now_utc() -> DateTime<Utc> {
    let now_ms = js_sys::Date::new_0().get_time() as i64;
    DateTime::from_timestamp_millis(now_ms)
        .expect("Date::new_0() should return a valid timestamp")
        .with_timezone(&Utc)
}

#[cfg(not(target_arch = "wasm32"))]
fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

// ═══════════════════════════════════════════════════════════════════════════
// Auth Context — 全应用共享的认证状态
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct AuthContext {
    pub user: Resource<Option<User>>,
    pub refresh: Action<(), ()>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Todo 纯细粒度模型
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
pub struct TodoRx {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<Todo> for TodoRx {
    fn from(todo: Todo) -> Self {
        Self {
            id: todo.id,
            title: RwSignal::new(todo.title),
            completed: RwSignal::new(todo.completed),
            created_at: todo.created_at,
        }
    }
}

#[derive(Clone, Copy)]
struct TodoContext {
    todos_sig: RwSignal<Vec<TodoRx>>,
    active_count: RwSignal<usize>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Shell & App
// ═══════════════════════════════════════════════════════════════════════════

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <link rel="icon" href="/icon.svg" media="(prefers-color-scheme: light)" />
                <link rel="icon" href="/icon-dark.svg" media="(prefers-color-scheme: dark)" />
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // ── 认证状态 ────────────────────────────────────────
    let (refresh_signal, set_refresh_signal) = signal(0);
    let user_res = Resource::new(
        move || refresh_signal.get(),
        |_| async { get_current_user().await.unwrap_or(None) },
    );
    let refresh = Action::new(move |_: &()| {
        let set = set_refresh_signal;
        async move { set.update(|n| *n += 1) }
    });

    provide_context(AuthContext {
        user: user_res,
        refresh,
    });

    let site_url = "https://todos.example.com";

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>

        <Title text="Todos — Fast Full-Stack Todo App"/>
        <Meta name="description" content="A blazingly fast, server-side rendered todo list built with Leptos, Rust, Axum, and PostgreSQL."/>
        <Meta name="theme-color" content="#4f46e5"/>
        <Link rel="canonical" href=site_url/>
        <Meta property="og:type" content="website"/>
        <Meta property="og:url" content=site_url/>
        <Meta property="og:title" content="Todos — Fast Full-Stack Todo App"/>
        <Meta property="og:description" content="A blazingly fast, server-side rendered todo list built with Leptos and Rust."/>
        <Meta property="og:image" content=format!("{}/og-image.jpg", site_url)/>
        <Meta name="twitter:card" content="summary_large_image"/>

        <div class="w-full min-h-screen bg-linear-to-b from-slate-50 via-white to-slate-100 font-sans text-slate-800 antialiased flex flex-col">
            <Router>
                <header class="sticky top-0 z-50 w-full bg-white/80 backdrop-blur-xl border-b border-slate-200/60 shadow-sm">
                    <div class="w-full px-4 sm:px-6 lg:px-8 h-14 sm:h-16 flex items-center justify-between">
                        <a href="/" class="text-lg sm:text-xl font-extrabold bg-linear-to-br from-indigo-600 to-violet-600 bg-clip-text text-transparent tracking-tight">
                            "Todos"
                        </a>
                        <NavMenu/>
                    </div>
                </header>

                <main class="flex-1 w-full px-4 sm:px-6 lg:px-8 py-6 sm:py-8 lg:py-10 flex flex-col items-center">
                    <Routes fallback=|| view! { <div class="mt-10 text-xl font-bold">"404 Not Found"</div> }>
                        <Route path=path!("/") view=Dashboard/>
                        <Route path=path!("/login") view=LoginPage/>
                        <Route path=path!("/register") view=RegisterPage/>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Navigation
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn NavMenu() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let auth_for_logout = auth.clone();

    let logout_action = Action::new(move |_: &()| {
        let a = auth_for_logout.clone();
        async move {
            let _ = logout().await;
            a.refresh.dispatch(());
        }
    });

    view! {
        <Transition fallback=|| view! { <span class="text-sm text-slate-400">"..."</span> }>
            <Show
                when=move || {
                    let u = auth.user.get();
                    u.is_some() && u.unwrap().is_some()
                }
                fallback=|| view! {
                    <div class="flex items-center gap-3">
                        <a href="/login"
                            class="px-4 py-2 text-sm font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg transition-colors">
                            "Sign In"
                        </a>
                        <a href="/register"
                            class="px-4 py-2 text-sm font-semibold text-indigo-600 hover:text-indigo-700 border border-indigo-200 rounded-lg transition-colors">
                            "Register"
                        </a>
                    </div>
                }
            >
                <div class="flex items-center gap-4">
                    <span class="text-sm font-medium text-slate-600">
                        {move || {
                            auth.user.get()
                                .and_then(|u| u.map(|u| u.username))
                                .unwrap_or_default()
                        }}
                    </span>
                    <button
                        class="text-sm font-medium text-red-500 hover:text-red-700 transition-colors"
                        on:click=move |_| { logout_action.dispatch(()); }
                    >
                        "Logout"
                    </button>
                </div>
            </Show>
        </Transition>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Dashboard — 受保护页面
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn Dashboard() -> impl IntoView {
    let auth = expect_context::<AuthContext>();

    view! {
        <Transition fallback=|| view! { <div class="mt-10 animate-pulse">"Loading..."</div> }>
            {move || {
                match auth.user.get() {
                    None => view! { <div class="mt-10">"Loading..."</div> }.into_any(),
                    Some(None) => view! { <Redirect path="/login"/> }.into_any(),
                    Some(Some(_)) => view! { <Todos/> }.into_any(),
                }
            }}
        </Transition>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Login Page
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn LoginPage() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let (error, set_error) = signal(Option::<String>::None);
    let (pending, set_pending) = signal(false);

    let username_ref = NodeRef::<leptos::html::Input>::new();
    let password_ref = NodeRef::<leptos::html::Input>::new();

    let do_login = {
        let auth = auth.clone();
        move || {
            let u = username_ref.get().map(|el| el.value()).unwrap_or_default();
            let p = password_ref.get().map(|el| el.value()).unwrap_or_default();
            if u.is_empty() || p.is_empty() {
                set_error.set(Some("Please fill in all fields".into()));
                return;
            }
            let (u, p) = (u, p);
            let auth = auth.clone();
            set_pending.set(true);
            set_error.set(None);

            spawn_local(async move {
                match login(u, p).await {
                    Ok(()) => {
                        auth.refresh.dispatch(());
                        // 使用浏览器的原生 location API 做页面跳转
                        #[cfg(target_arch = "wasm32")]
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href("/");
                        }
                    }
                    Err(e) => {
                        set_error.set(Some(e.to_string()));
                        set_pending.set(false);
                    }
                }
            });
        }
    };
    let do_login = std::rc::Rc::new(do_login);

    view! {
        <div class="w-full max-w-sm mt-10 bg-white p-8 rounded-3xl shadow-xl border border-slate-100">
            <h2 class="text-2xl font-bold text-center text-slate-800 mb-6">"Welcome Back"</h2>

            <Show when=move || error.get().is_some()>
                <div class="mb-5 p-3 bg-red-50 text-red-600 rounded-xl text-sm text-center border border-red-100">
                    {move || error.get().unwrap()}
                </div>
            </Show>

            <div class="flex flex-col gap-4">
                <input node_ref=username_ref type="text" placeholder="Username or email"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"
                    on:keydown={
                        let d = do_login.clone();
                        move |ev| { if ev.key() == "Enter" { d(); } }
                    }/>
                <input node_ref=password_ref type="password" placeholder="Password"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"
                    on:keydown={
                        let d = do_login.clone();
                        move |ev| { if ev.key() == "Enter" { d(); } }
                    }/>

                <button
                    class="mt-2 px-4 py-3 bg-linear-to-br from-indigo-500 to-violet-600 text-white font-bold rounded-xl shadow-md hover:shadow-lg transition-all active:scale-95 disabled:opacity-50"
                    disabled=move || pending.get()
                    on:click={
                        let d = do_login.clone();
                        move |_| d()
                    }
                >
                    {move || if pending.get() { "Signing in..." } else { "Sign In" }}
                </button>

                <p class="text-center text-sm text-slate-500 mt-2">
                    "Don't have an account? "
                    <a href="/register" class="text-indigo-600 hover:underline">"Register"</a>
                </p>
            </div>
        </div>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Register Page
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn RegisterPage() -> impl IntoView {
    let (error, set_error) = signal(Option::<String>::None);
    let (pending, set_pending) = signal(false);

    let username_ref = NodeRef::<leptos::html::Input>::new();
    let email_ref = NodeRef::<leptos::html::Input>::new();
    let password_ref = NodeRef::<leptos::html::Input>::new();
    let confirm_ref = NodeRef::<leptos::html::Input>::new();

    let do_register = {
        move || {
            let username = username_ref.get().map(|el| el.value()).unwrap_or_default();
            let email = email_ref.get().map(|el| el.value()).unwrap_or_default();
            let password = password_ref.get().map(|el| el.value()).unwrap_or_default();
            let confirm = confirm_ref.get().map(|el| el.value()).unwrap_or_default();

        if username.is_empty() || email.is_empty() || password.is_empty() {
            set_error.set(Some("Please fill in all fields".into()));
            return;
        }
        if password != confirm {
            set_error.set(Some("Passwords do not match".into()));
            return;
        }
        if password.len() < 8 {
            set_error.set(Some("Password must be at least 8 characters".into()));
            return;
        }

        let (username, email, password) = (username, email, password);
            set_pending.set(true);
            set_error.set(None);

            spawn_local(async move {
                match register(username, email, password).await {
                    Ok(()) => {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href("/login");
                        }
                    }
                    Err(e) => {
                        set_error.set(Some(e.to_string()));
                        set_pending.set(false);
                    }
                }
            });
        }
    };
    let do_register = std::rc::Rc::new(do_register);

    view! {
        <div class="w-full max-w-sm mt-10 bg-white p-8 rounded-3xl shadow-xl border border-slate-100">
            <h2 class="text-2xl font-bold text-center text-slate-800 mb-6">"Create Account"</h2>

            <Show when=move || error.get().is_some()>
                <div class="mb-5 p-3 bg-red-50 text-red-600 rounded-xl text-sm text-center border border-red-100">
                    {move || error.get().unwrap()}
                </div>
            </Show>

            <div class="flex flex-col gap-4">
                <input node_ref=username_ref type="text" placeholder="Username"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=email_ref type="email" placeholder="Email"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=password_ref type="password" placeholder="Password (min 8 chars)"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=confirm_ref type="password" placeholder="Confirm password"
                    class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>

                <button
                    class="mt-2 px-4 py-3 bg-linear-to-br from-indigo-500 to-violet-600 text-white font-bold rounded-xl shadow-md hover:shadow-lg transition-all active:scale-95 disabled:opacity-50"
                    disabled=move || pending.get()
                    on:click={
                        let d = do_register.clone();
                        move |_| d()
                    }
                >
                    {move || if pending.get() { "Creating..." } else { "Create Account" }}
                </button>

                <p class="text-center text-sm text-slate-500 mt-2">
                    "Already have an account? "
                    <a href="/login" class="text-indigo-600 hover:underline">"Sign In"</a>
                </p>
            </div>
        </div>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Todos 组件（无变化，仅保留原有逻辑）
// ═══════════════════════════════════════════════════════════════════════════

#[component]
pub fn Todos() -> impl IntoView {
    let (refetch, set_refetch) = signal(0u64);
    let todos_res = Resource::new_blocking(move || refetch.get(), |_| get_todos());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    view! {
        <section class="w-full bg-white/80 backdrop-blur-md rounded-2xl sm:rounded-3xl p-6 sm:p-8 lg:p-10 shadow-[0_1px_3px_rgba(0,0,0,0.04),0_8px_30px_rgba(0,0,0,0.04)] border border-slate-200/50 hover:shadow-[0_1px_3px_rgba(0,0,0,0.06),0_12px_40px_rgba(0,0,0,0.06)] transition-shadow duration-300">
            <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-800 text-center mb-1 sm:mb-2">
                "Task Dashboard"
            </h1>

            <Show when=move || error_msg.get().is_some()>
                <div role="alert" class="mb-6 px-4 py-3 bg-red-50 border border-red-200/60 rounded-xl text-sm text-red-600 flex items-center justify-between shadow-sm">
                    <span>{move || error_msg.get().unwrap_or_default()}</span>
                    <button class="ml-3 text-red-400 hover:text-red-600 font-bold" on:click=move |_| set_error_msg.set(None)>"×"</button>
                </div>
            </Show>

            <Transition fallback=move || view! { <p class="text-center text-slate-400 text-sm my-8 animate-pulse">"Loading tasks…"</p> }>
                {move || Suspend::new(async move {
                    match todos_res.await {
                        Ok(list) => view! { <TodoManager initial=list set_refetch set_error_msg/> }.into_any(),
                        Err(e) => view! { <div class="text-center text-red-500 my-6">{format!("Error: {}", e)}</div> }.into_any(),
                    }
                })}
            </Transition>
        </section>
    }
}

#[component]
fn TodoManager(
    initial: Vec<Todo>,
    set_refetch: WriteSignal<u64>,
    set_error_msg: WriteSignal<Option<String>>,
) -> impl IntoView {
    let initial_rx: Vec<TodoRx> = initial.into_iter().map(TodoRx::from).collect();
    let initial_count = initial_rx
        .iter()
        .filter(|t| !t.completed.get_untracked())
        .count();

    let todos_sig = RwSignal::new(initial_rx);
    let active_count = RwSignal::new(initial_count);

    provide_context(TodoContext {
        todos_sig,
        active_count,
    });

    let title_ref = NodeRef::<leptos::html::Input>::new();
    let handle_err = move |e: ServerFnError| {
        set_error_msg.set(Some(format!("Operation failed: {}", e)));
        set_refetch.update(|n| *n = n.wrapping_add(1));
    };

    let add = Action::new(move |(id, title): &(Uuid, String)| {
        let (id, title) = (*id, title.clone());
        async move {
            if let Err(e) = add_todo(id, title).await {
                handle_err(e)
            }
        }
    });

    let toggle = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = toggle_todo(id).await {
                handle_err(e)
            }
        }
    });

    let delete = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = delete_todo(id).await {
                handle_err(e)
            }
        }
    });

    let update = Action::new(move |(id, title): &(Uuid, String)| {
        let (id, title) = (*id, title.clone());
        async move {
            if let Err(e) = update_todo(id, title).await {
                handle_err(e)
            }
        }
    });

    view! {
        <div class="mb-5 w-full px-4 py-2.5 bg-linear-to-r from-indigo-50 to-violet-50 rounded-xl border border-indigo-100/60 text-center text-sm font-semibold text-indigo-600 tracking-wide shadow-sm">
            {move || format!("{} active task{}", active_count.get(), if active_count.get() == 1 { "" } else { "s" })}
        </div>

        <form
            class="flex flex-col sm:flex-row gap-3 sm:gap-4 mb-8 w-full"
            on:submit=move |ev| {
                ev.prevent_default();
                let value = title_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                if !value.is_empty() {
                    let id = Uuid::now_v7();
                    todos_sig.update(|t| t.push(TodoRx {
                        id, title: RwSignal::new(value.clone()), completed: RwSignal::new(false), created_at: now_utc()
                    }));
                    active_count.update(|c| *c += 1);
                    add.dispatch((id, value));
                    if let Some(input) = title_ref.get() { input.set_value(""); }
                }
            }
        >
            <input node_ref=title_ref type="text" placeholder="What needs to be done?"
                class="flex-1 px-5 py-3.5 text-base bg-slate-50 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/30"
            />
            <button type="submit" class="px-6 py-3.5 text-sm font-semibold text-white bg-linear-to-br from-indigo-500 to-violet-500 rounded-xl">"Add"</button>
        </form>

        <Show when=move || !todos_sig.read().is_empty()>
            <ul class="w-full list-none m-0 p-0 flex flex-col-reverse gap-2.5">
                <For
                    each=move || todos_sig.get()
                    key=|row| row.id
                    children=move |todo| view! { <TodoRow todo toggle delete update/> }
                />
            </ul>
        </Show>
    }
}

#[component]
fn TodoRow(
    todo: TodoRx,
    toggle: Action<Uuid, ()>,
    delete: Action<Uuid, ()>,
    update: Action<(Uuid, String), ()>,
) -> impl IntoView {
    let ctx = expect_context::<TodoContext>();
    let id = todo.id;
    let title = todo.title;
    let completed = todo.completed;

    let (editing, set_editing) = signal(false);
    let edit_ref = NodeRef::<leptos::html::Input>::new();

    Effect::new(move |_| {
        if editing.get() {
            if let Some(input) = edit_ref.get() {
                let _ = input.focus();
            }
        }
    });

    view! {
        <li class:completed=move || completed.get() class="group w-full flex items-center gap-3 p-3 border border-slate-100 rounded-xl bg-white/80 transition-all">
            <Show when=move || editing.get() fallback=move || view! {
                <TodoDisplay id title completed toggle delete set_editing ctx/>
            }>
                <TodoEdit id title set_editing update edit_ref/>
            </Show>
        </li>
    }
}

#[component]
fn TodoDisplay(
    id: Uuid,
    title: RwSignal<String>,
    completed: RwSignal<bool>,
    toggle: Action<Uuid, ()>,
    delete: Action<Uuid, ()>,
    set_editing: WriteSignal<bool>,
    ctx: TodoContext,
) -> impl IntoView {
    view! {
        <input type="checkbox"
            class="w-5 h-5 accent-indigo-500 cursor-pointer flex-none"
            prop:checked=move || completed.get()
            on:click=move |_| {
                let current = completed.get_untracked();
                completed.set(!current);
                if !current { ctx.active_count.update(|c| *c = c.saturating_sub(1)); }
                else { ctx.active_count.update(|c| *c += 1); }
                toggle.dispatch(id);
            }
        />
        <span
            class="flex-1 min-w-0 text-sm sm:text-base wrap-break-word transition-all duration-200"
            class:line-through=move || completed.get()
            class:text-slate-400=move || completed.get()
            class:text-slate-700=move || !completed.get()
        >
            {move || title.get()}
        </span>
        <div class="flex items-center gap-1.5 opacity-0 group-hover:opacity-100">
            <button class="p-1.5 text-slate-400 hover:text-indigo-500 transition-colors" on:click=move |_| set_editing.set(true)>"✎"</button>
            <button class="p-1.5 text-slate-400 hover:text-red-500 transition-colors" on:click=move |_| {
                if !completed.get_untracked() { ctx.active_count.update(|c| *c = c.saturating_sub(1)); }
                ctx.todos_sig.update(|t| t.retain(|todo| todo.id != id));
                delete.dispatch(id);
            }>"✕"</button>
        </div>
    }
}

#[component]
fn TodoEdit(
    id: Uuid,
    title: RwSignal<String>,
    set_editing: WriteSignal<bool>,
    update: Action<(Uuid, String), ()>,
    edit_ref: NodeRef<leptos::html::Input>,
) -> impl IntoView {
    view! {
        <input node_ref=edit_ref type="text"
            class="flex-1 px-3 py-2 text-sm bg-slate-50 border-2 border-indigo-300 rounded-lg focus:outline-none"
            prop:value=move || title.get()
            on:keydown=move |ev| {
                if ev.key() == "Enter" {
                    let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                    if !value.is_empty() {
                        title.set(value.clone());
                        set_editing.set(false);
                        update.dispatch((id, value));
                    }
                }
                else if ev.key() == "Escape" { set_editing.set(false); }
            }
        />
        <button class="px-4 py-2 text-xs font-semibold text-white bg-indigo-500 rounded-lg" on:click=move |_| {
            let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
            if !value.is_empty() {
                title.set(value.clone());
                set_editing.set(false);
                update.dispatch((id, value));
            }
        }>"Save"</button>
        <button class="px-3 py-2 text-xs font-semibold text-slate-500 bg-slate-100 rounded-lg" on:click=move |_| set_editing.set(false)>"Cancel"</button>
    }
}
