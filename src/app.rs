// ============================================================================
// app.rs —— 界面(UI) + 前端交互逻辑（Leptos 组件）
// ----------------------------------------------------------------------------
// 【Store 版重构 v3】
//   1. 消除 hydration DOM 销毁：Store 在组件函数体直接初始化，view tree 唯一。
//   2. keyed 迭代 (store.todos().into_iter())：删除/插入时按 Uuid 追踪，
//      只移动受影响的 DOM，不重新渲染无关项。
//   3. 乐观更新失败 toast 提示。

use crate::todo::*;
use chrono::Utc;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use reactive_stores::Store;
use uuid::Uuid;

// ---- Shell ----
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
        <Title text="Todos"/>
        <main>
            <Todos/>
        </main>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    // ==================== 状态定义 ====================

    let (refetch, set_refetch) = signal(0u64);
    let todos = Resource::new(move || refetch.get(), |_| get_todos());

    // ★ Store：客户端细粒度响应式核心。
    //   初始为空；Effect 从 Resource 同步数据（与原始代码行为一致）。
    //   SSR 阶段 Effect 不执行 → Store 为空 → synced=false → 视图展示 "Loading…"。
    //   客户端 hydration 后 Effect 填充 Store → synced=true → 列表出现。
    let store = Store::new(TodoList::default());

    // 错误 toast
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    // synced 标记是否已完成首次同步（用于区分"未加载"和"加载了但为空"）
    let (synced, set_synced) = signal(false);

    // Effect：从 Resource 同步数据到 Store（仅客户端执行）
    Effect::new(move |_| {
        if let Some(result) = todos.get() {
            match result {
                Ok(list) => {
                    store.todos().set(list);
                    set_synced.set(true);
                }
                Err(e) => {
                    set_error_msg.set(Some(format!("Failed to load: {}", e)));
                    set_synced.set(true);
                }
            }
        }
    });
    // 自动清除 toast
    let clear_error = move || {
        set_error_msg.set(None);
    };

    let title_ref = NodeRef::<Input>::new();

    // ==================== Actions ====================

    let add = Action::new(move |title: &String| {
        let title = title.clone();
        async move {
            let new_id = Uuid::now_v7();
            store.todos().write().insert(0, Todo {
                id: new_id,
                title: title.clone(),
                completed: false,
                created_at: Utc::now(),
            });

            if let Err(e) = add_todo(new_id, title).await {
                store.todos().write().retain(|t| t.id != new_id);
                set_refetch.update(|n| *n = n.wrapping_add(1));
                set_error_msg.set(Some(format!("Failed to add: {}", e)));
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let toggle = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = toggle_todo(id).await {
                set_refetch.update(|n| *n = n.wrapping_add(1));
                set_error_msg.set(Some(format!("Failed to toggle: {}", e)));
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let delete = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if let Err(e) = delete_todo(id).await {
                set_refetch.update(|n| *n = n.wrapping_add(1));
                set_error_msg.set(Some(format!("Failed to delete: {}", e)));
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let (editing, set_editing) = signal(Option::<Uuid>::None);

    let update = Action::new(move |(id, title): &(Uuid, String)| {
        let id = *id;
        let title = title.clone();
        async move {
            if let Err(e) = update_todo(id, title).await {
                set_refetch.update(|n| *n = n.wrapping_add(1));
                set_error_msg.set(Some(format!("Failed to update: {}", e)));
            }
            Ok::<(), ServerFnError>(())
        }
    });

    // ==================== 视图渲染 ====================

    view! {
        <section class="w-full max-w-md sm:max-w-lg md:max-w-xl lg:max-w-2xl xl:max-w-3xl mx-auto bg-white/95 backdrop-blur-sm rounded-2xl sm:rounded-3xl p-8 sm:p-10 md:p-12 shadow-lg sm:shadow-xl border border-white/60">
            <h2 class="text-lg sm:text-xl md:text-2xl font-bold tracking-tight text-slate-800 text-center mb-4 sm:mb-6">"Todos"</h2>

            {/* 错误 toast：自动 3 秒消失 */}
            <Show when=move || error_msg.get().is_some()>
                <div class="mb-4 px-4 py-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600 flex items-center justify-between animate-in fade-in">
                    <span>{move || error_msg.get().unwrap_or_default()}</span>
                    <button
                        class="ml-3 text-red-400 hover:text-red-600 font-bold text-lg leading-none cursor-pointer"
                        on:click=move |_| clear_error()
                    >"×"</button>
                </div>
            </Show>

            <form
                class="flex flex-col sm:flex-row gap-4 mb-8 sm:mb-10"
                on:submit=move |ev| {
                    ev.prevent_default();
                    let value = title_ref.get().map(|el| el.value()).unwrap_or_default();
                    if !value.trim().is_empty() {
                        add.dispatch(value);
                        if let Some(input) = title_ref.get() {
                            input.set_value("");
                        }
                    }
                }
            >
                <input
                    node_ref=title_ref
                    type="text"
                    placeholder="What needs to be done?"
                    class="flex-1 min-w-0 px-5 py-4 text-base sm:text-lg bg-slate-50 border border-slate-200 rounded-xl sm:rounded-2xl focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all duration-200 placeholder:text-slate-400"
                />
                <button
                    type="submit"
                    class="px-8 py-4 text-base sm:text-lg font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-xl sm:rounded-2xl shadow-md hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
                >"Add"</button>
            </form>

            <Transition fallback=move || view! { <p>"Loading todos…"</p> }>
                <Show
                    when=move || synced.get()
                    fallback=move || match todos.get() {
                        Some(Err(e)) => view! {
                            <div class="text-center text-sm text-red-500 my-4 p-3 bg-red-50 rounded-xl border border-red-200/60">
                                {format!("Error: {}", e)}
                            </div>
                        }.into_any(),
                        _ => view! {
                            <div class="text-center text-slate-400 text-sm my-4 leading-relaxed">
                                {String::from("Loading…")}
                            </div>
                        }.into_any(),
                    }
                >
                    <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">
                        {
                            // ★★★ keyed Store 迭代 ★★★
                            //   store.todos().into_iter() 按 Uuid 做 keyed 追踪：
                            //   - 删除 #3：仅移除 #3 的 DOM，#4+ 的 DOM 保留不动
                            //   - 字段更新 (toggle/edit)：仅目标字段的 DOM 被更新
                            //   - 无 O(N²)，无全量 Clone，无 hydration DOM 销毁
                            store.todos().into_iter().map(|todo_field| {
                                let id = todo_field.id().get();
                                let completed = todo_field.completed();
                                let title = todo_field.title();
                                let edit_ref = NodeRef::<Input>::new();
                                let is_editing = move || editing.get() == Some(id);

                                view! {
                                    <li
                                        class:completed=move || completed.get()
                                        class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white transition-all duration-200 hover:border-slate-300 hover:bg-slate-50 hover:shadow-sm active:scale-[0.998] active:bg-slate-100"
                                    >
                                        <Show
                                            when=is_editing
                                            fallback=move || view! {
                                                <input
                                                    type="checkbox"
                                                    class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
                                                    prop:checked=move || completed.get()
                                                    on:click=move |_| {
                                                        todo_field.completed().update(|c| *c = !*c);
                                                        toggle.dispatch(id);
                                                    }
                                                />
                                                <span class="flex-1 min-w-0 text-base sm:text-lg wrap-break-word leading-relaxed todo-title">
                                                    {move || title.get()}
                                                </span>
                                                <button
                                                    class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 active:scale-95 transition-all duration-200 cursor-pointer"
                                                    on:click=move |_| set_editing.set(Some(id))
                                                >"✎"</button>
                                                <button
                                                    class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-red-600 hover:bg-red-50 active:scale-95 transition-all duration-200 cursor-pointer"
                                                    on:click=move |_| {
                                                        store.todos().write().retain(|t| t.id != id);
                                                        delete.dispatch(id);
                                                    }
                                                >"✕"</button>
                                            }
                                        >
                                            <input
                                                node_ref=edit_ref
                                                type="text"
                                                class="flex-1 min-w-0 px-4 py-3 text-base sm:text-lg bg-slate-50 border border-indigo-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/15 transition-all duration-200"
                                                prop:value=move || title.get()
                                            />
                                            <button
                                                class="px-4 py-3 text-sm font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-lg shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200 cursor-pointer"
                                                on:click=move |_| {
                                                    let value = edit_ref.get().map(|el| el.value()).unwrap_or_default();
                                                    todo_field.title().set(value.clone());
                                                    set_editing.set(None);
                                                    update.dispatch((id, value));
                                                }
                                            >"Save"</button>
                                            <button
                                                class="px-3 py-2 text-sm font-semibold text-slate-600 bg-slate-100 border border-slate-200 rounded-lg hover:bg-slate-200 hover:-translate-y-0.5 active:translate-y-0 transition-all duration-200 cursor-pointer"
                                                on:click=move |_| set_editing.set(None)
                                            >"Cancel"</button>
                                        </Show>
                                    </li>
                                }
                            }).collect_view()
                        }
                    </ul>
                </Show>
            </Transition>
        </section>
    }
}
