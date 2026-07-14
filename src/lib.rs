// ============================================================================
// lib.rs —— 库(crate)的根文件，也是“客户端(WASM)的真正入口”
// ----------------------------------------------------------------------------
// 回顾 main.rs 里讲过的：本项目同一份代码会被编译成两份产物。
//   - 服务器端：入口是 main.rs 里的 async main（rlib 被服务器 bin 链接）。
//   - 浏览器端(WASM)：入口不是 main，而是【这个文件里的 hydrate() 函数】（cdylib）。
//
// 另外，lib.rs 是整个 crate 的“模块树根”：它用 `pub mod` 声明有哪些子模块，
// 别的文件（包括 main.rs 通过 `leptos_btc::app::*`）才能访问到这些模块里的内容。
// ============================================================================

// 声明并公开(pub) app 模块（对应 src/app.rs）。
// 【为什么必须写这一行】：Rust 不会自动把 src/ 下的文件当成模块，必须显式用 mod 声明。
// 写成 `pub mod` 而不是 `mod`，是因为 main.rs 要用 `use leptos_btc::app::*;` 从 crate
// 外部访问它——不是 pub 的话外部就看不见。
pub mod app;
// 同理声明并公开 todo 模块（对应 src/todo.rs），里面是数据结构 Todo 和 5 个服务器函数。
pub mod todo;

// 下面这个函数只在编译 WASM（hydrate 特性）时才存在。
// 【为什么用 #[cfg(feature = "hydrate")]】：hydrate() 依赖 wasm_bindgen、
// console_error_panic_hook 这些只在浏览器端才有意义的东西；服务器端(ssr)编译时
// 不需要它，用 cfg 把它排除掉，避免多余依赖和编译问题。
#[cfg(feature = "hydrate")]
// #[wasm_bindgen] 属性：把这个 Rust 函数“导出”给 JavaScript 调用。
// 【为什么需要它】：浏览器不会自己去调 Rust 函数。cargo-leptos 生成的 JS 胶水代码
// 会在页面加载后调用这个被导出的 hydrate()，从而启动前端。它就是 WASM 侧的启动开关。
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // 把 app 模块内容引入作用域，好直接写 App（而不是 crate::app::App）。
    use crate::app::*;
    // 安装一个 panic 钩子：当 Rust 代码在浏览器里 panic 时，把详细错误打印到
    // 浏览器控制台(console)。
    // 【为什么要它】：WASM 默认的 panic 信息很不友好（只会看到一句模糊的 “unreachable”），
    // 装上这个钩子后调试时能看到真正的 Rust 报错，极大方便排错。set_once 保证只装一次。
    console_error_panic_hook::set_once();
    // 关键的“注水(hydrate)”动作。
    // 【它做什么/为什么叫注水】：服务器端已经把 App 渲染成了静态 HTML 发给浏览器，
    // 用户能立刻看到内容，但此时页面还是“死的”（按钮点了没反应）。hydrate_body(App)
    // 会在浏览器里【再运行一遍同样的 App】，把事件监听、信号(signal)等交互逻辑“接”到
    // 已存在的 HTML 元素上，让页面“活”过来变得可交互。因为前后端是同一个 App，
    // 两边渲染结果一致，所以能精准地对接上——这正是全栈同构(isomorphic)的意义。
    leptos::mount::hydrate_body(App);
}
