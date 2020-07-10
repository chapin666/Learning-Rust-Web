
## Actix 和 Diesel 集成测试

Rust的编译器在编译时捕获 bug 方面做得非常出色。 如此之多，以至于我在编译代码时总是充满信心。 尽管在Rust中发生运行时错误的可能性很小（除非您是 unsafe 的粉丝)。但我们仍然会遇到逻辑错误， 为了确保我们也能处理这些错误，我们可以创建集成测试。

### 运行测试前

在运行测试之前，我们可能需要启动数据库或其他东西。为了做到这一点，我们将来也许可以使用[自定义的 test 框架](https://doc.rust-lang.org/unstable-book/language-features/custom-test-frameworks.html)，但是现在，我们将创建一个在每个测试开始时都需要的函数。我们还需要确保这个函数只在任何其他测试尚未启动的情况下进行启动。

```
// src/test.rs
use crate::db;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use dotenv::dotenv;

lazy_static! {
   static ref INITIATED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[cfg(test)]
pub fn init() {
   let mut initiated = INITIATED.lock().unwrap();
   if *initiated == false {
       dotenv().ok();
       db::init();
       *initiated = true;
   }
}
```

在这里，我们使用一个惰性静态函数来检查是否已启动测试。 由于默认情况下，Rust中的测试将并行运行，因此我们还需要使此变量成为线程安全的。 为此，我们在 Arc<Mutex> 中定义此变量。

另外，注意 #[cfg(test)] 注释，它告诉编译器只在运行测试时编译这个函数。

我们还需要将这个模块添加到主文件中。

```
// src/main.rs
#[cfg(test)]
mod test;
// ..
```

### 创建我们的第一个测试

>注意: Test::read_body_json() 和 test::TestRequest::send_request() 是当前不属于 activex-web 的新函数。我已经创建了一个 [PR](https://github.com/actix/actix-web/pull/1401)，所以我们希望很快看到他们作为 activex-web 一个部分。同时你可以把这个也加到你的 Cargo.toml:
>```
> actix-web = { version = "2.0", git = "https://github.com/thecloudmaker/actix-web", branch = "integration-tests" }
> ```

现在让我们定义第一个测试。我们可以在 Rust 中定义一个测试，方法是创建一个函数并用 #[test]对其进行注释。此注释不适用于异步函数，因此为了支持使用 #[activex_rt::test] 注释。

现在我们知道了如何创建测试，我们可以为用户接口创建第一个测试。

```
// src/user/tests.rs
#[cfg(test)]
mod tests {
   use crate::user::*;
   use actix_web::{test::{self, TestRequest}, App};
   use serde_json::json;

   #[actix_rt::test]
   async fn test_user() {
       crate::test::init();

       let request_body = json!({
           "email": "tore@cloudmaker.dev",
           "password": "test",
       });

       let mut app = test::init_service(App::new().configure(init_routes)).await;

       let resp = TestRequest::post().uri("/users").set_json(&request_body).send_request(&mut app).await;
       assert!(resp.status().is_success(), "Failed to create user");
       let user: User = test::read_body_json(resp).await;

       let resp = TestRequest::post().uri("/users").set_json(&request_body).send_request(&mut app).await;
       assert!(resp.status().is_client_error(), "Should not be possible to create user with same email twice");

       let resp = TestRequest::get().uri(&format!("/users/{}", user.id)).send_request(&mut app).await;
       assert!(resp.status().is_success(), "Failed to find user");

       let user: User = test::read_body_json(resp).await;
       assert_eq!(user.email, "tore@cloudmaker.dev", "Found wrong user");

       let request_body = json!({
           "email": "tore@cloudmaker.dev",
           "password": "new",
       });

       let resp = TestRequest::put().uri(&format!("/users/{}", user.id)).set_json(&request_body).send_request(&mut app).await;
       assert!(resp.status().is_success(), "Failed to update user");

       let user: User = test::read_body_json(resp).await;
       assert_eq!("new", user.password, "Failed to change password for user");

       let resp = TestRequest::delete().uri(&format!("/users/{}", user.id)).send_request(&mut app).await;
       assert!(resp.status().is_success(), "Failed to delete user");

       let resp = TestRequest::get().uri(&format!("/users/{}", user.id)).send_request(&mut app).await;
       assert!(resp.status().is_client_error(), "It should not be possible to find the user after deletion");
   }
}
```
在这里，我们首先初始化一个测试服务器，并使用 test::init_service 为用户接口提供路由。然后我们可以使用 TestRequest 发送测试请求。

我们的第一个测试是创建一个用户，我们也将用它作为其余测试的基础。第二个测试是检查我们是否应该不能创建具有相同电子邮件地址的第二个用户。

下面的测试检查我们是否能够找到用户，更新它，然后最终删除它。在这里我们也使用删除测试来清理自己。如果在这个测试之后不进行清理，那么在第二次运行测试时就会遇到问题。这是因为第一个测试将尝试创建一个用户，而我们在第一次运行测试时已经创建了这个用户。

在运行测试之前，我们还需要记住将测试添加到用户模块。

```
// src/user/mod.rs
mod tests;
// ..
```

现在让我们来做测试。

```
$ cargo test
```

### 有没有更简单的方法来清理测试之间的数据？

假设我们在整个测试过程中创建了大量数据，那么创建一个测试来在测试之后清除这些数据是没有意义的。那么我们还有什么选择呢？事实上，我们有好几个。我要提到一些。

一种方法是在运行测试之前删除所有数据库。这将给我们一张白纸，因为 init 函数已经在运行测试之前负责再次运行迁移。

另一种方法是启动几个 Docker 容器并在其中运行它们。然后在每次测试后把它们扔掉。

我将提到的最后一个选项是为我们的测试启动一个永远不会提交的数据库事务。我将展示如何使用 Diesel 来帮助我们完成这个操作。

> 注意： 

> 在 Diesel 中有一个[未决的问题](https://github.com/diesel-rs/diesel/issues/2123)，这个问题导致了一些测试，这些测试正在验证数据库是否正确地返回了一个错误消息。
> 例如，我们的测试是为了确保我们不能用同一封邮件创建一个用户两次。因此，我们应该注释掉该测试，直到解决此问题。。

让我们从注释上一个删除用户的测试开始。如果我们现在运行这个测试两次，你会注意到它第二次会失败，并且会显示“ Failed to create user”消息。这是因为我们正在尝试创建一个已经存在的用户。

现在，让我们用 begin_test_transaction() 启动测试事务。事务不能在多个连接之间共享，因此我们还需要将连接池大小设置为 1，以确保所有测试都使用相同的连接。

```
// src/db.rs
use diesel::prelude::*;
// ..

lazy_static! {
   static ref POOL: Pool = {
       let db_url = env::var("DATABASE_URL").expect("Database url not set");
       let manager = ConnectionManager::<PgConnection>::new(db_url);
       let pool_size = match cfg!(test) {
           true => 1,
           false => 10,
       };
       r2d2::Builder::new().max_size(pool_size).build(manager).expect("Failed to create db pool")
   };
}

pub fn init() {
   info!("Initializing DB");
   lazy_static::initialize(&POOL);
   let conn = connection().expect("Failed to get db connection");
   if cfg!(test) {
       conn.begin_test_transaction().expect("Failed to start transaction");
   }
   embedded_migrations::run(&conn).unwrap();
}
```

在这里，我们使用cfg宏启动测试事务并将池大小设置为1。 通常，除实际测试外，我建议仅谨慎使用 #[cfg(test)] 属性和 cfg!(test) 宏。原因是您可以编写能够通过测试但甚至可能不能编译的代码。

例如，您可以尝试将 #[cfg(test)] 注释放在数据库 init 函数的顶部。如果你现在运行 cargo test 一切似乎都可以，但如果你现在运行 cargo run 呢？

无论如何，您现在应该能够看到，我们现在可以运行 cargo test 多次，没有失败，因为我们的数据永远不会提交到数据库。