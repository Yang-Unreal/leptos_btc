// ============================================================================
// main.rs —— 程序的入口文件
// ----------------------------------------------------------------------------
// 技术栈：Leptos（Rust 全栈 Web 框架）+ Axum（Web 服务器）+ SQLx（异步数据库）
//         + PostgreSQL。
//
// 【最重要的背景，先理解它，后面所有"为什么"才讲得通】
// 这一份源码会被编译成【两份完全不同的产物】：
//   1) 服务器端（SSR = Server-Side Rendering）：一个真正运行的原生后端程序，
//      负责连接数据库、渲染 HTML、响应请求。
//   2) 客户端（WASM = WebAssembly）：跑在浏览器里的前端代码，负责页面交互。
//
// 为什么要"同一份代码编两次"？这正是全栈框架的卖点：前后端共用同一套类型和函数
// （比如 todo.rs 里的 Todo 结构体、get_todos 等），避免前后端各写一遍、还容易对不上。
//
// 但问题来了：数据库连接、Axum、tokio 这些东西【根本无法在浏览器的 WASM 里运行】。
// 如果把它们编译进 WASM 版本，会直接编译失败。
// 解决办法就是"条件编译"：用 #[cfg(feature = "ssr")] 把"只属于服务器"的代码
// 圈起来，让它【只在编译服务器版本时】才存在。这就是本文件有两个 main 的根本原因。
// ============================================================================

// 【Rust 基础语法讲解：属性（Attribute）】
// 以 # 开头的特殊指令，告诉编译器如何处理下面的代码。
// #[cfg(feature = "ssr")] 是条件编译属性，意思是：
// "如果当前编译启用了 'ssr' 这个 feature，就保留下面的代码；否则当它不存在。"
// feature 是 Cargo.toml 里定义的编译开关，可以开启/关闭某些代码块。
//
// 【Rust 基础语法讲解：宏（Macro）】
// #[tokio::main] 是一个"函数宏/属性宏"，以 ! 结尾的调用或 #[...] 形式出现。
// 宏是在编译期展开的"代码生成器"。#[tokio::main] 会把你下面的 async fn main()
// 改写成真正可以运行的同步 main()，并在里面自动启动 tokio 异步运行时。
// 【为什么需要它】：Rust 标准的程序入口 main 不允许是 async 的，但我们的几乎所有 I/O
// （连数据库、监听网络）都是异步的、需要 .await。没有一个"运行时"来驱动这些异步任务，
// .await 就无从执行。这个宏就是帮我们把运行时搭好，让 main 内部可以写异步代码。
//
// 【Rust 基础语法讲解：异步函数（async fn）】
// async fn 返回一个 Future（未来值）而不是直接返回值。
// 【为什么用异步】：Web 服务器要同时处理成百上千个连接。异步模型下，一个任务在等待
// I/O（比如等数据库返回）时会主动让出线程，去处理别的请求，而不是干等着占着线程。
// 这样用很少的线程就能扛住大量并发，这是高性能网络服务的标准做法。
//
// 【Rust 基础语法讲解：.await 运算符】
// .await 只能在 async 块或 async 函数里使用。
// 它的作用是"暂停当前函数的执行，等这个 Future 完成后再继续"。
// 想象你在等外卖：你不用站在门口干等（阻塞线程），而是去做别的事（让出线程），
// 外卖到了再回来取（.await 返回）。这样一个人可以同时等好几份外卖。
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    // 从项目根目录的 .env 文件加载环境变量（本项目主要是 DATABASE_URL）。
    // 【为什么这么写 let _ =】：dotenv() 返回 Result，若 .env 文件不存在会返回 Err。
    // 但"文件不存在"在这里是完全正常的——生产环境（如 Dokploy/Docker）通常直接在系统层面
    // 注入环境变量，根本没有 .env 文件。所以我们用 `let _ =` 明确表示"我知道有返回值，
    // 但故意丢弃它，也不当错误处理"。如果不写 let _，编译器会因为"未使用的 Result"而警告。
    // 【Rust 基础语法讲解：Result 类型】
    // Result<T, E> 是 Rust 表示"可能失败的操作"的标准方式，它是一个枚举（enum）：
    //   Ok(T)   - 成功，包含一个 T 类型的值
    //   Err(E)  - 失败，包含一个 E 类型的错误
    // Rust 强制你处理错误，不能忽略。这里用 let _ = 表示"我知道它失败了也没关系，丢弃即可"。
    let _ = dotenvy::dotenv();

    // ---- 把服务器专属的类型/函数引入当前作用域 ----
    // 【为什么把 use 写在函数内部，而不是文件顶部】：这些库（axum、sqlx、tokio…）都只在
    // ssr 特性下才存在。如果写在文件顶部，编译 WASM 版本时这些 use 会指向不存在的东西而报错。
    // 把它们放进这个已经被 #[cfg(feature = "ssr")] 圈住的函数里，就天然只在服务器版本里生效，
    // 省去了给每一行 use 都单独加 #[cfg(...)] 的麻烦。
    // 【Rust 基础语法讲解：use 导入语句】
    // use 语句把其他模块/库里的类型、函数、trait 等引入当前作用域。
    // 这样不用每次都写全名（如 axum::Router），直接写 Router 即可。
    // 花括号可以一次导入多个项，如 use leptos_axum::{...};。
    use axum::body::Body; // HTTP 报文的"主体"类型
    use axum::extract::Request; // 代表一个进来的 HTTP 请求
    use axum::routing::post; // 声明"只处理 POST 方法"的路由
    use axum::Router; // 路由器：把"网址"映射到"处理逻辑"
    use leptos::logging::log; // 日志宏 log!
    use leptos::prelude::*; // Leptos 常用项的集中导入（provide_context、get_configuration 等）
                            // 【Rust 基础语法讲解：trait 导入与方法调用】
                            // LeptosRoutes 是一个 trait。Rust 里"扩展方法"只有在对应 trait 被引入作用域后才能调用。
                            // 下面的 .leptos_routes_with_context(...) 就来自这个 trait，不 use 它这行方法就编译不过。
    use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
    //   generate_route_list：从组件里自动收集页面路由
    //   handle_server_fns_with_context：处理服务器函数调用、并允许注入上下文
    use leptos_btc::app::*; // 引入本项目的 App 组件、shell 函数等
    use sqlx::postgres::PgPoolOptions; // 创建 PostgreSQL 连接池的配置构造器

    // 读取 Leptos 配置（站点地址、输出目录等，来自 Cargo.toml 的 [package.metadata.leptos]）。
    // 【Rust 基础语法讲解：unwrap() 方法】
    // unwrap() 是 Option 和 Result 类型的方法：
    //   - 如果是 Some(x) / Ok(x)，返回 x
    //   - 如果是 None / Err(e)，直接 panic（崩溃）
    // 只在"确定一定有值"的情况下使用，否则程序会崩溃。
    // 【为什么用 unwrap】：配置读不出来说明项目根本没法启动，属于"启动阶段的致命错误"。
    // 这种情况下直接 panic 让程序立刻崩溃退出，是合理的——继续运行也没有意义。
    // （对比：请求处理过程中的错误就不该 unwrap，而要优雅返回错误响应。）
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    // 取出服务器要监听的地址（IP:端口），例如 127.0.0.1:3000。
    let addr = leptos_options.site_addr;

    // --- 创建 Postgres 连接池，并做启动时的建表 ---
    // 读取数据库连接串。
    // 【Rust 基础语法讲解：expect() 方法】
    // expect(msg) 和 unwrap() 类似，都是失败时 panic，但 expect 会显示你提供的自定义错误信息。
    // 没有 DATABASE_URL 时，"DATABASE_URL must be set" 比 unwrap 默认的报错信息更能一眼看出问题。
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // 用"建造者(builder)模式"创建连接池。
    // 【为什么用"连接池"而不是单个连接】：建立一次数据库连接（TCP + 认证）开销很大。
    // 连接池会预先维护若干条连接并反复复用；同时因为服务器是并发处理请求的，多个请求
    // 需要多条连接同时工作，单条连接会成为瓶颈。连接池正好解决"复用"和"并发"两件事。
    // 【Rust 基础语法讲解：建造者模式（Builder Pattern）】
    // PgPoolOptions::new() 返回一个"建造者"对象，然后 .max_connections(5) 等链式调用
    // 配置参数，最后 .connect(...) 真正执行创建。这种模式把复杂对象的构建过程拆成多步，
    // 代码可读性好，而且可以灵活配置可选参数。
    let pool = PgPoolOptions::new()
        // 【Rust 基础语法讲解：方法链（Method Chaining）】
        // .max_connections(5) 修改建造者的配置并返回自身（Builder），
        // 这样可以接着调下一个方法。这是建造者模式的标准写法。
        .max_connections(5)
        // 连接数据库；传 &database_url 是"借用"字符串——connect 只需要读一下内容，
        // 不需要拿走它的所有权，用完 database_url 在后面也仍然可用（虽然这里没再用）。
        // 【Rust 基础语法讲解：借用（Borrowing）】
        // &database_url 创建一个"引用"，允许函数读取数据而不拿走所有权。
        // 类比：你把一本书借给朋友看（&），朋友看完还给你，书还是你的。
        // 如果直接传 database_url（没有 &），就是把书送给朋友（所有权转移），你自己就没有了。
        .connect(&database_url)
        // 【Rust 基础语法讲解：.await】
        // 建立连接是异步 I/O，等它完成。.await 只能在 async 函数或 async 块里使用。
        .await
        // 连不上数据库同样属于启动致命错误。
        .expect("could not connect to Postgres");

    // 启动时确保 todos 表存在。
    // 【为什么在代码里建表，而且用 IF NOT EXISTS】：这是一种简单的"自举(bootstrap)"做法——
    // 程序一跑起来就把需要的表准备好，省去手动初始化数据库的步骤。IF NOT EXISTS 保证
    // 第二次、第三次启动时不会因为"表已存在"而报错，也就是让这段操作可以安全地重复执行
    // （幂等）。注意：真实大型项目通常改用专门的"数据库迁移(migration)"工具来管理表结构演进，
    // 这里为了简单直接内联建表。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (
            id          SERIAL PRIMARY KEY,                       -- 自增主键，唯一标识每条待办
            title       TEXT NOT NULL,                            -- 标题，NOT NULL 表示必填
            completed   BOOLEAN NOT NULL DEFAULT FALSE,           -- 是否完成，默认未完成
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()        -- 创建时间，默认当前时刻
        )",
    )
    .execute(&pool) // 借用连接池来执行；从池里临时借一条连接跑这条 SQL
    .await
    .expect("could not create todos table");

    // 生成页面路由列表。
    // 【为什么需要它】：Leptos 允许在组件里（app.rs）用声明式的方式定义有哪些页面路径。
    // generate_route_list 会"预跑"一遍 App 组件、把这些路径收集出来，交给 Axum 注册，
    // 这样服务器才知道每个 URL 对应要渲染哪个页面。App 作为参数传入（注意没有括号，
    // 传的是这个组件函数本身，而不是调用它的结果）。
    let routes = generate_route_list(App);

    // 克隆一份连接池句柄，专门给下面的 /api 路由用。
    // 【为什么要 clone，以及 clone 到底复制了什么】：
    //   - Rust 的所有权规则：一个值默认只有一个所有者。我们既要在 /api 路由里用 pool，
    //     又要在后面的 SSR 路由里用 pool，一个 pool 满足不了"同时被多处拥有"。
    //   - 连接池的 clone 很廉价：它内部用引用计数(Arc)包着真正的连接池，clone 只是把计数 +1，
    //     得到的多个句柄都指向【同一个】底层池、共享那 5 条连接。所以这里不会真的复制出多套连接。
    //   - 因此"为每个需要它的地方各 clone 一份句柄"是 Rust 里共享这类资源的惯用手法。
    let api_pool = pool.clone();

    // ---- 组装整个应用的路由表 ----
    // 【Rust 基础语法讲解：方法链（Method Chaining）】
    // 注意看：Router::new() 返回一个 Router，然后 .route(...) 又返回 Router，
    // 再 .leptos_routes_with_context(...) 还返回 Router...
    // 这种"每个方法都返回自身，可以接着调下一个方法"的写法叫方法链。
    // 好处是代码从左到右读，像流水线一样清晰；坏处是如果中间某个环节出错，调试时
    // 需要 mentally 把链拆开来看。建造者模式（Builder Pattern）大量使用这种技巧。
    let app = Router::new()
        // 定义所有"服务器函数"的入口，统一挂在 /api/ 下面。
        // 【什么是服务器函数、为什么要这条路由】：todo.rs 里带 #[server] 的函数（get_todos、
        // add_todo 等）是"服务器函数"。它们在客户端被调用时，Leptos 其实会自动发一个 HTTP
        // 请求到服务器；服务器这一侧就靠这条路由来接收并真正执行数据库操作。
        // "/api/{tail..}" 里的 {tail..} 是"通配尾段"，能匹配 /api/ 之后的任意路径，
        // 从而用一条路由覆盖所有服务器函数，不必给每个函数单独写一条。
        .route(
            "/api/{tail..}",
            // post(...)：只接受 POST。【为什么是 POST】：这些调用会修改数据/带请求体，
            // 语义上属于 POST；Leptos 的服务器函数默认也走 POST。
            post(move |req: Request<Body>| {
                // 第 2 层：每次请求进来，从闭包自持的 api_pool 再 clone 一份给"这一次请求"。
                // 【为什么在这里还要再 clone 一次】：这个闭包会被调用很多次（每个请求一次），
                // 每次都要把一份 pool "交给"下面的 async 块并被 move 走。如果直接 move 外层的
                // api_pool，它只有一份、move 一次就没了，第二个请求就无池可用。所以每次调用时
                // 先 clone 出一份"本次请求专用"的句柄，既满足所有权，又因为 clone 廉价而无负担。
                let pool = api_pool.clone();
                // 第 3 层：async move { ... } 是这次请求真正要跑的异步任务。
                // 【为什么又是 move】：这个 async 块（future）会被交给运行时，可能在闭包返回之后
                // 才真正执行，所以它必须【拥有】自己用到的数据（这里是 pool），不能借用即将离开
                // 作用域的局部变量。move 把上面那份 pool 搬进 future。
                // 【Rust 基础语法讲解：async move 块】
                // async move { ... } 是一个异步块，返回一个 Future（未来值）。
                // move 关键字表示块内用到的外部变量会被移动进块内，块拥有这些数据。
                // 这个 Future 可能会在闭包返回后才执行，所以必须拥有自己的数据。
                async move {
                    // 真正处理服务器函数调用。
                    // 【第一个参数那个闭包在干嘛、为什么必须有】：move || provide_context(pool.clone())
                    //   这是关键一环。todo.rs 里每个服务器函数体内都会写
                    //   `let pool = expect_context::<sqlx::PgPool>();`——它是在"向上下文索要"连接池。
                    //   而"把连接池放进上下文"正是这里的 provide_context 干的事。二者一供一取，配成一对。
                    //   如果这里不 provide_context，服务器函数里的 expect_context 就会找不到池而 panic。
                    //   【为什么这里又 clone】：provide_context 需要拿走一份池的所有权放进上下文，
                    //   而这个提供者闭包本身也可能被多次调用，所以同样每次 clone 一份，理由同上。
                    handle_server_fns_with_context(move || provide_context(pool.clone()), req).await
                }
            }),
        )
        // 注册 Leptos 的页面路由（负责把页面在服务器端渲染成 HTML）。
        .leptos_routes_with_context(
            &leptos_options, // 【Rust 基础语法讲解：引用】借用配置即可，不拿走所有权
            routes,          // 前面收集到的页面路由
            {
                // 第三个参数：又一个"提供上下文"的闭包。
                // 【为什么 SSR 这边也要提供连接池】：首屏是在服务器端渲染的。app.rs 里的
                //   Resource::new(..., get_todos) 会在 SSR 期间就【在服务器上直接调用】get_todos
                //   去查数据库、把数据一起渲染进首屏 HTML（这样用户一打开页面就能看到列表，
                //   而不是先看到空白再等前端二次请求）。既然 SSR 期间也会执行服务器函数，
                //   它同样需要 expect_context 拿到连接池，所以这里必须同样 provide_context。
                //   —— 这也解释了为什么 /api 那条路由和这条 SSR 路由都要各自提供一次连接池：
                //      服务器函数有"通过 /api 被前端调用"和"在 SSR 期间被直接调用"两种触发路径，
                //      两条路径的上下文是分开的，都得喂到。
                let ssr_pool = pool.clone(); // 给 SSR 单独 clone 一份句柄
                                             // 【Rust 基础语法讲解：闭包作为返回值/最后表达式】
                                             // 这个 { ... } 块最后是一个 move || provide_context(ssr_pool.clone())。
                                             // Rust 的函数/闭包最后一行如果没有分号，就是"返回值"。这里返回一个闭包。
                move || provide_context(ssr_pool.clone())
            },
            {
                // 第四个参数：页面"外壳(shell)"生成器。
                // 【它是什么】：shell（见 app.rs）产出最外层的 <!DOCTYPE html><html>…</html> 骨架，
                //   里面通过 <HydrationScripts> 注入让页面"活起来"的脚本。SSR 每次渲染页面都要
                //   套上这层外壳，所以这里传的是"一个能生成外壳的闭包"，而不是外壳本身。
                // 【为什么先 clone 再 move】：leptos_options 后面还要用（with_state），
                //   若直接 move 进闭包，后面就没得用了。所以先 clone 一份专门交给这个闭包。
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        // 兜底处理器：以上路由都没匹配到时走这里。
        // 【为什么需要它】：浏览器除了要页面和 /api，还会来要静态资源（编译好的 JS/WASM/CSS、
        //   图片等）；另外用户访问不存在的地址时也要给个 404 页面。file_and_error_handler
        //   就负责"尝试返回对应静态文件，找不到就返回错误页面"。没有它，静态资源就发不出去，
        //   前端也就无法"注水(hydrate)"成可交互页面。
        .fallback(leptos_axum::file_and_error_handler(shell))
        // 把配置作为 Router 的"共享状态(state)"存进去。
        // 【为什么要这样】：上面几个 handler（尤其是 fallback）内部需要用到 leptos_options
        //   才能找到静态文件目录等信息。with_state 把它交给 Router 统一保管，各 handler
        //   在运行时都能取到。这里是把所有权交出去，所以放在最后——之后不再需要它了。
        .with_state(leptos_options);

    // ---- 启动服务器，开始对外提供服务 ----
    // 打印监听地址。{} 是占位符，会被 &addr 填入。
    // 【为什么 clippy 里常见传 &addr 而不是 addr】：打印只需读一下地址，用借用即可，
    //   不必把 addr 的所有权交给 log!，这样后面仍可继续使用 addr。
    // 【Rust 基础语法讲解：格式化字符串（Format String）】
    // log!("listening on http://{}", &addr); 中的 {} 是占位符，会被后面的 &addr 替换。
    // 这是 Rust 的格式化输出语法，类似 Python 的 f-string 或 C 的 printf，但更类型安全。
    // ! 表示这是一个宏（macro），不是普通函数。宏在编译期展开，可以接受可变数量的参数。
    log!("listening on http://{}", &addr);

    // 绑定 TCP 端口，得到监听器。
    // 【Rust 基础语法讲解：类型路径（Type Path）】
    // tokio::net::TcpListener::bind(&addr) 使用"类型路径"语法：
    //   模块::子模块::类型::方法(...)
    // 这里 TcpListener 是 tokio::net 模块里的类型，bind 是它的关联函数（不是方法）。
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // 正式开始服务。
    // 【into_make_service 为什么需要】：Axum 需要为"每一个新连接"都能生成一个服务实例来处理，
    //   into_make_service 把我们的 Router 转换成这种"工厂"形式，交给 axum::serve 循环使用。
    axum::serve(listener, app.into_make_service())
        // 【Rust 基础语法讲解：.await 与无限运行】
        // 为什么这行 .await 会"卡住"不返回：serve 是一个持续运行的循环，它会一直
        //   接收并处理请求，正常情况下永不结束。于是这里的 .await 会让 main 一直停在这里，
        //   程序也就作为一个长期运行的服务持续对外提供功能。
        .await
        .unwrap(); // 服务器发生不可恢复的致命错误时崩溃
}
