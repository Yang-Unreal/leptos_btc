// ============================================================================
// app.rs —— reactive_stores 细粒度响应性 + keyed <For> + Suspend SSR 流式渲染
// ----------------------------------------------------------------------------
// 架构核心（三大目标的实现方式）：
//
// 1.【细粒度响应性 fine-grained reactivity】
//    使用 reactive_stores::Store：状态树的每个字段（todo.title / todo.completed）
//    都是独立的响应式节点。切换某一条 todo 的 completed，只会通知订阅了
//    【那一个字段】的 DOM 节点（那个 checkbox / 那个 <li> 的 class），
//    <For> 不会重新 diff，其它行的任何闭包都不会重跑，也没有任何 Vec/HashMap 克隆。
//
// 2.【O(1) 时间复杂度】
//    #[store(key: Uuid = |todo| todo.id)] 让 Vec<Todo> 成为"keyed 字段"：
//    每一行拿到的 Field<Todo> 通过 key 定位（内部维护 key→索引映射，O(1)），
//    字段级读写（title.set / completed.update）都是 O(1) 定点操作，
//    彻底替代了旧版 `todo_lookup.get()`（每次读取克隆整个 HashMap，O(n)）
//    和 `iter_mut().find()`（O(n) 线性扫描）。
//
// 3.【SEO / SSR】
//    <Transition> + Suspend::new(async ...) 直接 await Resource：
//    服务器端会等数据查完、把【真实的 todo 列表 HTML】流式渲染进首屏，
//    爬虫拿到的是完整内容（不再需要旧版 Show-fallback 渲染只读副本的 hack）。
//    另外补齐 <Meta> description / Open Graph 标签和语义化 <h1>。
// ============================================================================

use crate::todo::*;
use chrono::Utc;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, MetaTags, Stylesheet, Title};
use reactive_stores::{Field, Store};
use uuid::Uuid;

// ---------------- 响应式状态树 ----------------
// derive(Store) 生成 TodoStateStoreFields trait（提供 .todos() 字段访问器）。
// #[store(key: ...)]：把 Vec<Todo> 变成"按 id 索引"的 keyed 集合，
// 使 <For> 的每一行都能拿到一个稳定的、O(1) 定位的 Field<Todo>。
#[derive(Clone, Debug, Default, Store)]
struct TodoState {
    #[store(key: Uuid = |todo| todo.id)]
    todos: Vec<Todo>,
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
    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>
        // ---- SEO 元信息：标题 + 描述 + Open Graph ----
        <Title text="Todos — Fast Full-Stack Todo App"/>
        <Meta name="description" content="A blazingly fast, server-side rendered todo list built with Leptos, Rust, Axum, and PostgreSQL."/>
        <Meta property="og:title" content="Todos — Fast Full-Stack Todo App"/>
        <Meta property="og:description" content="A blazingly fast, server-side rendered todo list built with Leptos and Rust."/>
        <Meta property="og:type" content="website"/>
        <main>
            <Todos/>
        </main>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    // refetch 计数器：只在服务器操作失败时 +1，触发 Resource 重新拉取服务器真值
    //（乐观更新的"回滚"机制：直接用服务器数据重建整个 Store）。
    let (refetch, set_refetch) = signal(0u64);
    let todos_res = Resource::new(move || refetch.get(), |_| get_todos());

    // 错误提示放在 <Transition> 外层：refetch 重建内部组件时错误消息不会丢失。
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    view! {
        <section
            aria-label="Todo list"
            class="w-full max-w-md sm:max-w-lg md:max-w-xl lg:max-w-2xl xl:max-w-3xl mx-auto bg-white/95 backdrop-blur-sm rounded-2xl sm:rounded-3xl p-8 sm:p-10 md:p-12 shadow-lg sm:shadow-xl border border-white/60"
        >
            // SEO：页面唯一的语义化 <h1>（旧版只有 <h2>，缺少 h1 层级）。
            <h1 class="text-lg sm:text-xl md:text-2xl font-bold tracking-tight text-slate-800 text-center mb-4 sm:mb-6">"Todos"</h1>

            <Show when=move || error_msg.get().is_some()>
                <div role="alert" class="mb-4 px-4 py-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600 flex items-center justify-between">
                    <span>{move || error_msg.get().unwrap_or_default()}</span>
                    <button
                        aria-label="Dismiss error"
                        class="ml-3 text-red-400 hover:text-red-600 font-bold text-lg leading-none cursor-pointer"
                        on:click=move |_| set_error_msg.set(None)
                    >"×"</button>
                </div>
            </Show>

            // Transition（而不是 Suspense）：refetch 时保持旧内容显示，避免闪烁。
            // Suspend + .await：SSR 期间等待数据、把完整列表渲染进首屏 HTML（SEO 关键）。
            <Transition fallback=move || view! { <p class="text-center text-slate-400 text-sm my-4">"Loading todos…"</p> }>
                {move || Suspend::new(async move {
                    match todos_res.await {
                        Ok(list) => view! {
                            <TodoManager initial=list set_refetch set_error_msg/>
                        }
                        .into_any(),
                        Err(e) => view! {
                            <div class="text-center text-sm text-red-500 my-4 p-3 bg-red-50 rounded-xl border border-red-200/60">
                                {format!("Error: {}", e)}
                            </div>
                        }
                        .into_any(),
                    }
                })}
            </Transition>
        </section>
    }
}

// ============================================================================
// TodoManager —— 拥有 Store 的组件：表单 + 列表 + 所有服务器 Action
// 由 Suspend 用服务器数据创建；refetch 时会带着最新的服务器真值重建（回滚）。
// ============================================================================
#[component]
fn TodoManager(
    initial: Vec<Todo>,
    set_refetch: WriteSignal<u64>,
    set_error_msg: WriteSignal<Option<String>>,
) -> impl IntoView {
    let store = Store::new(TodoState { todos: initial });
    let title_ref = NodeRef::<Input>::new();

    // ---- Actions：只负责"通知服务器"，乐观更新已在事件处理器里定点完成。
    //      失败时：显示错误 + refetch（用服务器数据重建 Store，实现回滚）。
    let add = Action::new(move |(id, title): &(Uuid, String)| {
        let id = *id;
        let title = title.clone();
        async move {
            if let Err(e) = add_todo(id, title).await {
                set_error_msg.set(Some(format!("Failed to add: {}", e)));
                set_refetch.update(|n| *n = n.wrapping_add(1));
            }
        }
    });

    let toggle = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = toggle_todo(id).await {
                set_error_msg.set(Some(format!("Failed to toggle: {}", e)));
                set_refetch.update(|n| *n = n.wrapping_add(1));
            }
        }
    });

    let delete = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = delete_todo(id).await {
                set_error_msg.set(Some(format!("Failed to delete: {}", e)));
                set_refetch.update(|n| *n = n.wrapping_add(1));
            }
        }
    });

    let update = Action::new(move |(id, title): &(Uuid, String)| {
        let id = *id;
        let title = title.clone();
        async move {
            if let Err(e) = update_todo(id, title).await {
                set_error_msg.set(Some(format!("Failed to update: {}", e)));
                set_refetch.update(|n| *n = n.wrapping_add(1));
            }
        }
    });

    view! {
        <form
            class="flex flex-col sm:flex-row gap-4 mb-8 sm:mb-10"
            on:submit=move |ev| {
                ev.prevent_default();
                let value = title_ref.get().map(|el| el.value()).unwrap_or_default();
                let value = value.trim().to_string();
                if !value.is_empty() {
                    let id = Uuid::now_v7();
                    // 乐观更新：只写 todos 字段 → keyed <For> 精准插入一行 DOM。
                    store.todos().write().insert(0, Todo {
                        id,
                        title: value.clone(),
                        completed: false,
                        created_at: Utc::now(),
                    });
                    add.dispatch((id, value));
                    if let Some(input) = title_ref.get() { input.set_value(""); }
                }
            }
        >
            <input node_ref=title_ref type="text" placeholder="What needs to be done?"
                aria-label="New todo title"
                class="flex-1 min-w-0 px-5 py-4 text-base sm:text-lg bg-slate-50 border border-slate-200 rounded-xl sm:rounded-2xl focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all duration-200 placeholder:text-slate-400"
            />
            <button type="submit"
                class="px-8 py-4 text-base sm:text-lg font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-xl sm:rounded-2xl shadow-md hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
            >"Add"</button>
        </form>

        <Show
            when=move || !store.todos().read().is_empty()
            fallback=|| view! {
                <p class="text-center text-slate-400 text-sm my-4">"Nothing to do yet — add your first todo!"</p>
            }
        >
            <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">
                // keyed <For>：只有 todos 这个字段本身（插入/删除）被写入时才 diff；
                // 单条 todo 的字段更新（title/completed）完全绕过 <For>，直达对应 DOM。
                <For
                    each=move || store.todos()
                    key=|row| row.read().id
                    children=move |todo| view! {
                        <TodoRow store todo toggle delete update/>
                    }
                />
            </ul>
        </Show>
    }
}

// ============================================================================
// TodoRow —— 单行组件。todo: Field<Todo> 是通过 key O(1) 定位的字段句柄，
// 行内所有读写都是字段级的定点操作，互不影响、不触发列表 diff。
// ============================================================================
#[component]
fn TodoRow(
    store: Store<TodoState>,
    #[prop(into)] todo: Field<Todo>,
    toggle: Action<Uuid, ()>,
    delete: Action<Uuid, ()>,
    update: Action<(Uuid, String), ()>,
) -> impl IntoView {
    // id 永不变化，取一次即可（untracked，不建立订阅）。
    let id = todo.id().get_untracked();
    // 字段句柄（Copy，O(1) 定位）：只有这两个字段的订阅者会因它们的变化而更新。
    let completed = todo.completed();
    let title = todo.title();

    // 编辑态是"行私有"的局部信号：进入/退出编辑只影响本行
    //（旧版用父级 Option<Uuid> 信号，每次变化会重跑所有行的 is_editing 闭包）。
    let (editing, set_editing) = signal(false);
    let edit_ref = NodeRef::<Input>::new();

    Effect::new(move |_| {
        if editing.get() {
            if let Some(input) = edit_ref.get() {
                let _ = input.focus();
            }
        }
    });

    // 保存标题：O(1) 定点写入 title 字段 + 通知服务器。
    let save = move || {
        let value = edit_ref.get().map(|el| el.value()).unwrap_or_default();
        let value = value.trim().to_string();
        if !value.is_empty() {
            title.set(value.clone());
            set_editing.set(false);
            update.dispatch((id, value));
        }
    };

    view! {
        <li
            class:completed=move || completed.get()
            class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white transition-all duration-200 hover:border-slate-300 hover:bg-slate-50 hover:shadow-sm active:scale-[0.998] active:bg-slate-100"
        >
            <Show
                when=move || editing.get()
                fallback=move || view! {
                    <input type="checkbox"
                        aria-label="Toggle completed"
                        class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
                        prop:checked=move || completed.get()
                        on:click=move |_| {
                            // O(1) 定点翻转：只有本行的 checkbox 和 <li> class 更新。
                            completed.update(|c| *c = !*c);
                            toggle.dispatch(id);
                        }
                    />
                    <span class="flex-1 min-w-0 text-base sm:text-lg wrap-break-word leading-relaxed todo-title">
                        {move || title.get()}
                    </span>
                    <button
                        aria-label="Edit todo"
                        class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 active:scale-95 transition-all duration-200 cursor-pointer"
                        on:click=move |_| set_editing.set(true)
                    >"✎"</button>
                    <button
                        aria-label="Delete todo"
                        class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-red-600 hover:bg-red-50 active:scale-95 transition-all duration-200 cursor-pointer"
                        on:click=move |_| {
                            // 结构性变更（删除一行）才写 todos 字段本身，
                            // keyed <For> 只移除这一行对应的 DOM。
                            store.todos().write().retain(|t| t.id != id);
                            delete.dispatch(id);
                        }
                    >"✕"</button>
                }
            >
                <input node_ref=edit_ref type="text"
                    aria-label="Edit todo title"
                    class="flex-1 min-w-0 px-4 py-3 text-base sm:text-lg bg-slate-50 border border-indigo-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/15 transition-all duration-200"
                    prop:value=move || title.get()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            save();
                        } else if ev.key() == "Escape" {
                            set_editing.set(false);
                        }
                    }
                />
                <button
                    class="px-4 py-3 text-sm font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-lg shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200 cursor-pointer"
                    on:click=move |_| save()
                >"Save"</button>
                <button
                    class="px-3 py-2 text-sm font-semibold text-slate-600 bg-slate-100 border border-slate-200 rounded-lg hover:bg-slate-200 hover:-translate-y-0.5 active:translate-y-0 transition-all duration-200 cursor-pointer"
                    on:click=move |_| set_editing.set(false)
                >"Cancel"</button>
            </Show>
        </li>
    }
}
