// ============================================================================
// app.rs —— 界面(UI) + 前端交互逻辑（Leptos 组件）
// ----------------------------------------------------------------------------
// 这个文件定义“页面长什么样”和“点了按钮之后怎么反应”。它同时用于两端：
//   - 服务器端：SSR 时被执行一遍，生成首屏 HTML 字符串。
//   - 浏览器端：hydrate 时再执行一遍，把交互逻辑接到 HTML 上。
//
// 需要先建立的两个 Leptos 核心概念：
//   1) 组件(component)：用 #[component] 标注的函数，返回 `impl IntoView`（可渲染的视图）。
//      在 view! 里用 <大写名字/> 的形式像 HTML 标签一样使用它。
//   2) 响应式(reactive)：signal（信号）是“会变化的值”。当信号变化时，用到它的那部分
//      UI 会自动重新渲染。这就是 Leptos 不用手动操作 DOM 的原因——你改数据，界面自动跟着变。
// ============================================================================

// 【Rust 基础语法讲解：use 声明】
// use 语句把其他模块/库里的类型、函数、trait 等引入当前作用域，这样不用每次都写全名。
// 例如 leptos::html::Input 引入了 Input 类型，后面直接写 Input 即可。
use leptos::html::Input; // 代表 <input> 这个 HTML 元素类型，配合 NodeRef 直接读取输入框
use leptos::prelude::*;   // Leptos 绝大多数常用项（signal、view!、Action、Resource 等）
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title}; // 管理 <head> 里的元信息
use crate::todo::*;       // 引入 Todo 结构体和 5 个服务器函数（get_todos/add_todo/...）

// shell：整个页面的最外层 HTML 骨架。它不是 #[component]，是个普通函数，
// 在 main.rs 里被当作“页面外壳生成器”传给 leptos_routes_with_context。
// 【为什么需要一个单独的 shell】：SSR 需要一个完整的 <html>…</html> 文档，而不仅是 body 里的
//   内容。shell 负责 <head>（字符集、视口、注水脚本、meta）和把 <App/> 放进 <body>。
// 【Rust 基础语法讲解：pub fn + impl IntoView】
// pub fn 表示这是一个公共函数，可以被其他模块调用。
// -> impl IntoView 是“返回类型 impl Trait”语法：表示函数返回某个实现了 IntoView trait 的类型，
//   但具体类型对调用者隐藏。这让函数可以返回复杂的视图类型而无需写出完整类型名。
pub fn shell(options: LeptosOptions) -> impl IntoView {
    // view! { ... } 是 Leptos 的宏，让你在 Rust 里写类似 HTML 的模板。
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

// 【Rust 基础语法讲解：属性宏（Attribute Macro）】
// #[component] 是一个属性宏，它会自动把下面的函数转换成 Leptos 组件。
// 它主要做了两件事：
//   1. 为函数生成一个结构体，把参数变成结构体字段
//   2. 实现 IntoView trait，让这个函数可以在 view! 里像 HTML 标签一样使用
// 【Rust 基础语法讲解：为什么函数名用大写？】
// 在 view! 宏里，Rust/Leptos 通过首字母大小写区分 HTML 原生标签和自定义组件：
//   - <div>、<input> 小写 → HTML 原生标签
//   - <App/>、<Todos/> 大写 → 自定义组件
#[component]
pub fn App() -> impl IntoView {
    // 提供元信息上下文，让下面的 <Title>/<Stylesheet> 能正常工作（把内容收集到 <head>）。
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

    // 【Rust 基础语法讲解：let 语句与模式绑定】
    // let (refetch, set_refetch) = signal(0); 这行做了两件事：
    //   1. signal(0) 调用函数，返回一个元组 (读取端, 写入端)
    //   2. let (a, b) = ... 是模式匹配解构，把元组的两部分分别绑定到两个变量
    //
    // signal(0) 创建一个初值为 0 的信号，返回 (读取端 refetch, 写入端 set_refetch)。
    // 【这个信号是干嘛的、为什么需要它】：它是一个“刷新触发器”。每次我们想强制重新从
    //   服务器拉取最新列表时，就把它 +1。因为下面的 Resource 依赖它，值一变 Resource 就重跑。
    let (refetch, set_refetch) = signal(0);
    // Resource：把“异步数据加载”接入响应式系统。
    // 【两个参数的含义】：
    //   第一个闭包 move || refetch.get() 是“依赖来源”：读取了 refetch，于是 refetch 一变，
    //     Resource 就会重新执行第二个闭包。
    //   第二个闭包 |_| get_todos() 是“怎么加载数据”：调用服务器函数拉取待办列表。
    // 【为什么用 Resource 而不是直接调 get_todos】：Resource 会自动处理“加载中/成功/失败”
    //   三种状态，并且在 SSR 期间就把数据取好、随首屏一起发给浏览器（首屏直出数据，见前面讨论）。
    let todos = Resource::new(move || refetch.get(), |_| get_todos());

    // 本地镜像：把列表在客户端另存一份，用于“乐观更新(optimistic update)”。
    // RwSignal 是“可读可写合一”的信号。类型是 Option<Vec<Todo>>：None 表示“还没有数据”。
    // None::<Vec<Todo>> 里的 ::<Vec<Todo>> 是“显式指定泛型类型”，帮编译器确定这个 None 的类型。
    //   【Rust 基础语法讲解： turbofish 运算符 ::<T>】
    //   有时候编译器无法推断出泛型类型的具体值，就需要我们用 ::<T> 显式告诉编译器。
    //   这里 None 可以是任何类型的空值，加上 ::<Vec<Todo>> 就指定了"这是 Vec<Todo> 类型的 None"。
    //   【Rust 基础语法讲解：Option<T> 枚举】
    //   Option 是 Rust 标准库里的枚举，只有两个值：
    //     Some(T) - 有值，里面装着 T
    //     None    - 没有值（空）
    //   它替代了其他语言里的 null/nil，强迫你显式处理"没有值"的情况，避免空指针异常。
    //   例如：Option<Vec<Todo>> 表示"可能有一列 Vec<Todo>，也可能没有"。
    //
    // 【为什么要维护一份本地镜像】：如果每次点击都等服务器往返再刷新，UI 会有明显延迟。
    //   乐观更新的思路是：点击后【立刻】改本地镜像让界面秒变，同时后台悄悄发请求给服务器；
    //   只有当请求失败时才回滚（重新从服务器取）。这让应用手感很“跟手”。
    let todos_local = RwSignal::new(None::<Vec<Todo>>);
    // Effect：一个“副作用”，当它内部读取的信号变化时自动重新运行。
    // 【这个 Effect 的作用】：当 Resource（todos）成功拿到服务器数据时，把它同步进本地镜像。
    //   于是“服务器是数据源头(source of truth)”，而本地镜像是可被乐观修改的工作副本。
    Effect::new(move |_| {
        // todos.get() 返回 Option<Result<Vec<Todo>, _>>。
        // 【Rust 基础语法讲解：if let 模式匹配】
        // if let Some(Ok(list)) = todos.get() 这行做了三层模式匹配：
        //   1. todos.get() 返回 Option<...>
        //   2. 用 Some(...) 匹配，提取里面的值（如果是 None 就跳过整个 if）
        //   3. 用 Ok(list) 匹配，提取成功结果（如果是 Err 就跳过）
        // 这等价于：
        //   match todos.get() {
        //       Some(Ok(list)) => { ... }
        //       _ => {}  // None 或 Err 都不处理
        //   }
        // if let 是 match 的简写，只关心"成功"的情况，其他情况忽略。
        if let Some(Ok(list)) = todos.get() {
            todos_local.set(Some(list)); // 用服务器数据覆盖本地镜像
        }
    });

    // NodeRef：一个指向具体 DOM 元素的“引用”。这里指向新增待办的输入框。
    // 【为什么需要它】：提交表单时我们要读取输入框当前的文字、并在成功后清空它，
    //   NodeRef 让我们能直接拿到那个 <input> 元素来做这些操作。
    // 【Rust 基础语法讲解：泛型参数 <Input>】
    // NodeRef::<Input>::new() 中的 <Input> 是告诉 NodeRef 这个引用指向 <input> 元素。
    // 不同 HTML 元素有不同的类型（Input、Div、Button 等），这样可以在编译期保证类型安全。
    let title_ref = NodeRef::<Input>::new();

    // Action：封装“一次异步操作”（通常是一次数据修改）。dispatch 时执行，内部可 .await。
    // 【为什么用 Action 而不是自己 spawn 异步】：Action 帮你管理“进行中/完成/出错”状态，
    //   与响应式系统集成得很好，是 Leptos 里触发“写操作(增删改)”的推荐方式。
    // 【Rust 基础语法讲解：闭包作为参数】
    // Action::new(闭包) - 闭包的签名描述了 Action 的参数类型。
    // move |title: &String| 意思是：接收一个对 String 的引用，move 表示捕获外部变量时转移所有权。
    let add = Action::new(move |title: &String| {
        // 闭包参数是 &String（借用），先 clone 一份拿到所有权，好移进下面的 async 块。
        // 【Rust 基础语法讲解：clone() 方法】
        // clone() 创建值的深拷贝。这里从 &String（借用）复制成 String（拥有所有权）。
        // 为什么需要：async move 块需要拥有它的数据，不能只是借用。
        let title = title.clone();
        // 【Rust 基础语法讲解：async move 块】
        // async move { ... } 是一个异步块，返回一个 Future。
        // move 关键字表示块内用到的外部变量（这里是 title）会被移动进块内，块拥有这些数据。
        // 这个 Future 可能会在闭包返回后才执行，所以必须拥有自己的数据。
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
        // 【Rust 基础语法讲解：Copy 类型与解引用】
        // let id = *id; 这里做了两件事：
        //   1. *id 解引用：从 &i32（i32 的引用）得到 i32（值本身）
        //   2. let id = 把值绑定到新变量 id
        // 为什么可以这样：i32 是 Copy 类型（简单整型都是），复制成本极低。
        // Copy 类型在赋值/传参时会自动复制，不需要转移所有权。
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
        // 【Rust 基础语法讲解：解构赋值（Destructuring）】
        // let (id, title) = args; 从元组中提取值：
        //   - id 得到 i32（因为 i32 是 Copy，这里复制了值）
        //   - title 得到 String（因为 String 不是 Copy，这里转移了所有权）
        let (id, title) = args;      // 解构元组
        let id = *id;                // 复制 id（i32 是 Copy 类型）
        let title = title.clone();   // 复制标题（String 需要 clone 才能复制）
        async move {
            if update_todo(id, title).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    // ---------- 视图渲染区 ----------
    view! {
        <section class="todos">
            <h2>"Todos (Postgres + SQLx)"</h2>

            // 新增表单。
            <form
                class="todo-form"
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
                <input node_ref=title_ref type="text" placeholder="What needs to be done?"/>
                <button type="submit">"Add"</button>
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
                            <ul class="todo-list">
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
                                            <li class:completed=todo.completed>
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
                                                            class="edit-input"
                                                            // prop:value 设置输入框的初始内容为当前标题。
                                                            prop:value=todo.title
                                                        />
                                                        <button
                                                            class="save"
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
                                                                // 再把改动发给服务器（失败会在 update Action 里回滚）。
                                                                update.dispatch((id, value));
                                                            }
                                                        >"Save"</button>
                                                        <button
                                                            class="cancel"
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
                                                                // 再通知服务器（失败才回滚）。
                                                                toggle.dispatch(id);
                                                            }
                                                        />
                                                        <span class="title">{todo.title}</span>
                                                        <button
                                                            class="edit"
                                                            // 进入编辑态：记录正在编辑的是这条 id。
                                                            on:click=move |_| set_editing.set(Some(id))
                                                        >"✎"</button>
                                                        <button
                                                            class="delete"
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
                        view! { <div class="error">{format!("Error: {}", e)}</div> }
                            .into_view()
                            .into_any()
                    }
                    // 情况三：其它（仍在加载 / 正在协调）→ 显示 Loading。
                    // `_` 是“兜底分支”，匹配前面没覆盖到的所有情况。match 必须覆盖所有可能。
                    _ => view! { <div>"Loading…"</div> }.into_view().into_any(),
                }}
            </Transition>
        </section>
    }
}
