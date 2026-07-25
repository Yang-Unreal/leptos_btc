use crate::todo::*;
use chrono::Utc;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet, Title};
use uuid::Uuid;

// ----------------------------------------------------------------------------
// 纯原生细粒度模型 (Vanilla Fine-grained Reactivity)
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct TodoRx {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
    pub deleted: RwSignal<bool>, // 墓碑标记 (Tombstone marker)
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

// ----------------------------------------------------------------------------
// 全局上下文：用于跨组件的 O(1) 计数与墓碑回收机制
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct TodoContext {
    todos_sig: RwSignal<Vec<TodoRx>>,
    active_count: RwSignal<usize>,    // 严格 O(1) 的数学计数，拒绝迭代
    tombstone_count: RwSignal<usize>, // 幽灵节点计数，用于平摊 O(1) 的垃圾回收
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

        // 全套 SEO 元信息
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
        <Meta name="twitter:title" content="Todos — Fast Full-Stack Todo App"/>
        <Meta name="twitter:description" content="A blazingly fast, server-side rendered todo list built with Leptos and Rust."/>
        <Meta name="twitter:image" content=format!("{}/twitter-image.jpg", site_url)/>

        <div class="w-full min-h-screen bg-linear-to-b from-slate-50 via-white to-slate-100 font-sans text-slate-800 antialiased flex flex-col">
            <header class="sticky top-0 z-50 w-full bg-white/80 backdrop-blur-xl border-b border-slate-200/60 shadow-sm">
                <div class="w-full px-4 sm:px-6 lg:px-8 h-14 sm:h-16 flex items-center justify-between">
                    <div class="text-lg sm:text-xl font-extrabold bg-linear-to-br from-indigo-600 to-violet-600 bg-clip-text text-transparent tracking-tight">
                        "Todos"
                    </div>
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

    // 【完美优化 1：彻底消灭 O(N) 遍历计数】
    // 将计数器作为一个独立的数学变量，而不是依赖数组的衍生计算
    let initial_count = initial_rx
        .iter()
        .filter(|t| !t.completed.get_untracked() && !t.deleted.get_untracked())
        .count();

    let todos_sig = RwSignal::new(initial_rx);
    let active_count = RwSignal::new(initial_count);
    let tombstone_count = RwSignal::new(0usize); // 幽灵节点水位线

    provide_context(TodoContext {
        todos_sig,
        active_count,
        tombstone_count,
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
            // 直接读取数字信号，O(1)
            {move || format!("{} active task{}", active_count.get(), if active_count.get() == 1 { "" } else { "s" })}
        </div>

        <form
            class="flex flex-col sm:flex-row gap-3 sm:gap-4 mb-8 w-full"
            on:submit=move |ev| {
                ev.prevent_default();
                let value = title_ref.get().map(|el| el.value()).unwrap_or_default().trim().to_string();
                if !value.is_empty() {
                    let id = Uuid::now_v7();
                    // O(1) 内存插入
                    todos_sig.update(|t| t.push(TodoRx {
                        id, title: RwSignal::new(value.clone()), completed: RwSignal::new(false), deleted: RwSignal::new(false), created_at: Utc::now()
                    }));
                    // O(1) 计数更新
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
    // 注入上下文以执行 O(1) 的更新和垃圾回收
    let ctx = expect_context::<TodoContext>();

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
        // DOM 的 O(1) 瞬间卸载
        <Show when=move || !deleted.get()>
            <li class:completed=move || completed.get() class="group w-full flex items-center gap-3 p-3 border border-slate-100 rounded-xl bg-white/80 transition-all">
                <Show when=move || editing.get() fallback=move || view! {
                    <TodoDisplay id title completed toggle delete deleted set_editing ctx/>
                }>
                    <TodoEdit id title set_editing update edit_ref/>
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
    ctx: TodoContext,
) -> impl IntoView {
    view! {
           <input type="checkbox"
               class="w-5 h-5 accent-indigo-500 cursor-pointer flex-none"
               prop:checked=move || completed.get()
               on:click=move |_| {
                   let current = completed.get_untracked();
                   completed.set(!current); // O(1) 状态反转

                   // O(1) 纯数学增减，不遍历数组！
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
               <button class="p-1.5 text-slate-400 hover:text-indigo-500" on:click=move |_| set_editing.set(true)>"✎"</button>
               <button class="p-1.5 text-slate-400 hover:text-red-500" on:click=move |_| {
                   // 1. O(1) UI 隐藏
                   deleted.set(true);

                   // 2. O(1) 同步活跃计数
                   if !completed.get_untracked() {
                       ctx.active_count.update(|c| *c = c.saturating_sub(1));
                   }

                   // 【完美优化 2：平摊 O(1) 垃圾回收机制 (Tombstone Sweeping)】
                   // 解决内存泄漏与渲染幽灵节点的负担，同时避免了次次 O(N) 移动数组。
                   ctx.tombstone_count.update(|c| *c += 1);
                   if ctx.tombstone_count.get_untracked() >= 30 {
                       // 当幽灵节点堆积超过 30 个时，集中做一次 O(N) 物理大清理
                       ctx.todos_sig.update(|t| t.retain(|todo| !todo.deleted.get_untracked()));
                       ctx.tombstone_count.set(0); // 清空水位线
                   }

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
                        title.set(value.clone()); // O(1)
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
