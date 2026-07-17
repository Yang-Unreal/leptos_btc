## 📑 目录

### 🦀 第一部分：宏（Macros）

- [第一章：解剖声明宏的核心语法糖 `$( $x:expr ),*`](#🌌-第一章解剖声明宏的核心语法糖-xexpr-)
  - [1. 语法深度拆解](#1-语法深度拆解)
  - [2. 展开体（RHS）的"成对法则"](#2-展开体rhs的成对法则)
- [第二章：声明宏（Declarative Macros）实战](#🏗️-第二章声明宏declarative-macros实战)
  - [完整实现代码](#完整实现代码)
- [第三章：过程宏（Procedural Macros）硬核筑基](#🔮-第三章过程宏procedural-macros硬核筑基)
  - [1. 建立"双项目"工作区（Workspace）环境配置](#1-建立双项目工作区workspace环境配置)
  - [2. 手写三大过程宏实现](#2-手写三大过程宏实现)
  - [3. 揭秘底层核心：为什么变量前要加 `#` 符号？](#3-揭秘底层核心为什么变量前要加-符号)
- [第四章：端到端整合测试](#🧪-第四章端到端整合测试)
- [第五章：宏高手的调试神兵与避坑准则](#🛠️-第五章宏高手的调试神兵与避坑准则)
  - [1. 宏调试终极武器：`cargo-expand`](#1-宏调试终极武器cargo-expand)
  - [2. 宏的终极使用哲学](#2-宏的终极使用哲学)
- [第六章：声明宏 vs 类函数宏——核心差异全景](#⚔️-第六章声明宏-vs-类函数宏核心差异全景)
  - [📊 声明宏 vs 类函数宏 核心对比](#📊-声明宏-vs-类函数宏-核心对比)
  - [🔍 核心差异深度拆解](#🔍-核心差异深度拆解)

### 🧬 第二部分：Trait（特质）

- [第七章：Rust 核心语法——Trait（特质）全景](#🧬-第七章rust-核心语法trait特质全景)
  - [1. Trait 是什么？为什么需要它？](#1-trait-是什么为什么需要它)
  - [2. 实战：自己写一个 Trait](#2-实战自己写一个-trait)
  - [3. 回到 Leptos](#3-回到-leptos)

### 🌐 第三部分：Leptos 框架

- [第八章：Leptos 组件与 `#[component]` 宏](#🌐-第八章leptos-组件与-component-宏)
  - [1. `#[component]` 宏是什么？](#1-component-宏是什么)
  - [2. 定义与使用组件](#2-定义与使用组件)
  - [3. 组件的运行机制](#3-组件的运行机制)
  - [4. 组件命名规则](#4-组件命名规则)
  - [5. 子组件（Children）](#5-子组件children)
  - [6. 自定义属性（Props）](#6-自定义属性props)

---

# 🌌 第一章：解剖声明宏的核心语法糖 `$( $x:expr ),*`

这个看似神秘的表达式，本质上是宏系统里的“正则表达式”。它的使命是：**匹配一串用逗号分隔、数量任意的 Rust 表达式。**

```text
  $(   $x : expr   )   ,   *
  ──   ─────────   ─   ─   ─
  ①        ②       ①   ③   ④

```

### 1. 语法深度拆解

* **① 外围双筒镜 `$( ... )` —— 捕获组（Capture Group）**
相当于正则表达式中的圆括号。它告诉编译器：“**括号内部定义的匹配模式，是一个需要被整体循环匹配的单元。**”
* **② 核心捕获器 `$x:expr` —— 匹配碎片（Fragment Specifier）**
在单次循环中，它负责抓住一个合法的 Rust **表达式（Expression）**，并将其绑定到临时变量 `$x` 上。
* **③ 粘合剂 `,` —— 分隔符（Separator）**
规定了多个重复元素之间，**必须用什么符号隔开**。你可以换成 `;` 或者 `+`，甚至不写（直接靠空格换行分隔）。
* **④ 循环控制符 `*` —— 重复次数（Repetition Operator）**
规定该模式可以出现多少次：
* `*`：匹配 **0 次或多次**（最常用，无参数传入时也能成功匹配）。
* `+`：匹配 **1 次或多次**（至少要传一个，否则编译报错）。
* `?`：匹配 **0 次或 1 次**（用于处理可选参数）。



### 2. 展开体（RHS）的“成对法则”

在左侧匹配（LHS）中使用了 `$( ... )*` 捕获的数据，在右侧的代码替换体（RHS）中，就**必须**以同样的 `$( ... )*` 结构来展开它。

```rust
macro_rules! my_print {
    ( $( $x:expr ),* ) => {
        {
            $(
                println!("值是: {}", $x);
            )* // 这里的 * 代表：针对捕获到的每一个元素，把上面的代码复制一份展开
        }
    };
}

```

当调用 `my_print!(1, "hello", true);` 时，编译器会在编译期将其**无缝展开**为：

```rust
{
    println!("值是: {}", 1);
    println!("值是: {}", "hello");
    println!("值是: {}", true);
}

```

---

# 🏗️ 第二章：声明宏（Declarative Macros）实战

声明宏通过 `macro_rules!` 定义，核心思想是“像 `match` 一样匹配代码，然后查找替换”。它可以让我们打破函数的参数限制，轻松消灭模板代码。

我们来动手实现一个像 Python/JS 一样清爽初始化键值对的 `hashmap!` 宏：

1. **设计目标语法:** 第 1 步.
我们希望消灭繁琐的 `insert` 语句，直接用键值映射的语法创建 HashMap：

```rust
let scores = hashmap!{
    "Alice" => 100,
    "Bob" => 95
};

```


2. **构建匹配模式:** 第 2 步.
我们需要匹配任意对 `键 => 值` 的组合。
利用我们刚学到的捕获语法，将键指定为 `$key:expr`，将值指定为 `$val:expr`，中间夹着不可变的分隔符 `=>`。


3. **编写替换代码:** 第 3 步.
在右侧代码块中，先创建空的 `HashMap`，接着通过循环块将所有捕获到的对塞入 Map，最后隐式返回这个实例。


### 完整实现代码

```rust
use std::collections::HashMap;

#[macro_export] // 允许该宏被其他模块导入使用
macro_rules! hashmap {
    // 匹配模式：大括号包裹，匹配 0 个或多个 "键 => 值" 对，用逗号分隔
    ( $( $key:expr => $val:expr ),* ) => {
        {
            let mut map = HashMap::new();
            // 展开循环：有多少个键值对，就执行多少次 insert
            $(
                map.insert($key, $val);
            )*
            map // 返回生成的哈希表
        }
    };
}

fn main() {
    let user_ages = hashmap! {
        "Alice" => 18,
        "Bob" => 20,
        "Charlie" => 22
    };
    
    println!("Alice's age: {:?}", user_ages.get("Alice"));
}

```

---

# 🔮 第三章：过程宏（Procedural Macros）硬核筑基

如果说声明宏是“文本替换”，那过程宏就是“把你的代码转成抽象语法树（AST），运行一段临时的 Rust 脚本去揉捏这棵树，再把改好的树吐回给编译器”。

过程宏作为编译器的插件，**必须写在独立的、类型为 `proc-macro = true` 的特殊 Crate 中**。

## 1. 建立“双项目”工作区（Workspace）环境配置

我们需要构建一个包含“宏声明库”与“业务测试项目”的多项目工作区结构。

1. **创建目录及工程:** 在终端中执行.
建立顶层工作区，包含用于写宏的 `my_macros` 库项目，以及用于测试的 `test_app` 二进制项目：

```bash
mkdir rust_macros && cd rust_macros
cargo new my_macros --lib
cargo new test_app

```


2. **配置宏库依赖:** 修改 my_macros/Cargo.toml.
你必须开启 `proc-macro = true`，并引入 `syn`（代码解析）和 `quote`（代码生成）两个神器库：

```toml
[package]
name = "my_macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true # 极其重要：声明此库是过程宏插件

[dependencies]
syn = { version = "2.0", features = ["full"] }
quote = "1.0"

```


3. **连接主应用:** 修改 test_app/Cargo.toml.
在测试项目中将刚刚建好的本地宏库作为依赖引入进来：

```toml
[dependencies]
my_macros = { path = "../my_macros" }

```


---

## 2. 手写三大过程宏实现

打开 `my_macros/src/lib.rs`，清空默认内容，将以下三大过程宏的实现完整写入：

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn, LitStr};

// =========================================================================
// 1. 自定义派生宏 (Derive Macro)
// =========================================================================
// 贴在结构体上方，自动为其实现指定的 Trait 并注入方法
#[proc_macro_derive(Hello)]
pub fn hello_derive(input: TokenStream) -> TokenStream {
    // 将代码文本解析为 DeriveInput 语法树
    let input = parse_macro_input!(input as DeriveInput);

    // 拿到结构体的标识符（即名字）
    let name = input.ident;

    // 使用 quote! 动态生成要附加的 Rust 代码
    let expanded = quote! {
        impl Hello for #name {
            fn hello() {
                println!("你好！我是自动生成的代码，我的结构体名字是：{}", stringify!(#name));
            }
        }
    };

    // 把生成的代码转回 TokenStream 交付给编译器
    TokenStream::from(expanded)
}

// =========================================================================
// 2. 属性宏 (Attribute Macro)
// =========================================================================
// 贴在函数上方，可以直接重写或包裹该函数的内部逻辑
#[proc_macro_attribute]
pub fn timer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 将被修饰的函数解析为 ItemFn 语法树
    let input_fn = parse_macro_input!(item as ItemFn);
    
    let name = &input_fn.sig.ident;   // 提取原函数名
    let block = &input_fn.block;     // 提取原函数体
    let vis = &input_fn.vis;         // 提取函数可见性
    let sig = &input_fn.sig;         // 提取完整的函数签名

    // 重新组装函数，在原函数逻辑的前后切入耗时统计代码
    let expanded = quote! {
        #vis #sig {
            let start_time = std::time::Instant::now();
            
            // 执行原函数体，并用作用域锁住，确保返回值被安全捕获
            let result = { #block };
            
            println!("⏱️ [性能监控] 函数 [{}] 执行耗时: {:?}", stringify!(#name), start_time.elapsed());
            
            result // 保持原函数返回值的一致性
        }
    };

    TokenStream::from(expanded)
}

// =========================================================================
// 3. 类函数宏 (Function-like Macro)
// =========================================================================
// 外观看起来像普通的宏调用，但内部可以用强力的 AST 解析器做各种编译期计算
#[proc_macro]
pub fn reverse_string(input: TokenStream) -> TokenStream {
    // 限定用户必须传入一个字符串字面量（如 "abc"）
    let input_lit = parse_macro_input!(input as LitStr);
    
    // 提取出真实的字符串内容
    let val = input_lit.value();
    
    // ⚠️ 重点：这是在编译期执行的！直接在编译器进程中反转字符串
    let reversed: String = val.chars().rev().collect();

    // 将反转后的字符串重新以字面量的形式写回程序，没有任何运行时开销
    let expanded = quote! {
        #reversed
    };

    TokenStream::from(expanded)
}

```

---

## 3. 揭秘底层核心：为什么变量前要加 `#` 符号？

在上面编写过程宏时，你会注意到 `quote!` 宏内部的变量前都加了 `#`（例如 `#name`、`#vis`）。

这被称为“变量插值（Interpolation）操作符”。

### 🔍 决定生死的关键对比

`quote!` 的核心职责是把宏里写的代码变成文本段，如果不做特殊标记，它根本分不清什么是“变量”，什么是“普通代码”。

* **如果不加 `#**`：
```rust
quote! { impl Hello for name {} }

```


编译器会原封不动地生成 `impl Hello for name {}`。它会去满世界寻找一个刚好叫 `name` 的结构体，导致编译直接挂掉。
* **如果加上 `#**`：
```rust
quote! { impl Hello for #name {} }

```


`quote!` 看到 `#` 就会触发替换雷达，它会读取当前宏运行上下文里 `name` 这个变量中存的值（比如 `User`），然后把值注入进去，生成 `impl Hello for User {}`。

> 💡 **概念类比：** 这与 Python 中的 `f"Hello {name}"` 或 JavaScript 中的 `Hello ${name}` 逻辑如出一辙。只不过在 Rust 声明宏中我们用 `$` 做指示符，而在过程宏的 `quote!` 库中，我们用 `#` 做指示符。

---

# 🧪 第四章：端到端整合测试

万事俱备，现在我们来到测试项目 `test_app/src/main.rs` 中，将所有手写出来的宏全部拉出来跑一遍：

```rust
// 1. 引入写好的三大过程宏
use my_macros::{Hello, timer, reverse_string};

// 2. 声明派生宏自动实现时所依赖的公共 Trait
pub trait Hello {
    fn hello();
}

// ==========================================
// 应用测试 1：验证自定义派生宏 (Derive Macro)
// ==========================================
#[derive(Hello)]
struct User {
    name: String,
}

// ==========================================
// 应用测试 2：验证属性宏 (Attribute Macro)
// ==========================================
#[timer]
fn heavy_calculation() {
    let mut sum: u64 = 0;
    for i in 0..1_000_000 {
        sum += i;
    }
    println!("计算完成，结果是：{}", sum);
}

fn main() {
    // 执行测试 1
    User::hello(); 
    // 控制台输出: 你好！我是自动生成的代码，我的结构体名字是：User

    println!("---------------------------------");

    // 执行测试 2
    heavy_calculation();
    // 控制台输出:
    // 计算完成，结果是：499999500000
    // ⏱️ [性能监控] 函数 [heavy_calculation] 执行耗时: 14.2µs

    println!("---------------------------------");

    // 执行测试 3：验证类函数宏 (Function-like Macro)
    // "Rust Macros Are Powerful" 在编译时就完成了反转，打包出的二进制里直接就是反转后的结果
    let cool_text = reverse_string!("Rust Macros Are Powerful");
    println!("反转后的文本是: {}", cool_text);
    // 控制台输出: 反转后的文本是: lufrewoP erA sorcaM tsuR
}

```

---

# 🛠️ 第五章：宏高手的调试神兵与避坑准则

## 1. 宏调试终极武器：`cargo-expand`

因为宏是在编译期展开的，一旦宏内部逻辑出错，编译器往往只能对你调用宏的那一行抛出极其抽象的报错，让人一头雾水。

你可以使用社区公认的调试神器 `cargo-expand`，它可以**把所有宏展开后的真实 Rust 代码原汁原味地还原出来**：

```bash
# 1. 安装扩展
cargo install cargo-expand

# 2. 在项目根目录下直接运行
cargo expand

```

运行后，你会看到你的 `#[timer]` 宏是如何魔改原函数的，所有的黑魔法在它面前都会现出原形。

## 2. 宏的终极使用哲学

> ⚠️ **能用函数解决的问题，绝对不要写宏。**

* **代价：** 宏会显著增加编译时间，破坏编辑器的代码自动补全与跳转体验，并大大提高代码的理解门槛。
* **合适的使用场景：**
1. 普通函数无论如何都无法优雅消除的巨量模板代码（使用**声明宏**）。
2. 需要在编译期对类型结构体做深度解析，自动实现某些复杂 Trait（使用**派生宏**，如 `serde` 的序列化）。
3. 需要在编译期自制语法解析器（DSL），如在 Rust 里直接校验 SQL 语句合法性或解析 HTML（使用**属性宏/类函数宏**）。

---

# ⚔️ 第六章：声明宏 vs 类函数宏——核心差异全景

这是一个经典的困惑。因为在**调用**它们的时候，它们长得几乎一模一样，都是 `名称!(...)` 的形式（比如声明宏 `vec![1, 2, 3]` 和类函数宏 `reverse_string!("hello")`）。

但如果把引擎盖打开，你会发现它们的**底层逻辑、运行机制和能力上限**有着天壤之别。

## 📊 声明宏 vs 类函数宏 核心对比

| 特性 | 声明宏 (`macro_rules!`) | 类函数宏 (`#[proc_macro]`) |
| --- | --- | --- |
| **本质是什么** | **模式匹配与文本替换**（类似于高阶的“查找与替换”） | **编译期运行的 Rust 程序**（是一个真正的函数） |
| **输入与输出** | 符合 Rust 指定碎片类型（如 `expr`, `ident`）的标记 | 任意的标记流（`TokenStream` -> `TokenStream`） |
| **能力上限** | 只能做局部的语法树替换和规整的循环展开 | **无限**。可以解析非 Rust 语法、读写文件、甚至发网络请求 |
| **编写位置** | 可以写在项目的**任何地方**，随写随用 | 必须写在独立的、配置为 `proc-macro = true` 的 Crate 里 |
| **编译速度** | 相对较快 | 较慢（因为要先编译宏本身，再运行宏去编译主程序） |
| **调试难度** | 困难（报错通常很隐晦） | 极其困难（但可以通过自定义错误并精准定位到源码行） |

## 🔍 核心差异深度拆解

### 1. 运行机制：匹配 vs 编程

* **声明宏（Declarative Macro）：**
它就像是一个“复印机”。你给它设定好图纸（`match` 模式），它看到符合图纸的代码，就按照你写的模板复印一份贴过去。它本身**不具备逻辑计算能力**，你不能在声明宏里写 `if a > b` 或者用 `.chars().rev()` 去反转字符串。
* **类函数宏（Function-like Procedural Macro）：**
它是一个真正的“加工厂”。编译器在遇到它时，会把括号里的代码打包成一段文本标记流（`TokenStream`），作为参数传给你的宏函数。在宏函数内部，你可以用**完整的、毫无保留的 Rust 语言**去处理这段文本（比如利用 `syn` 库解析成语法树，用 `quote` 动态生成新代码）。

### 2. 语法容忍度：必须合法 vs 任意创造

* **声明宏：**
括号里传入的代码，**必须符合 Rust 的基本语法碎片的定义**。例如你指定了 `$x:expr`，那传入的就必须是一个合法的 Rust 表达式。你不能在里面瞎写一段 HTML 标签（如 `<div class="box">`），编译器在匹配阶段就会直接报错。
* **类函数宏：**
它接收的是最原始的 `TokenStream`，它**不要求传入的代码符合 Rust 语法**！这意味着你可以发明属于你自己的全新语言。这也是为什么类似前端框架 Leptos 的 `view!` 宏可以在里面直接写原生的 HTML，或者有些库可以在宏里直接写 SQL 语句。

### 3. 开发成本与依赖

* **声明宏：**
零成本。你只需要在你的 `main.rs` 或任何模块里敲下 `macro_rules! my_macro { ... }` 就能立刻开始用，不需要引入任何第三方库。
* **类函数宏：**
成本极高。你得大费周章地建一个双项目工作区，修改 `Cargo.toml`，引入 `syn` 和 `quote` 库，还要小心翼翼地处理编译顺序。

> 💡 **一句话总结你的技术选型：**
> 如果你只是想写一个快捷方式，少敲几行重复的 Rust 代码（比如批量生成相似的函数，或者做一个灵活的初始化脚手架），用**声明宏**；如果你需要魔改语法、解析非 Rust 文本、或者根据外部条件（如数据库结构、本地文件）在编译期动态生成代码，则必须动用**类函数宏**。

---

# 🧬 第七章：Rust 核心语法——Trait（特质）全景

在前面使用 `impl IntoView` 时，我们已经接触了 trait。现在把它彻底讲透。

## 1. Trait 是什么？为什么需要它？

Trait 是 Rust 的“行为契约”，类似于其他语言中的接口（interface）。它定义了一组方法签名，任何实现了这些方法的类型都被认为“遵守了这个契约”。

例如 `IntoView` 这个 Trait：任何能被渲染成浏览器视图的类型（组件、HTML 元素等）都必须实现 `IntoView`。`#[component]` 宏会自动为组件实现 `IntoView`。

**为什么 Rust 用 Trait 而不是继承：** Rust 没有类和继承机制。Trait 提供了更灵活的组合方式：一个类型可以同时实现多个 Trait，而且 Trait 还可以提供默认实现（类似 Java 8 的 default 方法）。这种“组合优于继承”的设计让代码更灵活、更易于维护。

## 2. 实战：自己写一个 Trait

下面这个例子会完整展示：定义 Trait → 不同类型实现 → 传入不同参数 → 产生不同结果。

场景：一个“支付系统”，不同支付方式（微信、支付宝、银行卡）的“支付”行为不一样，而且即使同一种方式，传入不同的金额/订单号，结果也不同。

```rust
// 1. 定义一个名为 Payment 的 Trait，要求实现者必须提供 pay 方法
trait Payment {
    fn pay(&self, amount: u64, order_id: &str) -> String;
}

// 2. 定义三个不同的支付方式结构体
struct WeChatPay;
struct Alipay;
struct BankCard;

// 3. 为不同方式实现 Payment Trait，行为各不相同
impl Payment for WeChatPay {
    fn pay(&self, amount: u64, order_id: &str) -> String {
        format!("微信支付成功：订单 {}，金额 {} 元", order_id, amount)
    }
}

impl Payment for Alipay {
    fn pay(&self, amount: u64, order_id: &str) -> String {
        format!("支付宝付款成功：订单 {}，金额 {} 元", order_id, amount)
    }
}

impl Payment for BankCard {
    fn pay(&self, amount: u64, order_id: &str) -> String {
        format!("银行卡扣款成功：订单 {}，金额 {} 元", order_id, amount)
    }
}

// 4. 使用 impl Trait 作为参数，传入不同实现者和不同参数
fn process_payment(method: impl Payment, amount: u64, order_id: &str) {
    println!("{}", method.pay(amount, order_id));
}
```

调用示例：

```rust
process_payment(WeChatPay, 100, "ORD-001"); // 微信支付成功：订单 ORD-001，金额 100 元
process_payment(Alipay, 200, "ORD-002");    // 支付宝付款成功：订单 ORD-002，金额 200 元
process_payment(BankCard, 300, "ORD-003");  // 银行卡扣款成功：订单 ORD-003，金额 300 元
```

关键点：同一个函数 `process_payment`，传入不同的“实现者”会产生完全不同的输出；即使传入相同的实现者，只要金额或订单号不同，结果也会不同。这就是 Rust Trait 的多态：通过统一的接口，让不同类型的对象表现出不同的行为。

## 3. 回到 Leptos

`IntoView` 也是这样一个 Trait。`view! { <div>hello</div> }`、`view! { <span>world</span> }` 是两个完全不同类型的视图，但它们都实现了 `IntoView`，所以你的 `App` 函数可以放心地返回“任意一种”，编译器会自动处理。

---

# 🌐 第八章：Leptos 组件与 `#[component]` 宏

Leptos 是一个 Rust 全栈前端框架。它的核心之一，就是让你用普通的 Rust 函数定义**组件（Component）**，然后在 `view!` 模板里像写自定义 HTML 标签一样使用它们。本章结合官方 `#[component]` 宏文档，系统讲解其用法。

## 1. `#[component]` 宏是什么？

`#[component]` 是一个**属性宏**（属于过程宏的一种，见第三章）。它给一个普通函数加上标注，使该函数可以被当作 Leptos 组件，在模板里以 `<Component/>` 的形式直接使用。

- 组件函数可以接收任意多个参数，这些参数名在你使用组件时就变成了**属性的名字（Props）**。
- 每个组件函数都应返回 `-> impl IntoView`（回顾第七章：Trait 让函数能返回任意实现了 `IntoView` 的视图类型）。
- 你可以给函数参数写 Rust 文档注释（`///`），宏会自动把它们生成为组件的文档。

## 2. 定义与使用组件

下面是一个接收 `name` 与 `age` 两个自定义属性的简单组件：

```rust
use std::time::Duration;
use leptos::prelude::*;

#[component]
fn HelloComponent(
    /// 用户的名字
    name: String,
    /// 用户的年龄
    age: u8,
) -> impl IntoView {
    // 创建响应式信号（signal），当值变化时 UI 会自动更新
    let (age, set_age) = create_signal(age);
    // 每秒把 age 加 1
    set_interval(
        move || set_age.update(|age| *age += 1),
        Duration::from_secs(1),
    );

    // 返回界面。信号变化时会自动重渲染
    view! {
        <p>"Your name is " {name} " and you are " {move || age.get()} " years old."</p>
    }
}

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <HelloComponent name="Greg".to_string() age=32/>
        </main>
    }
}
```

使用时 `<HelloComponent name=... age=.../>` 的属性名，正对应函数参数名。

## 3. 组件的运行机制

理解 Leptos 组件有两条关键结论：

- **组件函数只运行一次。** 它不是“每次状态变化就重跑一次的渲染函数”，而是**只运行一次的“初始化（setup）函数”**：它负责创建界面，并搭好一套响应式系统来更新界面。因此，在组件函数里做稍微昂贵的工作是没问题的——它只会发生一次，而不是每次状态变化都发生。
- **组件名通常用 PascalCase（大驼峰）。** 即使你用 `snake_case`（蛇形命名）写函数名，生成的组件名依然是 PascalCase。框架正是靠这个规则来区分“这是组件”还是“原生 HTML 元素”。

```rust
// PascalCase：生成的组件名为 MyComponent
#[component]
fn MyComponent() -> impl IntoView {}

// snake_case：生成的组件名仍为 MySnakeCaseComponent
#[component]
fn my_snake_case_component() -> impl IntoView {}
```

## 4. 组件命名规则小结

框架识别组件靠的是 PascalCase 名称。函数可以是 snake_case，但宏生成后的组件标识符一律转成大驼峰，所以 `<MySnakeCaseComponent/>` 才是正确的模板写法。

## 5. 子组件（Children）

用 `children` 属性可以拿到组件内部包裹的子内容，类型为 `Children`（它是 `Box<dyn FnOnce() -> AnyView>` 的别名）：

- 若需要 `Fn` 或 `FnMut` 语义，可用 `ChildrenFn` / `ChildrenFnMut` 别名。
- 若想遍历子节点，可用 `ChildrenFragment`。

```rust
#[component]
fn ComponentWithChildren(children: ChildrenFragment) -> impl IntoView {
    view! {
        <ul>
            {children()
                .nodes
                .into_iter()
                .map(|child| view! { <li>{child}</li> })
                .collect::<Vec<_>>()}
        </ul>
    }
}

#[component]
fn WrapSomeChildren() -> impl IntoView {
    view! {
        <ComponentWithChildren>
            "Ooh, look at us!"
            <span>"We're being projected!"</span>
        </ComponentWithChildren>
    }
}
```

## 6. 自定义属性（Props）

可以在单个组件参数上用 `#[prop]` 属性定制属性的接收方式：

| 属性 | 作用 |
| --- | --- |
| `#[prop(into)]` | 对传入的值自动调用 `.into()`（例如属性类型为 `Signal`，用户可传 `ReadSignal`/`RwSignal` 自动转换） |
| `#[prop(optional)]` | 使用时不传该属性则取默认值；类型为 `Option<T>` 时按 `name=T` 传入，收到 `Some(T)` |
| `#[prop(optional_no_strip)]` | 同上，但必须显式传 `None` 或 `Some(T)`（可省略不传即得到 `None`） |
| `#[prop(default = <expr>)]` | 指定属性默认值，未传时使用 |
| `#[prop(name = "new_name")]` | 指定属性的对外名称（可用于解构结构体字段，见下例） |
| `#[prop(marker)]` | 标记该属性为仅用于默认的占位符，不出现在文档与构造器中（常用于泛型组件，如 `#[prop(marker)] _marker: PhantomData<T>`） |

```rust
#[component]
pub fn MyComponent(
    #[prop(into)] name: String,
    #[prop(optional)] optional_value: Option<i32>,
    #[prop(optional_no_strip)] optional_no_strip: Option<i32>,
    #[prop(default = 7)] optional_default: i32,
    #[prop(name = "data")] UserInfo { email, user_id }: UserInfo,
) -> impl IntoView {
    // 任意所需界面
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <MyComponent
            name="Greg"               // 自动 .into() 成 String
            optional_value=42         // 收到 Some(42)
            optional_no_strip=Some(42)// 收到 Some(42)
            optional_default=42       // 收到 42
            data=UserInfo { email: "foo", user_id: "bar" }
        />
        <MyComponent
            name="Bob"
            data=UserInfo { email: "foo", user_id: "bar" }
            // 可选属性可省略
        />
    }
}

pub struct UserInfo {
    pub email: &'static str,
    pub user_id: &'static str,
}
```

> 💡 小结：Leptos 的 `#[component]` 宏把"普通函数 + 属性参数 + `impl IntoView` 返回"自动改造成可在 `view!` 模板里当作自定义标签使用的组件，并负责生成结构体、属性解析器与 `IntoView` 实现。它正是第三章所讲"属性宏"能力的典型应用。