use crate::auth::*;
use crate::todo::*;
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{provide_meta_context, Meta, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use leptos_router::path;
use uuid::Uuid;

// ============================================================================
// 平台兼容的时间获取
// ============================================================================
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
// Auth Context
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct AuthContext {
    pub user: Resource<Option<User>>,
    pub refresh: Action<(), ()>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 细粒度响应式模型 (Rx)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
pub struct SubtaskRx {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Copy, Debug)]
pub struct TodoRx {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
    pub created_at: chrono::DateTime<Utc>,
    pub subtasks: RwSignal<Vec<SubtaskRx>>,
}

impl TodoRx {
    /// 自动根据子任务状态计算进度，如果没有子任务，进度由自身完成状态决定
    pub fn progress(&self) -> Signal<f64> {
        let subtasks = self.subtasks;
        let completed = self.completed;
        Signal::derive(move || {
            let subs = subtasks.read();
            if subs.is_empty() {
                return if completed.get() { 100.0 } else { 0.0 };
            }
            let total = subs.len() as f64;
            let done = subs.iter().filter(|s| s.completed.get()).count() as f64;
            (done / total) * 100.0
        })
    }
}

impl From<Todo> for TodoRx {
    fn from(todo: Todo) -> Self {
        let subtasks_rx = todo
            .subtasks
            .into_iter()
            .map(|s| SubtaskRx {
                id: s.id,
                todo_id: s.todo_id,
                title: RwSignal::new(s.title),
                completed: RwSignal::new(s.completed),
                created_at: s.created_at,
            })
            .collect();

        Self {
            id: todo.id,
            title: RwSignal::new(todo.title),
            completed: RwSignal::new(todo.completed),
            created_at: todo.created_at,
            subtasks: RwSignal::new(subtasks_rx),
        }
    }
}

// 将所有的 Action 放入 Context，彻底消除 Prop Drilling
#[derive(Clone, Copy)]
struct TodoContext {
    todos_sig: RwSignal<Vec<TodoRx>>,
    active_count: RwSignal<usize>,
    toggle: Action<Uuid, ()>,
    delete: Action<Uuid, ()>,
    update: Action<(Uuid, String), ()>,
    add_subtask: Action<(Uuid, Uuid, String), ()>,
    toggle_subtask: Action<Uuid, ()>,
    delete_subtask: Action<Uuid, ()>,
    update_subtask: Action<(Uuid, String), ()>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Shell & App & Nav
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
            <body><App/></body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
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

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>
        <Title text="Project Dashboard — Fast Full-Stack Manager"/>
        <Meta name="description" content="A blazingly fast project management tool built with Leptos."/>
        <Meta name="theme-color" content="#4f46e5"/>

        <div class="w-full min-h-screen bg-linear-to-b from-slate-50 via-white to-slate-100 font-sans text-slate-800 antialiased flex flex-col">
            <Router>
                <header class="sticky top-0 z-50 w-full bg-white/80 backdrop-blur-xl border-b border-slate-200/60 shadow-sm">
                    <div class="w-full px-4 sm:px-6 lg:px-8 h-14 sm:h-16 flex items-center justify-between">
                        <a href="/" class="text-lg sm:text-xl font-extrabold bg-linear-to-br from-indigo-600 to-violet-600 bg-clip-text text-transparent tracking-tight">
                            "Tasks.IO"
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
                when=move || { auth.user.get().and_then(|u| u).is_some() }
                fallback=|| view! {
                    <div class="flex items-center gap-3">
                        <a href="/login" class="px-4 py-2 text-sm font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg transition-colors">"Sign In"</a>
                        <a href="/register" class="px-4 py-2 text-sm font-semibold text-indigo-600 hover:text-indigo-700 border border-indigo-200 rounded-lg transition-colors">"Register"</a>
                    </div>
                }
            >
                <div class="flex items-center gap-4">
                    <span class="text-sm font-medium text-slate-600">{move || { auth.user.get().and_then(|u| u.map(|u| u.username)).unwrap_or_default() }}</span>
                    <button class="text-sm font-medium text-red-500 hover:text-red-700 transition-colors" on:click=move |_| { logout_action.dispatch(()); }>
                        "Logout"
                    </button>
                </div>
            </Show>
        </Transition>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Auth Pages
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn Dashboard() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    view! {
        <Transition fallback=|| view! { <div class="mt-10 animate-pulse">"Loading..."</div> }>
            {move || match auth.user.get() {
                None => view! { <div class="mt-10">"Loading..."</div> }.into_any(),
                Some(None) => view! { <Redirect path="/login"/> }.into_any(),
                Some(Some(_)) => view! { <Todos/> }.into_any(),
            }}
        </Transition>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let (error, set_error) = signal(Option::<String>::None);
    let (pending, set_pending) = signal(false);
    let username_ref = NodeRef::<leptos::html::Input>::new();
    let password_ref = NodeRef::<leptos::html::Input>::new();

    let do_login = std::rc::Rc::new({
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
    });

    view! {
        <div class="w-full max-w-sm mt-10 bg-white p-8 rounded-3xl shadow-xl border border-slate-100">
            <h2 class="text-2xl font-bold text-center text-slate-800 mb-6">"Welcome Back"</h2>
            <Show when=move || error.get().is_some()>
                <div class="mb-5 p-3 bg-red-50 text-red-600 rounded-xl text-sm text-center border border-red-100">{move || error.get().unwrap()}</div>
            </Show>
            <div class="flex flex-col gap-4">
                <input node_ref=username_ref type="text" placeholder="Username or email" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"
                    on:keydown={ let d = do_login.clone(); move |ev| { if ev.key() == "Enter" { d(); } } }/>
                <input node_ref=password_ref type="password" placeholder="Password" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"
                    on:keydown={ let d = do_login.clone(); move |ev| { if ev.key() == "Enter" { d(); } } }/>
                <button class="mt-2 px-4 py-3 bg-linear-to-br from-indigo-500 to-violet-600 text-white font-bold rounded-xl shadow-md hover:shadow-lg transition-all active:scale-95 disabled:opacity-50"
                    disabled=move || pending.get() on:click={ let d = do_login.clone(); move |_| d() }>
                    {move || if pending.get() { "Signing in..." } else { "Sign In" }}
                </button>
                <p class="text-center text-sm text-slate-500 mt-2">"Don't have an account? "<a href="/register" class="text-indigo-600 hover:underline">"Register"</a></p>
            </div>
        </div>
    }
}

#[component]
fn RegisterPage() -> impl IntoView {
    let (error, set_error) = signal(Option::<String>::None);
    let (pending, set_pending) = signal(false);
    let username_ref = NodeRef::<leptos::html::Input>::new();
    let email_ref = NodeRef::<leptos::html::Input>::new();
    let password_ref = NodeRef::<leptos::html::Input>::new();
    let confirm_ref = NodeRef::<leptos::html::Input>::new();

    let do_register = std::rc::Rc::new(move || {
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
        set_pending.set(true);
        set_error.set(None);
        spawn_local(async move {
            match register(username, email, password).await {
                Ok(()) =>
                {
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
    });

    view! {
        <div class="w-full max-w-sm mt-10 bg-white p-8 rounded-3xl shadow-xl border border-slate-100">
            <h2 class="text-2xl font-bold text-center text-slate-800 mb-6">"Create Account"</h2>
            <Show when=move || error.get().is_some()>
                <div class="mb-5 p-3 bg-red-50 text-red-600 rounded-xl text-sm text-center border border-red-100">{move || error.get().unwrap()}</div>
            </Show>
            <div class="flex flex-col gap-4">
                <input node_ref=username_ref type="text" placeholder="Username" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=email_ref type="email" placeholder="Email" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=password_ref type="password" placeholder="Password (min 8 chars)" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <input node_ref=confirm_ref type="password" placeholder="Confirm password" class="px-4 py-3 bg-slate-50 border border-slate-200 rounded-xl focus:ring-2 focus:ring-indigo-500/30 focus:outline-none transition-all"/>
                <button class="mt-2 px-4 py-3 bg-linear-to-br from-indigo-500 to-violet-600 text-white font-bold rounded-xl shadow-md hover:shadow-lg transition-all active:scale-95 disabled:opacity-50"
                    disabled=move || pending.get() on:click={ let d = do_register.clone(); move |_| d() }>
                    {move || if pending.get() { "Creating..." } else { "Create Account" }}
                </button>
                <p class="text-center text-sm text-slate-500 mt-2">"Already have an account? "<a href="/login" class="text-indigo-600 hover:underline">"Sign In"</a></p>
            </div>
        </div>
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Todos (Project Board)
// ═══════════════════════════════════════════════════════════════════════════

#[component]
pub fn Todos() -> impl IntoView {
    let (refetch, set_refetch) = signal(0u64);
    let todos_res = Resource::new_blocking(move || refetch.get(), |_| get_todos());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    view! {
        <section class="w-full max-w-4xl bg-white/80 backdrop-blur-md rounded-2xl sm:rounded-3xl p-6 sm:p-8 lg:p-10 shadow-[0_1px_3px_rgba(0,0,0,0.04),0_8px_30px_rgba(0,0,0,0.04)] border border-slate-200/50 hover:shadow-[0_1px_3px_rgba(0,0,0,0.06),0_12px_40px_rgba(0,0,0,0.06)] transition-shadow duration-300">
            <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-800 text-center mb-1 sm:mb-2">
                "Project Kanban"
            </h1>
            <Show when=move || error_msg.get().is_some()>
                <div role="alert" class="mb-6 px-4 py-3 bg-red-50 border border-red-200/60 rounded-xl text-sm text-red-600 flex items-center justify-between shadow-sm">
                    <span>{move || error_msg.get().unwrap_or_default()}</span>
                    <button class="ml-3 text-red-400 hover:text-red-600 font-bold" on:click=move |_| set_error_msg.set(None)>"×"</button>
                </div>
            </Show>
            <Transition fallback=move || view! { <p class="text-center text-slate-400 text-sm my-8 animate-pulse">"Loading projects…"</p> }>
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
    let title_ref = NodeRef::<leptos::html::Input>::new();

    let handle_err = move |e: ServerFnError| {
        set_error_msg.set(Some(format!("Operation failed: {}", e)));
        set_refetch.update(|n| *n = n.wrapping_add(1));
    };

    // ── 初始化所有的后台 Actions ──
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
    let add_subtask = Action::new(move |(id, todo_id, title): &(Uuid, Uuid, String)| {
        let (id, todo_id, title) = (*id, *todo_id, title.clone());
        async move {
            if let Err(e) = add_subtask(id, todo_id, title).await {
                handle_err(e)
            }
        }
    });
    let toggle_subtask = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = toggle_subtask(id).await {
                handle_err(e)
            }
        }
    });
    let delete_subtask = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = delete_subtask(id).await {
                handle_err(e)
            }
        }
    });
    let update_subtask = Action::new(move |(id, title): &(Uuid, String)| {
        let (id, title) = (*id, title.clone());
        async move {
            if let Err(e) = update_subtask(id, title).await {
                handle_err(e)
            }
        }
    });

    provide_context(TodoContext {
        todos_sig,
        active_count,
        toggle,
        delete,
        update,
        add_subtask,
        toggle_subtask,
        delete_subtask,
        update_subtask,
    });

    view! {
        <div class="mb-5 w-full px-4 py-2.5 bg-linear-to-r from-indigo-50 to-violet-50 rounded-xl border border-indigo-100/60 text-center text-sm font-semibold text-indigo-600 tracking-wide shadow-sm">
            {move || format!("{} active task epic{}", active_count.get(), if active_count.get() == 1 { "" } else { "s" })}
        </div>

        <form class="flex flex-col sm:flex-row gap-3 sm:gap-4 mb-8 w-full"
            on:submit=move |ev| {
                ev.prevent_default();
                let value = title_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                if !value.is_empty() {
                    let id = Uuid::now_v7();
                    todos_sig.update(|t| t.push(TodoRx {
                        id, title: RwSignal::new(value.clone()), completed: RwSignal::new(false), created_at: now_utc(), subtasks: RwSignal::new(Vec::new())
                    }));
                    active_count.update(|c| *c += 1);
                    add.dispatch((id, value));
                    if let Some(input) = title_ref.get() { input.set_value(""); }
                }
            }
        >
            <input node_ref=title_ref type="text" placeholder="Create a new task epic..."
                class="flex-1 px-5 py-3.5 text-base bg-slate-50 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/30"
            />
            <button type="submit" class="px-6 py-3.5 text-sm font-semibold text-white bg-linear-to-br from-indigo-500 to-violet-500 rounded-xl shadow-md active:scale-95 transition-all">"Create"</button>
        </form>

        <Show when=move || !todos_sig.read().is_empty()>
            <ul class="w-full list-none m-0 p-0 flex flex-col-reverse gap-4">
                <For
                    each=move || todos_sig.get()
                    key=|row| row.id
                    children=move |todo| view! { <TodoRow todo/> }
                />
            </ul>
        </Show>
    }
}

// ── 单个主任务（包含进度条与子任务手风琴容器） ──
#[component]
fn TodoRow(todo: TodoRx) -> impl IntoView {
    let ctx = expect_context::<TodoContext>();
    let (expanded, set_expanded) = signal(false);
    let (editing, set_editing) = signal(false);
    let edit_ref = NodeRef::<leptos::html::Input>::new();

    Effect::new(move |_| {
        if editing.get() {
            if let Some(input) = edit_ref.get() {
                let _ = input.focus();
            }
        }
    });

    let subtasks = todo.subtasks;
    let new_sub_ref = NodeRef::<leptos::html::Input>::new();

    view! {
        <li class="group w-full flex flex-col p-4 border border-slate-200/80 rounded-2xl bg-white shadow-sm hover:shadow-md transition-all">

            <div class="flex items-center gap-3">
                <button
                    class="p-1 text-slate-400 hover:text-indigo-600 transition-transform duration-300"
                    class:rotate-90=move || expanded.get()
                    on:click=move |_| set_expanded.update(|e| *e = !*e)
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                </button>

                <Show when=move || editing.get() fallback=move || view! { <TodoDisplay todo=todo set_editing ctx/> }>
                    <TodoEdit todo=todo set_editing edit_ref ctx/>
                </Show>
            </div>


            <div class="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden mt-3 mb-1 shrink-0 relative">
                <div class="absolute left-0 top-0 h-full bg-linear-to-r from-indigo-500 to-violet-500 transition-all duration-500 ease-out"
                     style=move || format!("width: {}%", todo.progress().get())></div>
            </div>


            <div class:hidden=move || !expanded.get()>
                <div class="pl-8 pt-4 pb-2 flex flex-col gap-3 border-t border-slate-50 mt-3">
                    <For
                        each=move || subtasks.get()
                        key=|sub| sub.id
                        children=move |sub| view! { <SubtaskRow sub parent_subtasks=subtasks ctx/> }
                    />

                    <form
                        class="flex items-center gap-2 mt-1 relative"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            let val = new_sub_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                            if !val.is_empty() {
                                let sub_id = Uuid::now_v7();
                                subtasks.update(|subs| subs.push(SubtaskRx {
                                    id: sub_id, todo_id: todo.id, title: RwSignal::new(val.clone()), completed: RwSignal::new(false), created_at: now_utc()
                                }));
                                ctx.add_subtask.dispatch((sub_id, todo.id, val));
                                if let Some(input) = new_sub_ref.get() { input.set_value(""); }
                            }
                        }
                    >
                        <div class="absolute left-3 text-slate-300">
                           <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                <line x1="12" y1="5" x2="12" y2="19"></line>
                                <line x1="5" y1="12" x2="19" y2="12"></line>
                            </svg>
                        </div>
                        <input node_ref=new_sub_ref type="text" placeholder="Add a subtask..."
                            class="flex-1 pl-9 pr-3 py-2 text-sm bg-slate-50 border border-slate-200/70 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/30 transition-all placeholder-slate-400"
                        />
                    </form>
                </div>
            </div>
        </li>
    }
}

// ── 主任务展示态 ──
#[component]
fn TodoDisplay(todo: TodoRx, set_editing: WriteSignal<bool>, ctx: TodoContext) -> impl IntoView {
    let id = todo.id;
    let title = todo.title;
    let completed = todo.completed;

    view! {
        <input type="checkbox"
            class="w-5 h-5 accent-indigo-500 cursor-pointer flex-none rounded border-slate-300 transition-all"
            prop:checked=move || completed.get()
            on:click=move |_| {
                let current = completed.get_untracked();
                completed.set(!current);
                if !current { ctx.active_count.update(|c| *c = c.saturating_sub(1)); }
                else { ctx.active_count.update(|c| *c += 1); }
                ctx.toggle.dispatch(id);
            }
        />
        <span
            class="flex-1 min-w-0 text-base sm:text-lg font-semibold wrap-break-word transition-all duration-200"
            class:line-through=move || completed.get()
            class:text-slate-400=move || completed.get()
            class:text-slate-800=move || !completed.get()
        >
            {move || title.get()}
        </span>
        <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button class="p-2 text-slate-400 hover:text-indigo-500 transition-colors rounded-lg hover:bg-slate-50" on:click=move |_| set_editing.set(true)>
                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
            </button>
            <button class="p-2 text-slate-400 hover:text-red-500 transition-colors rounded-lg hover:bg-red-50" on:click=move |_| {
                if !completed.get_untracked() { ctx.active_count.update(|c| *c = c.saturating_sub(1)); }
                ctx.todos_sig.update(|t| t.retain(|item| item.id != id));
                ctx.delete.dispatch(id);
            }>
                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
            </button>
        </div>
    }
}

// ── 主任务编辑态 ──
#[component]
fn TodoEdit(
    todo: TodoRx,
    set_editing: WriteSignal<bool>,
    edit_ref: NodeRef<leptos::html::Input>,
    ctx: TodoContext,
) -> impl IntoView {
    let id = todo.id;
    let title = todo.title;

    view! {
        <input node_ref=edit_ref type="text"
            class="flex-1 px-3 py-1.5 text-base font-semibold bg-indigo-50/50 border border-indigo-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/40"
            prop:value=move || title.get()
            on:keydown=move |ev| {
                if ev.key() == "Enter" {
                    let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                    if !value.is_empty() {
                        title.set(value.clone());
                        set_editing.set(false);
                        ctx.update.dispatch((id, value));
                    }
                }
                else if ev.key() == "Escape" { set_editing.set(false); }
            }
        />
        <button class="px-3 py-1.5 text-xs font-bold text-white bg-indigo-500 rounded-lg hover:bg-indigo-600 transition-colors shadow-sm" on:click=move |_| {
            let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
            if !value.is_empty() {
                title.set(value.clone());
                set_editing.set(false);
                ctx.update.dispatch((id, value));
            }
        }>"Save"</button>
        <button class="px-3 py-1.5 text-xs font-bold text-slate-500 bg-slate-100 rounded-lg hover:bg-slate-200 transition-colors" on:click=move |_| set_editing.set(false)>"Cancel"</button>
    }
}

// ── 子任务渲染组件 ──
#[component]
fn SubtaskRow(
    sub: SubtaskRx,
    parent_subtasks: RwSignal<Vec<SubtaskRx>>,
    ctx: TodoContext,
) -> impl IntoView {
    let id = sub.id;
    let title = sub.title;
    let completed = sub.completed;

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
        <div class="group/sub flex items-center gap-3 py-1.5 px-2 rounded-lg hover:bg-slate-50 transition-colors border border-transparent hover:border-slate-100 min-h-9">
            <Show when=move || editing.get() fallback=move || view! {
                <input type="checkbox"
                    class="w-4 h-4 accent-violet-500 cursor-pointer flex-none rounded-sm border-slate-300"
                    prop:checked=move || completed.get()
                    on:click=move |_| {
                        completed.update(|c| *c = !*c);
                        ctx.toggle_subtask.dispatch(id);
                    }
                />
                <span
                    class="flex-1 text-sm font-medium wrap-break-word transition-all duration-200"
                    class:line-through=move || completed.get()
                    class:text-slate-400=move || completed.get()
                    class:text-slate-600=move || !completed.get()
                >
                    {move || title.get()}
                </span>

                <div class="opacity-0 group-hover/sub:opacity-100 flex items-center gap-1 transition-opacity">
                    <button
                        class="p-1.5 text-slate-300 hover:text-indigo-500 transition-all rounded hover:bg-indigo-50"
                        on:click=move |_| set_editing.set(true)
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
                    </button>
                    <button
                        class="p-1.5 text-slate-300 hover:text-red-500 transition-all rounded hover:bg-red-50"
                        on:click=move |_| {
                            parent_subtasks.update(|subs| subs.retain(|s| s.id != id));
                            ctx.delete_subtask.dispatch(id);
                        }
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            }>
                <input node_ref=edit_ref type="text"
                    class="flex-1 px-2 py-1 text-sm bg-white border border-violet-300 rounded focus:outline-none focus:ring-2 focus:ring-violet-500/40"
                    prop:value=move || title.get()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                            if !value.is_empty() {
                                title.set(value.clone());
                                set_editing.set(false);
                                ctx.update_subtask.dispatch((id, value));
                            }
                        }
                        else if ev.key() == "Escape" { set_editing.set(false); }
                    }
                />
                <button class="px-2 py-1 text-[11px] font-bold text-white bg-violet-500 rounded hover:bg-violet-600 transition-colors shadow-sm" on:click=move |_| {
                    let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                    if !value.is_empty() {
                        title.set(value.clone());
                        set_editing.set(false);
                        ctx.update_subtask.dispatch((id, value));
                    }
                }>"Save"</button>
                <button class="px-2 py-1 text-[11px] font-bold text-slate-500 bg-slate-100 rounded hover:bg-slate-200 transition-colors" on:click=move |_| set_editing.set(false)>"Cancel"</button>
            </Show>
        </div>
    }
}
