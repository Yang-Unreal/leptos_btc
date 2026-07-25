use crate::todo::*;
use chrono::Utc;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet, Title};
use uuid::Uuid;

// ============================================================================
// 纯原生细粒度模型 (Vanilla Fine-grained Reactivity)
// 抛弃第三方 Store，完全使用 Leptos 原生 RwSignal 构建状态树。
// ============================================================================
#[derive(Clone, Debug)]
pub struct TodoRx {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
    // 核心重构：引入逻辑删除状态，实现严格 O(1) 的删除复杂度
    pub deleted: RwSignal<bool>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<Todo> for TodoRx {
    fn from(todo: Todo) -> Self {
        Self {
            id: todo.id,
            title: RwSignal::new(todo.title),
            completed: RwSignal::new(todo.completed),
            deleted: RwSignal::new(false),
            created_at: todo.created_at,
        }
    }
}

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

                // 【SEO优化：JSON-LD 结构化数据】
                <script type="application/ld+json">
                    {r#"
                    {
                      "@context": "https://schema.org",
                      "@type": "WebApplication",
                      "name": "Todos",
                      "url": "https://todos.example.com",
                      "applicationCategory": "ProductivityApplication",
                      "operatingSystem": "All",
                      "description": "A blazingly fast, server-side rendered todo list built with Leptos and Rust."
                    }
                    "#}
                </script>
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
    let site_url = "https://todos.example.com";

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>

        // 【SEO优化：全套 Meta 标签与 Canonical URL】
        <Title text="Todos — Fast Full-Stack Todo App"/>
        <Meta name="description" content="A blazingly fast, server-side rendered todo list built with Leptos, Rust, Axum, and PostgreSQL."/>
        <Meta name="theme-color" content="#4f46e5"/>
        <Link rel="canonical" href=site_url/>

        // Open Graph
        <Meta property="og:type" content="website"/>
        <Meta property="og:url" content=site_url/>
        <Meta property="og:title" content="Todos — Fast Full-Stack Todo App"/>
        <Meta property="og:description" content="A blazingly fast, server-side rendered todo list built with Leptos and Rust."/>
        <Meta property="og:image" content=format!("{}/og-image.jpg", site_url)/>

        // Twitter Cards
        <Meta name="twitter:card" content="summary_large_image"/>
        <Meta name="twitter:title" content="Todos — Fast Full-Stack Todo App"/>
        <Meta name="twitter:description" content="A blazingly fast, server-side rendered todo list built with Leptos and Rust."/>
        <Meta name="twitter:image" content=format!("{}/twitter-image.jpg", site_url)/>

        // 【SEO优化：语义化 HTML 地标 (Landmarks)】
        <div class="w-full min-h-screen bg-linear-to-b from-slate-50 via-white to-slate-100 font-sans text-slate-800 antialiased flex flex-col">
            <header class="sticky top-0 z-50 w-full bg-white/80 backdrop-blur-xl border-b border-slate-200/60 shadow-sm">
                <div class="w-full px-4 sm:px-6 lg:px-8 h-14 sm:h-16 flex items-center justify-between">
                    <div class="text-lg sm:text-xl font-extrabold bg-linear-to-br from-indigo-600 to-violet-600 bg-clip-text text-transparent tracking-tight">
                        "Todos"
                    </div>
                    <nav aria-label="Main Navigation" class="flex items-center gap-4">
                        <a href="/" class="text-sm font-medium text-slate-500 hover:text-indigo-600 transition-colors duration-200">
                            "Tasks"
                        </a>
                    </nav>
                </div>
            </header>

            <main class="flex-1 w-full px-4 sm:px-6 lg:px-8 py-6 sm:py-8 lg:py-10 flex flex-col">
                <Todos/>
            </main>
        </div>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    let (refetch, set_refetch) = signal(0u64);

    // Resource::new_blocking 保证绝对的流式阻塞，服务端查库完成前不输出 HTML，爬虫完美抓取。
    let todos_res = Resource::new_blocking(move || refetch.get(), |_| get_todos());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    view! {
        <section
            aria-labelledby="todo-heading"
            class="w-full bg-white/80 backdrop-blur-md rounded-2xl sm:rounded-3xl p-6 sm:p-8 lg:p-10 shadow-[0_1px_3px_rgba(0,0,0,0.04),0_8px_30px_rgba(0,0,0,0.04)] border border-slate-200/50 hover:shadow-[0_1px_3px_rgba(0,0,0,0.06),0_12px_40px_rgba(0,0,0,0.06)] transition-shadow duration-300"
        >
            <h1 id="todo-heading" class="text-xl sm:text-2xl font-bold tracking-tight text-slate-800 text-center mb-1 sm:mb-2">
                "Task Dashboard"
            </h1>
            <p class="text-center text-xs sm:text-sm text-slate-400 mb-6 sm:mb-8">
                "Stay organized. Get things done."
            </p>

            <Show when=move || error_msg.get().is_some()>
                <div role="alert" class="mb-6 px-4 py-3 bg-red-50 border border-red-200/60 rounded-xl text-sm text-red-600 flex items-center justify-between shadow-sm">
                    <span class="flex items-center gap-2">
                        <svg class="w-4 h-4 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/></svg>
                        {move || error_msg.get().unwrap_or_default()}
                    </span>
                    <button class="ml-3 text-red-400 hover:text-red-600 font-bold text-lg leading-none transition-colors duration-150" on:click=move |_| set_error_msg.set(None)>"×"</button>
                </div>
            </Show>

            <Transition fallback=move || view! { <p class="text-center text-slate-400 text-sm my-8 animate-pulse">"Loading tasks…"</p> }>
                {move || Suspend::new(async move {
                    match todos_res.await {
                        Ok(list) => view! { <TodoManager initial=list set_refetch set_error_msg/> }.into_any(),
                        Err(e) => view! {
                            <div class="text-center text-sm text-red-500 my-6 p-4 bg-red-50/60 rounded-xl border border-red-100">
                                {format!("Error: {}", e)}
                            </div>
                        }.into_any(),
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
    // 转换为原生 Signal 模型
    let initial_rx: Vec<TodoRx> = initial.into_iter().map(TodoRx::from).collect();
    let todos_sig = RwSignal::new(initial_rx);

    // 【深度响应式应用：create_memo】
    // 演示真正的细粒度衍生状态：实时计算未完成且未删除的 Todo 数量。
    // 该 Memo 追踪了 todos_sig、内部的 completed 和 deleted 多个信号维度。
    let active_count = Memo::new(move |_| {
        todos_sig
            .read()
            .iter()
            .filter(|t| !t.completed.get() && !t.deleted.get())
            .count()
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
            class="flex flex-col sm:flex-row gap-3 sm:gap-4 mb-8 sm:mb-10 w-full"
            on:submit=move |ev| {
                ev.prevent_default();
                let value = title_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                if !value.is_empty() {
                    let id = Uuid::now_v7();

                    // 【O(1) 内存插入】
                    // 使用 push (均摊 O(1)) 替代 insert(0) 避免数组平移。
                    todos_sig.update(|t| t.push(TodoRx {
                        id,
                        title: RwSignal::new(value.clone()),
                        completed: RwSignal::new(false),
                        deleted: RwSignal::new(false),
                        created_at: Utc::now(),
                    }));

                    add.dispatch((id, value));
                    if let Some(input) = title_ref.get() { input.set_value(""); }
                }
            }
        >
            <input node_ref=title_ref type="text" placeholder="What needs to be done?"
                aria-label="New todo title"
                class="flex-1 px-5 py-3.5 text-base bg-slate-50 border border-slate-200 rounded-xl text-slate-800 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500/30 focus:border-indigo-400 transition-all duration-200 shadow-sm hover:shadow-md"
            />
            <button type="submit" class="px-6 py-3.5 text-sm font-semibold text-white bg-linear-to-br from-indigo-500 to-violet-500 rounded-xl shadow-md shadow-indigo-500/20 hover:shadow-lg hover:shadow-indigo-500/25 hover:from-indigo-600 hover:to-violet-600 active:scale-[0.97] transition-all duration-200">
                "Add"
            </button>
        </form>

        <Show
            when=move || !todos_sig.read().is_empty()
            fallback=|| view! { <p class="text-center text-slate-400 text-sm py-8">No tasks yet.</p> }
        >
            // CSS flex-col-reverse 将 push 到队尾的新元素在视觉上渲染至最顶部，完美协同底层 O(1)。
            <ul class="w-full list-none m-0 p-0 flex flex-col-reverse gap-2.5 sm:gap-3">
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
    let id = todo.id;
    let title = todo.title;
    let completed = todo.completed;
    let deleted = todo.deleted;

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
        <Show when=move || !deleted.get()>
            <li
                class:completed=move || completed.get()
                class="group w-full flex items-center gap-3 sm:gap-4 p-3 sm:p-4 border border-slate-100 rounded-xl bg-white/80 backdrop-blur-sm transition-all duration-200 hover:bg-white hover:shadow-md hover:border-slate-200/60 active:scale-[0.998]"
            >
                <Show
                    when=move || editing.get()
                    fallback=move || view! { <TodoDisplay id=id title=title completed=completed toggle=toggle delete=delete deleted=deleted set_editing=set_editing/> }
                >
                    <TodoEdit id=id title=title set_editing=set_editing update=update edit_ref=edit_ref/>
                </Show>
            </li>
        </Show>
    }
}

#[component]
fn TodoDisplay(
    id: Uuid,
    title: RwSignal<String>,
    completed: RwSignal<bool>,
    toggle: Action<Uuid, ()>,
    delete: Action<Uuid, ()>,
    deleted: RwSignal<bool>,
    set_editing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <input type="checkbox"
            aria-label="Toggle completed"
            class="w-5 h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
            prop:checked=move || completed.get()
            on:click=move |_| {
                completed.update(|c| *c = !*c);
                toggle.dispatch(id);
            }
        />
        <span class="flex-1 min-w-0 text-sm sm:text-base text-slate-700 wrap-break-word todo-title leading-relaxed transition-all duration-200">
            {move || title.get()}
        </span>
        <div class="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
            <button class="p-1.5 text-slate-400 hover:text-indigo-500 rounded-lg hover:bg-indigo-50 transition-colors duration-150" aria-label="Edit todo" on:click=move |_| set_editing.set(true)>
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
            </button>
            <button class="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 transition-colors duration-150" aria-label="Delete todo" on:click=move |_| {
                deleted.set(true);
                delete.dispatch(id);
            }>
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
            </button>
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
            class="flex-1 px-3 py-2 text-sm bg-slate-50 border-2 border-indigo-300 rounded-lg focus:outline-none focus:ring-1 focus:ring-indigo-500/20 transition-all duration-200"
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
        <button class="px-4 py-2 text-xs font-semibold text-white bg-indigo-500 rounded-lg hover:bg-indigo-600 active:scale-95 transition-all duration-150 shadow-sm" on:click=move |_| {
            let value = edit_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
            if !value.is_empty() {
                title.set(value.clone());
                set_editing.set(false);
                update.dispatch((id, value));
            }
        }>"Save"</button>
        <button class="px-3 py-2 text-xs font-semibold text-slate-500 bg-slate-100 rounded-lg hover:bg-slate-200 active:scale-95 transition-all duration-150" on:click=move |_| set_editing.set(false)>"Cancel"</button>
    }
}
