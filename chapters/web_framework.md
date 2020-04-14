## Rust Web 框架介绍

由于Rust生态系统还处于早期阶段，因此框架有很多选择。每个人都有自己的优点和缺点，没有明显的赢家。

### [Hyper](https://github.com/hyperium/hyper)

第一个出场的就是hyper，它的特点就是高性能，后面会给出的压测结果，跟 actix-web 差不多；另外首先实现了 Client 组件，方便写单元测试验证；其实很多 web 框架基于 hyper 实现，侧面说明他底层的封装还是不错的。不过它也有些缺点：
hyper应用侧的功能相对少，所以会导致很多框架又在他的基础上继续封装；

### [Actix-web](https://actix.rs/)

Actix-web 是一个基于 Actix 构建的框架，Actix 是 Rust 的一个 actor 系统。它虽然成立时间没有 Rocket 那么长，但已经获得了另一个社区的最爱。独特的 actor 方法意味着单独的组件（如数据库访问和后台任务）都被实现为异步actor，它们通过消息传递相互通信。Actix-web 可能因出现在 [TechEmpower Web 框架基准测试](https://www.techempower.com/benchmarks/)的排名顶端而闻名。Actix-web 正在积极开发中，并且具有相当全面的文档。


### [Rocket](https://rocket.rs/)

Rocket 是一个十几岁的框架 - 比其他许多框架更发达，但仍然不太成熟。它的特殊功能是通过宏来注释请求处理函数，这个宏包括路由、参数和所需的数据，例如有效的反序列化形式，以及定义一种依赖注入。此外，文档非常好，开发活跃，并且与 Actix 一起，这是最常用的框架之一，因此可以从发展的社区知识中受益。Rocket 需要 Rust 的 nightly 以上版本。


### [Tower-web](https://github.com/carllerche/tower-web)

Tower-web是另一个平易近人的框架，旨在提供所有标准功能。它基于 Tower：一个网络客户端/服务器组件库，这意味着它最终应该获得 “batteries included” 状态。它也是由 Rust 最流行的异步运行时库 Tokio 的核心贡献者之一开发的，这似乎是一个优点。像火箭一样，Tower-web 使用宏来减少样板，但不需要 Rust nightly 。由于它是新的，它仍然缺少功能和文档的方式，但正在积极开发。


### [Warp](https://github.com/seanmonstar/warp)

Warp 是一个具有独特可组合性角度的框架，允许将可重复使用的“过滤器”链接在一起，这些过滤器可用于参数提取或包括所需应用程序状态，这样可以构建路由和请求处理程序。它在文档方面也很新颖，但在积极开发中。其开发人员和 Tower-web 的开发人员彼此都比较了解，并且可能在未来将 Warp 和 Tower-web 合并为单个框架。


### [Yew](https://github.com/yewstack/yew) （前端）

受 Elm 和 React 启发的前端框架启发，Yew 利用 Rust 的能力编译到 WebAssembly。似乎它与 JavaScript 有良好的互操作性，并且已经足够成熟可使用。它没有很多文档，但确实有很多例子，并且正在积极开发中。


### [Diesel](https://github.com/diesel-rs/diesel)（ORM）

Diesel是Rust的事实上的ORM解决方案。它支持迁移，模式生成，并且具有构建DSL的良好查询。我在过去使用MySQL时遇到了问题，看起来像Postgres是它受欢迎的数据库（足够公平），但是开发是活跃的。