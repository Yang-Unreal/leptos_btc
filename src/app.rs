// ============================================================================
// app.rs —— RwSignal + <For> + Memo<HashMap> O(1) lookup + Suspense SSR
// ----------------------------------------------------------------------------
// 核心原则：
//   1. <For> children 用 Memo<HashMap> 做 O(1) 查找，避免 O(n) 线性遍历
//   2. todos_local.set/update 触发 <For> keyed diff → 仅变动项重渲染
//   3. SSR：Suspense 包裹 + Show fallback 读 todos.get() → 静态列表可被索引

use crate::todo::*;
use chrono::Utc;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use std::collections::HashMap;
use uuid::Uuid;

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
    let (refetch, set_refetch) = signal(0u64);
    let todos = Resource::new(move || refetch.get(), |_| get_todos());

    let todos_local = RwSignal::new(Vec::<Todo>::new());

    let todo_lookup = Memo::new(move |_| {
        todos_local.with(|list| {
            list.iter().map(|t| (t.id, t.clone())).collect::<HashMap<_, _>>()
        })
    });

    Effect::new(move |_| {
        if let Some(Ok(list)) = todos.get() {
            todos_local.set(list);
        }
    });

    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let title_ref = NodeRef::<Input>::new();

    // ==================== Actions ====================

    let add = Action::new(move |title: &String| {
        let title = title.clone();
        async move {
            let new_id = Uuid::now_v7();
            let new_todo = Todo {
                id: new_id, title: title.clone(), completed: false, created_at: Utc::now(),
            };
            todos_local.update(|list| list.insert(0, new_todo));
            if let Err(e) = add_todo(new_id, title).await {
                todos_local.update(|list| list.retain(|t| t.id != new_id));
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

    // ==================== 视图 ====================
    view! {
        <section class="w-full max-w-md sm:max-w-lg md:max-w-xl lg:max-w-2xl xl:max-w-3xl mx-auto bg-white/95 backdrop-blur-sm rounded-2xl sm:rounded-3xl p-8 sm:p-10 md:p-12 shadow-lg sm:shadow-xl border border-white/60">
            <h2 class="text-lg sm:text-xl md:text-2xl font-bold tracking-tight text-slate-800 text-center mb-4 sm:mb-6">"Todos"</h2>

            <Show when=move || error_msg.get().is_some()>
                <div class="mb-4 px-4 py-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600 flex items-center justify-between">
                    <span>{move || error_msg.get().unwrap_or_default()}</span>
                    <button class="ml-3 text-red-400 hover:text-red-600 font-bold text-lg leading-none cursor-pointer"
                        on:click=move |_| set_error_msg.set(None)>"×"</button>
                </div>
            </Show>

            <form
                class="flex flex-col sm:flex-row gap-4 mb-8 sm:mb-10"
                on:submit=move |ev| {
                    ev.prevent_default();
                    let value = title_ref.get().map(|el| el.value()).unwrap_or_default();
                    if !value.trim().is_empty() {
                        add.dispatch(value);
                        if let Some(input) = title_ref.get() { input.set_value(""); }
                    }
                }
            >
                <input node_ref=title_ref type="text" placeholder="What needs to be done?"
                    class="flex-1 min-w-0 px-5 py-4 text-base sm:text-lg bg-slate-50 border border-slate-200 rounded-xl sm:rounded-2xl focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all duration-200 placeholder:text-slate-400"
                />
                <button type="submit"
                    class="px-8 py-4 text-base sm:text-lg font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-xl sm:rounded-2xl shadow-md hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
                >"Add"</button>
            </form>

            <Suspense fallback=move || view! { <p class="text-center text-slate-400 text-sm my-4">"Loading todos…"</p> }>
                <Show
                    when=move || !todos_local.get().is_empty()
                    fallback=move || {
                        match todos.get() {
                            Some(Ok(list)) if !list.is_empty() => view! {
                                <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">
                                    {list.into_iter().map(|todo| view! {
                                        <li class:completed=todo.completed
                                            class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white">
                                            <input type="checkbox" class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500"
                                                checked=todo.completed disabled />
                                            <span class="flex-1 min-w-0 text-base sm:text-lg">{todo.title}</span>
                                        </li>
                                    }).collect_view()}
                                </ul>
                            }.into_any(),
                            Some(Err(e)) => view! {
                                <div class="text-center text-sm text-red-500 my-4 p-3 bg-red-50 rounded-xl border border-red-200/60">
                                    {format!("Error: {}", e)}
                                </div>
                            }.into_any(),
                            _ => view! {
                                <div class="text-center text-slate-400 text-sm my-4">"Loading…"</div>
                            }.into_any(),
                        }
                    }
                >
                    <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">
                        <For
                            each=move || todos_local.get()
                            key=|todo: &Todo| todo.id
                            children=move |todo: Todo| {
                                let id = todo.id;
                                let edit_ref = NodeRef::<Input>::new();
                                let is_editing = move || editing.get() == Some(id);

                                let completed = move || {
                                    todo_lookup.get().get(&id).map(|t| t.completed).unwrap_or_default()
                                };
                                let title = move || {
                                    todo_lookup.get().get(&id).map(|t| t.title.clone()).unwrap_or_default()
                                };

                                let _ = Effect::new(move |_| {
                                    if is_editing() {
                                        if let Some(input) = edit_ref.get() {
                                            let _ = input.focus();
                                        }
                                    }
                                });

                                view! {
                                    <li
                                        class:completed=completed
                                        class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white transition-all duration-200 hover:border-slate-300 hover:bg-slate-50 hover:shadow-sm active:scale-[0.998] active:bg-slate-100"
                                    >
                                        <Show
                                            when=is_editing
                                            fallback=move || view! {
                                                <input type="checkbox"
                                                    class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
                                                    prop:checked=completed
                                                    on:click=move |_| {
                                                        todos_local.update(|list| {
                                                            if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                                                                t.completed = !t.completed;
                                                            }
                                                        });
                                                        toggle.dispatch(id);
                                                    }
                                                />
                                                <span class="flex-1 min-w-0 text-base sm:text-lg wrap-break-word leading-relaxed todo-title">
                                                    {title}
                                                </span>
                                                <button class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 active:scale-95 transition-all duration-200 cursor-pointer"
                                                    on:click=move |_| set_editing.set(Some(id))>"✎"</button>
                                                <button class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-red-600 hover:bg-red-50 active:scale-95 transition-all duration-200 cursor-pointer"
                                                    on:click=move |_| {
                                                        todos_local.update(|list| list.retain(|t| t.id != id));
                                                        delete.dispatch(id);
                                                    }>"✕"</button>
                                        }
                                    >
                                        <input node_ref=edit_ref type="text"
                                            class="flex-1 min-w-0 px-4 py-3 text-base sm:text-lg bg-slate-50 border border-indigo-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/15 transition-all duration-200"
                                            prop:value=title
                                            on:keydown=move |ev| {
                                                if ev.key() == "Enter" {
                                                    let value = edit_ref.get().map(|el| el.value()).unwrap_or_default();
                                                    if !value.trim().is_empty() {
                                                        todos_local.update(|list| {
                                                            if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                                                                t.title = value.clone();
                                                            }
                                                        });
                                                        set_editing.set(None);
                                                        update.dispatch((id, value));
                                                    }
                                                } else if ev.key() == "Escape" {
                                                    set_editing.set(None);
                                                }
                                            }
                                        />
                                        <button class="px-4 py-3 text-sm font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-lg shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200 cursor-pointer"
                                            on:click=move |_| {
                                                let value = edit_ref.get().map(|el| el.value()).unwrap_or_default();
                                                todos_local.update(|list| {
                                                    if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                                                        t.title = value.clone();
                                                    }
                                                });
                                                set_editing.set(None);
                                                update.dispatch((id, value));
                                            }>"Save"</button>
                                        <button class="px-3 py-2 text-sm font-semibold text-slate-600 bg-slate-100 border border-slate-200 rounded-lg hover:bg-slate-200 hover:-translate-y-0.5 active:translate-y-0 transition-all duration-200 cursor-pointer"
                                            on:click=move |_| set_editing.set(None)>"Cancel"</button>
                                    </Show>
                                </li>
                            }
                        }
                        />
                    </ul>
                </Show>
            </Suspense>
        </section>
    }
}
