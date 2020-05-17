## REST API

在本教程中，我们将使用 activex web 2.0 和 Diesel 在 Rust 中创建一个 REST API。 我们将使用 Postgres 作为我们的数据库，所以如果你的计算机上没有安装 Postgres，你应该首先这样做。

### Hello world

我们将首先使用 Cargo 创建我们的项目，然后进入项目目录。

```
$ cargo new rest_api
$ cd rest_api
```

在第一个例子中，我们需要将 activex web 添加到依赖项中。

```
[dependencies]
actix-web = "2.0"
actix-rt = "1.0"
```

> 个人觉得手动添加依赖比较麻烦，建议使用 [cargo-edit](https://github.com/killercup/cargo-edit) 来自动添加.

然后在 src / main.rs 中设置请求处理程序和服务器。

```
// src/main.rs
use actix_web::{App, HttpResponse, HttpServer, Responder, get};

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
    })
        .bind("127.0.0.1:5000")?
        .run()
        .await
}
```

现在我们已经创建了第一个服务器，让我们用 cargo run 运行它。 要测试我们的 REST API，让我们访问 localhost: 5000，我们希望看到 hello world。

### 自动重新加载

每次修改代码后都手动重新编译，可能相当繁琐，所以让 cargo-watch 在每次更改时为我们重新编译代码。 将它与 listenfd crate 和 systemfd 实用程序结合在一起。在代码重新编译的同时，保持连接的开放性也是很有意义的。 这样，就可以避免我们的 REST 客户端在代码重新编译时中断请求，因为它无法到达服务器。 通过保持连接处于打开状态，我们只需要向服务器发出一个调用，一旦服务器重新编译并准备好处理我们的请求，它就会立即响应。

为此，我们需要安装 cargo-watch 和 systemfd。 两者都是用 Rust 编写的，可以在 crates.io 上使用，因此我们可以将它们与 cargo 一起安装。

```
cargo install systemfd cargo-watch
```

我们还需要将 listenfd 添加到依赖项中。

```
[dependencies]
listenfd = "0.3"
```

然后我们需要对 src / main 进行一些更改。 这样我们就可以使用 systemfd 为我们提供侦听器，但是当我们不需要它的时候，也有一个后备方案。比如我们部署代码的时候。

```
// src/main.rs
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use listenfd::ListenFd;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(||
        App::new()
            .service(index)
    );

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => server.bind("127.0.0.1:5000")?,
    };

    server.run().await
}
```

现在，我们可以运行服务器和文件监视器，修改后会使用这个命令自动重新编译我们的代码。

```
$ systemfd --no-pid -s http::5000 -- cargo watch -x run
```

### 环境变量和日志记录

您可能会在某个时候部署代码。 然后，您可能希望使用与本地计算机上不同的设置运行服务器，比如使用不同的端口或不同级别的日志记录。 您可能还需要使用一些不应该出现在代码中的秘密，比如数据库密码。 为此，我们可以使用环境变量。

另外，当您部署代码时，您可以确保代码在某个时候会遇到问题。 为了帮助解决这些问题，良好的日志记录非常重要，这样我们就可以找出问题所在并解决问题。

为了设置环境变量和日志，我们将添加另外一些依赖项。

```
[dependencies]
dotenv = "0.11"
log = "0.4"
env_logger = "0.6"
```

为了方便起见，让我们设置一些可用于开发的默认参数。 我们可以在根目录创建一个 .env 文件。
```
RUST_LOG=rest_api=info,actix=info
HOST=127.0.0.1
PORT=5000
```

log crate 提供了 error, warn, info, debug 和 trace 五种不同的日志级别。其中 error 表示最高优先级的日志消息，trace 表示最低优先级的日志消息。 在本教程中，我们将把 REST API 和 activex 的日志级别设置为 info，这意味着我们将从 error, warn 和 info 获取所有消息。

要激活日志和环境变量文件，我们只需要对主文件做一些小的修改。

```
// src/main.rs
#[macro_use]
extern crate log;

use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use listenfd::ListenFd;
use std::env;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(||
        App::new()
            .service(index)
    );

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("Host not set");
            let port = env::var("PORT").expect("Port not set");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await
}
```
dotenv().ok() 数获取环境变量， 并将它们添加到我们的服务器环境变量中。 通过这种方式，我们可以使用 std::env::var() 函数来使用这些变量，就像我们设置主机和端口时所做的那样。

log crate 提供了五个宏让我们来写日志消息。分别是：error!, warn!, info! debug! 和 trace!。要在 stdout 或 stderr 中查看日志消息，我们需要初始化 env_logger，并使用一个函数执行这个操作: env_logger::init()。

### Api 

我们的 API 将发送和接收 json 数据，因此我们需要一种方法将 json 序列化和反序列化到 Rust 所识别的数据结构中。 为此，我们将使用 Serde。 我们需要将其添加到依赖项列表中。
```
[dependencies]
serde = "1.0"
serde_json = "1.0"
```

现在我们将定义一个用户模型，并添加 Serialize 和 Deserialize 注释，这样我们的模型就可以从中提取并转换为 json。

```
// src/user/model.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub email: String,
}
```

让我们继续创建我们的 REST API。 我们的下一步将是持久化数据，但目前我们只使用硬编码虚拟数据。
```
// src/user/routes.rs
use crate::user::User;
use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;

#[get("/users")]
async fn find_all() -> impl Responder {
    HttpResponse::Ok().json(
        vec![
            User { id: 1, email: "tore@cloudmaker.dev".to_string() },
            User { id: 2, email: "tore@cloudmaker.dev".to_string() },
        ]
    )
}

#[get("/users/{id}")]
async fn find() -> impl Responder {
    HttpResponse::Ok().json(
        User { id: 1, email: "tore@cloudmaker.dev".to_string() }
    )
}

#[post("/users")]
async fn create(user: web::Json<User>) -> impl Responder {
    HttpResponse::Created().json(user.into_inner())
}

#[put("/users/{id}")]
async fn update(user: web::Json<User>) -> impl Responder {
    HttpResponse::Ok().json(user.into_inner())
}

#[delete("/users/{id}")]
async fn delete() -> impl Responder {
    HttpResponse::Ok().json(json!({"message": "Deleted"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);
}
```
我们还需要将用户路由与用户模型连接起来，并使其在用户目录之外可用。
```
// src/user/mod.rs
mod model;
mod routes;

pub use model::User;
pub use routes::init_routes;
```
现在，我们可以用实际的 用户API 替换 "helloworld"。
```
// src/main.rs
#[macro_use]
extern crate log;

use actix_web::{App, HttpServer};
use dotenv::dotenv;
use listenfd::ListenFd;
use std::env;

mod user;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| 
        App::new()
            .configure(user::init_routes)
    );

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("Host not set");
            let port = env::var("PORT").expect("Port not set");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await
}
```
我们现在调用刚刚创建的用户接口，应该能够测试通过。 例如，你可以使用 [Insomnia](https://insomnia.rest/) 或者 [curl](https://curl.haxx.se/)。

### 持久化数据

如果我们不能持久保存数据，那么拥有几个接口实际上是没有帮助的。 为此，我们将使用 Diesel，它是一个相当成熟的 ORM。 允许我们连接到 Postgres，MySQL 和 SQLite，但是在这个教程中我们只涉及到 Postgres。

Diesel 依赖于 [openssl](https://www.openssl.org/source/) 和 [libpq](https://www.postgresql.org/download/)，因此我们需要在安装 Diesel CLI 之前安装它们。 如果你正在使用一个类似 Debian 的操作系统，你可以简单的使用 apt 安装它。

```
$ sudo apt install openssl libpq-dev -y
```

当我们安装了所需的依赖项，我们可以安装 Diesel CLI。

```
 cargo install diesel_cli --no-default-features --features postgres
```

为了让 Diesel 知道我们的数据库在哪里，我们需要将 DATABASE url 添加到我们的 .env 文件中。

```
DATABASE_URL=postgres://postgres:password@localhost/rest_api
```

我们可以使用 Diesel CLI 在我们的项目中设置 Diesel，并为我们的用户迁移创建文件。

```
$ diesel setup
$ diesel migration generate create_user
```

在迁移文件夹中，我们现在应该能够找到第一次迁移的文件夹。 此文件夹应包含两个文件。 一个名为 up.sql，我们将在其中创建用户model，另一个名为 down.sql，它将恢复我们在 up.sql 文件中所做的一切。
```
// up.sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE "user" (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    created_at TIMESTAMP   NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMP
);
```
```
// down.sql
DROP TABLE "user";
```

现在我们已经完成了第一次迁移，我们可以使用 Diesel CLI 运行它。

```
$ diesel migration run
```

这个命令还应该创建一个 model 文件，稍后我们将使用这个文件来构建 sql 查询。 此文件的默认位置是 src/schema.rs

在处理数据库时，我们应该为可能发生的连接问题或数据库冲突等问题做好准备。 因此我们将创建一个自己的错误类型来处理这些问题。
```
// src/api_error.rs
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use serde::Deserialize;
use serde_json::json;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub status_code: u16,
    pub message: String,
}

impl ApiError {
    pub fn new(status_code: u16, message: String) -> ApiError {
        ApiError { status_code, message }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl From<DieselError> for ApiError {
    fn from(error: DieselError) -> ApiError {
        match error {
            DieselError::DatabaseError(_, err) => ApiError::new(409, err.message().to_string()),
            DieselError::NotFound => ApiError::new(404, "Record not found".to_string()),
            err => ApiError::new(500, format!("Diesel error: {}", err)),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match StatusCode::from_u16(self.status_code) {
            Ok(status_code) => status_code,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match status_code.as_u16() < 500 {
            true => self.message.clone(),
            false => {
                error!("{}", self.message);
                "Internal server error".to_string()
            },
        };

        HttpResponse::build(status_code)
            .json(json!({ "message": message }))
    }
}
```

错误类型包含一个状态码和一个消息，我们将使用这个消息来创建错误消息。 通过实现 ResponseError 来创建错误消息，使用 ResponseError 来创建 json 响应。

如果我们有一个内部服务器错误，对用户不能更好的表达。 对于这种情况，我们只是让用户知道出了问题，并将错误消息写入日志。

我们的错误类型也实现了 From<diesel::result::Error>，这样我们就不必每次都处理一个 Diesel 错误。

```
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
diesel = { version = "1.4", features = ["postgres", "r2d2", "uuid", "chrono"] }
diesel_migrations = "1.4"
lazy_static = "1.4"
r2d2 = "0.8"
uuid = { version = "0.6", features = ["serde", "v4"] }
```

对于处理状态，我们将使用静态，尽管 activex 已经内置了状态管理。 您可以阅读我关于松散耦合的文章，以了解我为什么决定采用这种方法，尽管有些人可能不同意。

现在，让我们建立一个数据库连接，并使用 r2d2有效地处理连接池。

```
// src/db.rs
use crate::api_error::ApiError;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use lazy_static::lazy_static;
use r2d2;
use std::env;

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

embed_migrations!();

lazy_static! {
    static ref POOL: Pool = {
        let db_url = env::var("DATABASE_URL").expect("Database url not set");
        let manager = ConnectionManager::<PgConnection>::new(db_url);
        Pool::new(manager).expect("Failed to create db pool")
    };
}

pub fn init() {
    info!("Initializing DB");
    lazy_static::initialize(&POOL);
    let conn = connection().expect("Failed to get db connection");
    embedded_migrations::run(&conn).unwrap();
}

pub fn connection() -> Result<DbConnection, ApiError> {
    POOL.get()
        .map_err(|e| ApiError::new(500, format!("Failed getting db connection: {}", e)))
}
```

建立了数据库连接后，我们终于可以创建用于创建、读取、更新和删除用户数据的 API。
```
// src/user/model.rs
use crate::api_error::ApiError;
use crate::db;
use crate::schema::user;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, AsChangeset)]
#[table_name = "user"]
pub struct UserMessage {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "user"]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

impl User {
    pub fn find_all() -> Result<Vec<Self>, ApiError> {
        let conn = db::connection()?;

        let users = user::table
            .load::<User>(&conn)?;

        Ok(users)
    }

    pub fn find(id: Uuid) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = user::table
            .filter(user::id.eq(id))
            .first(&conn)?;

        Ok(user)
    }

    pub fn create(user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = User::from(user);
        let user = diesel::insert_into(user::table)
            .values(user)
            .get_result(&conn)?;

        Ok(user)
    }

    pub fn update(id: Uuid, user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = diesel::update(user::table)
            .filter(user::id.eq(id))
            .set(user)
            .get_result(&conn)?;

        Ok(user)
    }

    pub fn delete(id: Uuid) -> Result<usize, ApiError> {
        let conn = db::connection()?;

        let res = diesel::delete(
                user::table
                    .filter(user::id.eq(id))
            )
            .execute(&conn)?;

        Ok(res)
    }
}

impl From<UserMessage> for User {
    fn from(user: UserMessage) -> Self {
        User {
            id: Uuid::new_v4(),
            email: user.email,
            password: user.password,
            created_at: Utc::now().naive_utc(),
            updated_at: None,
        }
    }
}
```

有了用户应用程序接口，我们就可以使用它来代替之前使用的虚假数据。

```
// src/user/routes.rs
use crate::api_error::ApiError;
use crate::user::{User, UserMessage};
use actix_web::{delete, get, post, put, web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

#[get("/users")]
async fn find_all() -> Result<HttpResponse, ApiError> {
    let users = User::find_all()?;
    Ok(HttpResponse::Ok().json(users))
}

#[get("/users/{id}")]
async fn find(id: web::Path<Uuid>) -> Result<HttpResponse, ApiError> {
    let user = User::find(id.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

#[post("/users")]
async fn create(user: web::Json<UserMessage>) -> Result<HttpResponse, ApiError> {
    let user = User::create(user.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

#[put("/users/{id}")]
async fn update(id: web::Path<Uuid>, user: web::Json<UserMessage>) -> Result<HttpResponse, ApiError> {
    let user = User::update(id.into_inner(), user.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

#[delete("/users/{id}")]
async fn delete(id: web::Path<Uuid>) -> Result<HttpResponse, ApiError> {
    let num_deleted = User::delete(id.into_inner())?;
    Ok(HttpResponse::Ok().json(json!({ "deleted": num_deleted })))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);
}
```

现在我们只需要在主文件中添加 db、schema 和 api-error 模型。 我还强烈建议初始化数据库，尽管这并非绝对必要。 我们使用延迟静态来处理数据库池。 所以如果我们不正确地启动它，它在被使用之前就不会被启动。 

```
// src/main.rs
#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use actix_web::{App, HttpServer};
use dotenv::dotenv;
use listenfd::ListenFd;
use std::env;

mod api_error;
mod db;
mod schema;
mod user;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    db::init();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| 
        App::new()
            .configure(user::init_routes)
    );

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("Host not set");
            let port = env::var("PORT").expect("Port not set");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await
}
```

我们现在应该能够通过接口创建、读取、更新和删除用户。接下来，我计划展示如何对用户进行身份验证。