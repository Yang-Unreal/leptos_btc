// ============================================================================
// app.rs —— 界面(UI) + 前端交互逻辑（Leptos 组件）
// ----------------------------------------------------------------------------
// 这个文件定义“页面长什么样”和“点了按钮之后怎么反应”。它同时用于两端：
//   - 服务器端：SSR 时被执行一遍，生成首屏 HTML 字符串。
//   - 浏览器端：hydrate 时再执行一遍，把交互逻辑接到 HTML 上。

use crate::todo::*;
use chrono::Utc;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use uuid::Uuid; // 管理 <head> 里的元信息

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
        // 往 <head> 注入一个样式表链接。id="leptos" 让 cargo-leptos 能热重载这份 CSS。
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>

        // 设置浏览器标签页标题。
        <Title text="Todos"/>

        // 真正的应用界面：一个 <main> 里放我们的 Todos 组件。
        <main>
            <Todos/>
        </main>
    }
}

/// 一个由 Postgres 支撑的全栈 CRUD 示例组件。
/// （三个斜杠 /// 是“文档注释”，会被 rustdoc 收录成 API 文档。）
#[component]
pub fn Todos() -> impl IntoView {
    // ---------- 状态定义区 ----------

    let (refetch, set_refetch) = signal(0);

    let todos = Resource::new(move || refetch.get(), |_| get_todos());
    let todos_local = RwSignal::new(None::<Vec<Todo>>);

    Effect::new(move |_| {
        if let Some(Ok(list)) = todos.get() {
            todos_local.set(Some(list)); // 用服务器数据覆盖本地镜像
        }
    });

    let title_ref = NodeRef::<Input>::new();

    let add = Action::new(move |title: &String| {
        let title = title.clone();
        async move {
            let new_id = Uuid::now_v7();

            todos_local.update(|opt| {
                if let Some(list) = opt {
                    let mut new_list = list.clone();
                    new_list.insert(
                        0,
                        Todo {
                            id: new_id,
                            title: title.clone(),
                            completed: false,
                            created_at: Utc::now(),
                        },
                    );
                    *opt = Some(new_list);
                }
            });

            match add_todo(new_id, title).await {
                Ok(_) => {}
                Err(_) => {
                    todos_local.update(|opt| {
                        if let Some(list) = opt {
                            list.retain(|t| t.id != new_id);
                        }
                    });
                    set_refetch.update(|n| *n += 1);
                }
            }

            Ok::<(), ServerFnError>(())
        }
    });

    let toggle = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if toggle_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let delete = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            if delete_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let (editing, set_editing) = signal(Option::<Uuid>::None);

    let update = Action::new(move |args: &(Uuid, String)| {
        let (id, title) = args;
        let id = *id;
        let title = title.clone();
        async move {
            if update_todo(id, title).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    // ---------- 视图渲染区 ----------
    view! {
        <section class="w-full max-w-md sm:max-w-lg md:max-w-xl lg:max-w-2xl xl:max-w-3xl mx-auto bg-white/95 backdrop-blur-sm rounded-2xl sm:rounded-3xl p-8 sm:p-10 md:p-12 shadow-lg sm:shadow-xl border border-white/60">
            <h2 class="text-lg sm:text-xl md:text-2xl font-bold tracking-tight text-slate-800 text-center mb-4 sm:mb-6">"Todos"</h2>

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

                // 1. <Show> 替代最外层的 match 表达式
                <Show
                    when=move || todos_local.with(|opt| opt.is_some())
                    // 如果本地没数据，用 fallback 处理服务端的报错或继续显示 Loading
                    fallback=move || match todos.get() {
                        Some(Err(e)) => view! {
                            <div class="text-center text-sm text-red-500 my-4 p-3 bg-red-50 rounded-xl border border-red-200/60">
                                {format!("Error: {}", e)}
                            </div>
                        }.into_view(),
                        _ => view! {
                            <div class="text-center text-slate-400 text-sm my-4 leading-relaxed">
                                {String::from("Loading…")}
                            </div>
                        }.into_view(),
                    }
                >
                    <div>
                        <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">

                            // 2. <For> 替代 .into_iter().map().collect_view()
                            // 它通过 key 缓存 DOM 节点，极大提高性能！
                            <For
                                each=move || todos_local.get().unwrap_or_default()
                                key=|todo| todo.id
                                children=move |todo| {
                                    let id = todo.id;
                                    let edit_ref = NodeRef::<Input>::new();

                                    // 派生信号1：当前项是否在编辑
                                    let is_editing = move || editing.get() == Some(id);

                                    // 【非常重要】：派生信号2 & 3
                                    // 因为 <For> 在 id 不变时【不会】重新执行此闭包（保留了DOM节点）。
                                    // 所以为了让 completed 和 title 的改变能反映到 UI 上，
                                    // 必须动态地去 todos_local 中查当前最新值。
                                    let title = move || {
                                        todos_local.with(|opt| {
                                            opt.as_ref()
                                                .and_then(|l| l.iter().find(|t| t.id == id).map(|t| t.title.clone()))
                                                .unwrap_or_default()
                                        })
                                    };
                                    let completed = move || {
                                        todos_local.with(|opt| {
                                            opt.as_ref()
                                                .and_then(|l| l.iter().find(|t| t.id == id).map(|t| t.completed))
                                                .unwrap_or_default()
                                        })
                                    };

                                    view! {
                                        // 这里绑定的是上面派生的 completed 闭包（响应式）
                                        <li class:completed=completed class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white transition-all duration-200 hover:border-slate-300 hover:bg-slate-50 hover:shadow-sm active:scale-[0.998] active:bg-slate-100">

                                            // 3. <Show> 替代内层的 if/else，彻底消灭 .into_any()
                                            <Show
                                                when=is_editing
                                                // 【展示态】
                                                fallback=move || view! {
                                                    <input
                                                        type="checkbox"
                                                        class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
                                                        // 同样绑定响应式的 completed
                                                        prop:checked=completed
                                                        on:click=move |_| {
                                                            todos_local.update(|opt| {
                                                                if let Some(list) = opt {
                                                                    if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                                                                        t.completed = !t.completed;
                                                                    }
                                                                }
                                                            });
                                                            toggle.dispatch(id);
                                                        }
                                                    />
                                                    // 绑定响应式的 title
                                                    <span class="flex-1 min-w-0 text-base sm:text-lg wrap-break-word leading-relaxed todo-title">
                                                        {title}
                                                    </span>
                                                    <button
                                                        class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 active:scale-95 transition-all duration-200"
                                                        on:click=move |_| set_editing.set(Some(id))
                                                    >"✎"</button>
                                                    <button
                                                        class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-red-600 hover:bg-red-50 active:scale-95 transition-all duration-200"
                                                        on:click=move |_| {
                                                            todos_local.update(|opt| {
                                                                if let Some(list) = opt {
                                                                    list.retain(|t| t.id != id);
                                                                }
                                                            });
                                                            delete.dispatch(id);
                                                        }
                                                    >"✕"</button>
                                                }
                                            >
                                                // 【编辑态】
                                                <input
                                                    node_ref=edit_ref
                                                    type="text"
                                                    class="flex-1 min-w-0 px-4 py-3 text-base sm:text-lg bg-slate-50 border border-indigo-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/15 transition-all duration-200"
                                                    // 编辑框的初始值
                                                    prop:value=title
                                                />
                                                <button
                                                    class="px-4 py-3 text-sm font-semibold text-white bg-linear-to-r from-indigo-500 to-violet-500 rounded-lg shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
                                                    on:click=move |_| {
                                                        let value = edit_ref
                                                            .get()
                                                            .map(|el| el.value())
                                                            .unwrap_or_default();

                                                        todos_local.update(|opt| {
                                                            if let Some(list) = opt {
                                                                if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                                                                    t.title = value.clone();
                                                                }
                                                            }
                                                        });
                                                        set_editing.set(None);
                                                        update.dispatch((id, value));
                                                    }
                                                >"Save"</button>
                                                <button
                                                    class="px-3 py-2 text-sm font-semibold text-slate-600 bg-slate-100 border border-slate-200 rounded-lg hover:bg-slate-200 hover:-translate-y-0.5 active:translate-y-0 transition-all duration-200"
                                                    on:click=move |_| set_editing.set(None)
                                                >"Cancel"</button>
                                            </Show>
                                        </li>
                                    }
                                }
                            /> // <For> 结束
                        </ul>
                    </div>
                </Show>
            </Transition>
        </section>
    }
}
