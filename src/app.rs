// ============================================================================
// app.rs —— 界面(UI) + 前端交互逻辑（Leptos 组件）
// ----------------------------------------------------------------------------
// 这个文件定义“页面长什么样”和“点了按钮之后怎么反应”。它同时用于两端：
//   - 服务器端：SSR 时被执行一遍，生成首屏 HTML 字符串。
//   - 浏览器端：hydrate 时再执行一遍，把交互逻辑接到 HTML 上。

use crate::todo::*; // 引入 Todo 结构体和 5 个服务器函数（get_todos/add_todo/...）
use leptos::html::Input; // 代表 <input> 这个 HTML 元素类型，配合 NodeRef 直接读取输入框
use leptos::prelude::*; // Leptos 绝大多数常用项（signal、view!、Action、Resource 等）
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title}; // 管理 <head> 里的元信息

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                // AutoReload：开发模式下代码改动后自动刷新浏览器（cargo leptos watch 用）。
                // options.clone()：这些组件都需要一份配置，clone 是因为要分别交给多个组件。
                <AutoReload options=options.clone() />
                // HydrationScripts：注入加载 WASM 和启动 hydrate() 的 <script>。
                // 【为什么至关重要】：没有它，浏览器就不会去下载/运行前端 WASM，页面永远是
                //   “死的”静态 HTML，点按钮没反应。它是把 lib.rs 的 hydrate() 接起来的关键。
                <HydrationScripts options/>
                // MetaTags：占位符，Title/Stylesheet 等组件设置的 <head> 内容最终注入这里。
                <MetaTags/>
            </head>
            <body>
                // 把根组件 App 放进 body。真正的界面从这里开始。
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
        <Title text="Leptos + Postgres CRUD"/>

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
            // 调用服务器函数新增（浏览器里 → 发 HTTP 请求）。? 表示失败就中断并返回错误。
            add_todo(title).await?;
            // 成功后把刷新触发器 +1 → Resource 重新拉取 → Effect 再把最新数据写回本地镜像。
            // 【为什么新增走“服务器为准”而不是乐观更新】：新增的记录 id、created_at 是数据库
            //   生成的，本地无法预知，所以新增后干脆重新拉一遍拿到权威数据最稳妥。
            set_refetch.update(|n| *n += 1);
            // 显式标注这个 async 块的返回类型为 Result<(), ServerFnError>。
            // 【Rust 基础语法讲解： turbofish 运算符在类型标注中的使用】
            // Ok::<(), ServerFnError>(()) 中的 ::<(), ServerFnError> 告诉编译器：
            //   - 这个 Ok 包装的类型是 ()（空元组，表示没有返回值）
            //   - Result 的错误类型是 ServerFnError
            // 【为什么要写这一句】：帮助编译器确定 ? 运算符要处理的错误类型。
            //   ? 运算符需要知道错误类型是什么才能正确传播。如果不标注，编译器可能无法推断。
            Ok::<(), ServerFnError>(())
        }
    });

    // 切换完成状态的 Action。
    let toggle = Action::new(move |id: &i32| {
        let id = *id; // i32 实现了 Copy，用 * 解引用直接复制出值
        async move {
            // 注意：本地的乐观更新已经在“点击处理函数”里先做了（见下方 checkbox 的 on:click）。
            // 这里只负责发请求；只有当请求【失败】时，才重新从服务器同步，以回滚错误的乐观改动。
            if toggle_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    // 删除的 Action，套路同上：乐观删除在点击时已做，失败才回滚。
    let delete = Action::new(move |id: &i32| {
        let id = *id;
        async move {
            if delete_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    // 记录“当前正在编辑哪一条”。None 表示没有任何条目处于编辑态。
    // 【为什么需要它】：每条待办可以切换到“编辑输入框”状态，用这个信号统一控制谁在编辑。
    // 【Rust 基础语法讲解：Option::<T>::None】
    // signal(Option::<i32>::None) 创建一个初值为 None 的信号，类型是 Option<i32>。
    // 当 Some(id) 时表示正在编辑 id 对应的待办。
    let (editing, set_editing) = signal(Option::<i32>::None);

    // 修改标题的 Action。参数是 (id, 新标题) 的元组。
    // 【Rust 基础语法讲解：元组（Tuple）】
    // 元组是把多个不同类型值组合在一起的方式，写法是 (T1, T2, T3)。
    // 这里 Action 的参数是 &(i32, String)，即"一个包含 i32 和 String 的元组的引用"。
    let update = Action::new(move |args: &(i32, String)| {
        let (id, title) = args; // 解构元组
        let id = *id; // 复制 id（i32 是 Copy 类型）
        let title = title.clone(); // 复制标题（String 需要 clone 才能复制）
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
            <h2 class="text-lg sm:text-xl md:text-2xl font-bold tracking-tight text-slate-800 text-center mb-4 sm:mb-6">"Todos (Postgres + SQLx)"</h2>

            // 新增表单。
            <form
                class="flex flex-col sm:flex-row gap-4 mb-8 sm:mb-10"
                // on:submit=... 绑定表单提交事件。ev 是事件对象。
                on:submit=move |ev| {
                    // 阻止浏览器默认的“提交表单会刷新页面”行为——我们要用 JS/WASM 处理。
                    ev.prevent_default();
                    // 读取输入框当前值：title_ref.get() 拿到 <input> 元素（Option），
                    // .map(|el| el.value()) 取它的文字，取不到就用空字符串兜底。
                    let value = title_ref.get().map(|el| el.value()).unwrap_or_default();
                    // 非空才提交（trim 去空白后判断）。
                    if !value.trim().is_empty() {
                        add.dispatch(value); // 触发上面的 add Action → 调 add_todo
                        // 提交后清空输入框，方便继续输入下一条。
                        if let Some(input) = title_ref.get() {
                            input.set_value("");
                        }
                    }
                }
            >
                // node_ref=title_ref：把这个 <input> 和上面的 NodeRef 关联起来。
                <input
                    node_ref=title_ref
                    type="text"
                    placeholder="What needs to be done?"
                    class="flex-1 min-w-0 px-5 py-4 text-base sm:text-lg bg-slate-50 border border-slate-200 rounded-xl sm:rounded-2xl focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all duration-200 placeholder:text-slate-400"
                />
                <button
                    type="submit"
                    class="px-8 py-4 text-base sm:text-lg font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-500 rounded-xl sm:rounded-2xl shadow-md hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
                >"Add"</button>
            </form>

            // Transition：在异步数据加载时显示 fallback，加载完成后显示真实内容；
            // 且在“重新加载”时不会闪回 fallback（比 Suspense 更平滑）。
            // 【为什么用它】：Resource 的数据是异步来的，首次加载时用它显示“Loading…”，
            //   体验更好。
            <Transition fallback=move || view! { <p>"Loading todos…"</p> }>
                // 这个 move || 闭包是响应式的：它读取的信号一变，这块 UI 就自动重渲染。
                // 【Rust 基础语法讲解：match 表达式与模式匹配】
                // match (todos.get(), todos_local.get()) 同时看两个来源：
                //   todos（服务器 Resource）和 todos_local（本地镜像）。
                // match 会逐一检查每个分支，找到第一个匹配的并执行。
                // Rust 的 match 是“穷尽的（exhaustive）”：必须覆盖所有可能的情况，否则编译不过。
                {move || match (todos.get(), todos_local.get()) {
                    // 情况一：本地镜像有数据（也覆盖了“乐观修改之后”的状态）→ 用它渲染列表。
                    // (_, Some(list)) 里第一个 _ 表示“不关心 Resource 当前是什么状态”。
                    // 【Rust 基础语法讲解：通配符 _】
                    // _ 是模式匹配中的通配符，表示"匹配任意值但不关心具体是什么"。
                    // 这里我们只关心本地镜像是否有数据，不关心服务器 Resource 的状态。
                    (_, Some(list)) => view! {
                        <div>
                             <ul class="list-none m-0 p-0 flex flex-col gap-3 sm:gap-4">
                                // 把 Vec<Todo> 转成一串 <li>。
                                // 【Rust 基础语法讲解：迭代器（Iterator）】
                                // list.into_iter() 把 Vec 变成迭代器，可以逐个消费其中的元素。
                                // into_iter() 会"消耗"list（所有权转移），每个元素取出后原 Vec 不再可用。
                                // 对比 iter()（借用，不消耗）和 iter_mut()（可变借用，可修改元素）。
                                {list
                                    .into_iter()
                                    .map(|todo| {
                                        // 为每一条待办生成一个 <li>。
                                        let id = todo.id;
                                        // 每条编辑输入框各自的 NodeRef。
                                        let edit_ref = NodeRef::<Input>::new();
                                        // 当前这条是否处于编辑态。
                                        let is_editing = editing.get() == Some(id);
                                        view! {
                                            // class:completed=... 是条件类名：completed 为真时
                                            // 给 <li> 加上 completed 样式类（通常用于加删除线）。
                                             <li class:completed=todo.completed class="flex items-center gap-3 sm:gap-4 p-4 sm:p-5 border border-slate-200 rounded-xl bg-white transition-all duration-200 hover:border-slate-300 hover:bg-slate-50 hover:shadow-sm active:scale-[0.998] active:bg-slate-100">
                                                // 根据是否在编辑，显示两套不同的界面。
                                                // 【Rust 基础语法讲解：if 表达式】
                                                // Rust 里 if 是表达式，不是语句。它会返回一个值。
                                                // 这里 if is_editing { ... } else { ... } 整体返回一个 view。
                                                // 两个分支必须返回相同类型（都用 .into_any() 擦除成 AnyView）。
                                                {if is_editing {
                                                    // —— 编辑态：文本框 + 保存 + 取消 ——
                                                    view! {
                                                         <input
                                                             node_ref=edit_ref
                                                             type="text"
                                                              class="flex-1 min-w-0 px-4 py-3 text-base sm:text-lg bg-slate-50 border border-indigo-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500/15 transition-all duration-200"
                                                             // prop:value 设置输入框的初始内容为当前标题。
                                                             prop:value=todo.title
                                                         />
                                                         <button
                                                              class="px-4 py-3 text-sm font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-500 rounded-lg shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:opacity-90 transition-all duration-200"
                                                             on:click=move |_| {
                                                                 // 读取编辑框里的新文字。
                                                                 let value = edit_ref
                                                                     .get()
                                                                     .map(|el| el.value())
                                                                     .unwrap_or_default();
                                                                 // 乐观更新：立刻改本地镜像里对应那条的标题。
                                                                 // 【Rust 基础语法讲解：闭包参数 |_|】
                                                                 // |_| 表示"忽略这个参数"。on:click 会传入事件对象，
                                                                 // 但我们不需要它，所以用 _ 表示忽略。
                                                                 todos_local.update(|opt| {
                                                                     if let Some(list) = opt {
                                                                         // iter_mut 拿到可修改的引用，
                                                                         // find 找到 id 匹配的那条。
                                                                         // 【Rust 基础语法讲解：方法链】
                                                                         // list.iter_mut() 返回可变迭代器
                                                                         //   .find(|t| t.id == id) 查找满足条件的元素
                                                                         //   整个链式调用返回 Option<&mut Todo>
                                                                         if let Some(t) = list
                                                                             .iter_mut()
                                                                             .find(|t| t.id == id)
                                                                         {
                                                                             t.title = value.clone();
                                                                         }
                                                                     }
                                                                 });
                                                                 // 退出编辑态。
                                                                 set_editing.set(None);
                                                                 // 再把改动发给服务器（失败会在 update Action 里回滚)。
                                                                 update.dispatch((id, value));
                                                             }
                                                         >"Save"</button>
                                                         <button
                                                             class="px-3 py-2 text-sm font-semibold text-slate-600 bg-slate-100 border border-slate-200 rounded-lg hover:bg-slate-200 hover:-translate-y-0.5 active:translate-y-0 transition-all duration-200"
                                                             // 取消：仅退出编辑态，不改数据。
                                                             on:click=move |_| set_editing.set(None)
                                                        >"Cancel"</button>
                                                    }.into_any()
                                                    // 【为什么末尾要 .into_any()】：if 的两个分支返回的
                                                    //   view 具体类型不同，Rust 要求 if/else 两支类型一致。
                                                    //   into_any() 把它们“擦除”成同一个统一类型(AnyView)，
                                                    //   这样两支才能匹配、通过编译。
                                                    // 【Rust 基础语法讲解：类型擦除】
                                                    // Rust 是静态类型语言，通常要求编译期知道确切类型。
                                                    // 当我们需要"不同类型但行为相同"时，可以用 trait object
                                                    // （如 AnyView）进行"类型擦除"，让编译器把它们当作同一类型。
                                                } else {
                                                    // —— 展示态：勾选框 + 标题 + 编辑 + 删除 ——
                                                    view! {
                                                        <input
                                                            type="checkbox"
                                                            class="w-4 h-4 sm:w-5 sm:h-5 accent-indigo-500 cursor-pointer flex-none transition-transform duration-150 hover:scale-110"
                                                            // prop:checked 反映当前完成状态。
                                                            prop:checked=todo.completed
                                                            on:click=move |_| {
                                                                // 乐观更新：立刻在本地把 completed 取反。
                                                                todos_local.update(|opt| {
                                                                    if let Some(list) = opt {
                                                                        if let Some(t) = list
                                                                            .iter_mut()
                                                                            .find(|t| t.id == id)
                                                                        {
                                                                            t.completed = !t.completed;
                                                                        }
                                                                    }
                                                                });
                                                                // 再通知服务器（失败才回滚)。
                                                                toggle.dispatch(id);
                                                            }
                                                        />
                                                         <span class="flex-1 min-w-0 text-base sm:text-lg break-words leading-relaxed todo-title">{todo.title}</span>
                                                        <button
                                                            class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 active:scale-95 transition-all duration-200"
                                                            // 进入编辑态：记录正在编辑的是这条 id。
                                                            on:click=move |_| set_editing.set(Some(id))
                                                        >"✎"</button>
                                                        <button
                                                            class="w-8 h-8 sm:w-9 sm:h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-red-600 hover:bg-red-50 active:scale-95 transition-all duration-200"
                                                            on:click=move |_| {
                                                                // 乐观删除：立刻从本地镜像移除这条。
                                                                // retain 保留"不等于 id"的所有元素。
                                                                // 【Rust 基础语法讲解：闭包在迭代器中的使用】
                                                                // list.retain(|t| t.id != id) 遍历列表，
                                                                // 闭包返回 true 保留元素，false 移除。
                                                                // 这里保留 id 不等于当前 id 的元素，实现删除。
                                                                todos_local.update(|opt| {
                                                                    if let Some(list) = opt {
                                                                        list.retain(|t| t.id != id);
                                                                    }
                                                                });
                                                                // 再通知服务器删除（失败才回滚）。
                                                                delete.dispatch(id);
                                                            }
                                                        >"✕"</button>
                                                    }.into_any()
                                                }}
                                            </li>
                                        }
                                    })
                                    .collect_view()} // 把一串 view 收集成可渲染的列表
                            </ul>
                        </div>
                    }
                    .into_view()
                    .into_any(),
                    // 情况二：服务器返回错误，且本地没有任何缓存 → 显示错误信息。
                    // (Some(Err(e)), None)：Resource 已返回但是 Err，且本地镜像还是 None。
                    (Some(Err(e)), None) => {
                        view! { <div class="text-center text-sm text-red-500 my-4 p-3 bg-red-50 rounded-xl border border-red-200/60">{format!("Error: {}", e)}</div> }
                            .into_view()
                            .into_any()
                    }
                    // 情况三：其它（仍在加载 / 正在协调）→ 显示 Loading。
                    // `_` 是“兜底分支”，匹配前面没覆盖到的所有情况。match 必须覆盖所有可能。
                    _ => view! { <div class="text-center text-slate-400 text-sm my-4 leading-relaxed">"Loading…"</div> }.into_view().into_any(),
                }}
            </Transition>
        </section>
    }
}
