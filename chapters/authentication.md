## 身份认证

在本教程中，我们将为上一个教程中创建的 [REST API](/chapters/rest-api.md) 创建身份验证，因此我强烈推荐首先阅读这个教程。 我们将使用 Redis 来处理我们的会话，所以您的计算机上应该安装 Redis。

### Password hashing

永远不要在未加密的数据库中存储真实用户密码。 那么让我们从加密密码开始。 对于哈希密码，我们将使用 ```Argon2```。 ```Argon2``` 是众所周知的安全哈希算法，甚至赢得了密码[哈希竞赛(PHC)](https://password-hashing.net/)。 我们还需要一个随机生成器来生成 salt。 因此，让我们将依赖项添加到 Cargo.toml 中。

```
[dependencies]
rand = "0.7"
rust-argon2 = "0.5"
```

我们要散列密码作为我们的用户创建 API 的一部分，我们可以创建一个函数来验证密码。

```
// src/user/model.rs
use argon2::Config;
use rand::Rng;
// ...

impl User {
    // ..
    pub fn create(user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let mut user = User::from(user);
        user.hash_password()?;
        let user = diesel::insert_into(user::table)
            .values(user)
            .get_result(&conn)?;

        Ok(user)
    }
    // ...
    pub fn hash_password(&mut self) -> Result<(), ApiError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        self.password = argon2::hash_encoded(self.password.as_bytes(), &salt, &config)
            .map_err(|e| ApiError::new(500, format!("Failed to hash password: {}", e)))?;

        Ok(())
    }

    pub fn verify_password(&self, password: &[u8]) -> Result<bool, ApiError> {
        argon2::verify_encoded(&self.password, password)
            .map_err(|e| ApiError::new(500, format!("Failed to verify password: {}", e)))
    }
}
// ...
```

对于创建散列，您可以看到我们为每个密码使用了一个随机的32位 salt。 这是为了避免拥有相同密码的用户最终拥有相同的密码散列，这也使得攻击者更难破解密码。

将密码散列返回给用户是不好的做法。 通过跳过密码的序列化，Serde 可以非常容易地对用户隐藏密码散列。

```
// src/user/model.rs
#[derive(Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "user"]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}
```

现在，我们可以尝试用用户创建接口来创建一个新用户，我们将看到用户密码在我们的数据库中是加密的。

### 注册用户

注册用户将是一项简单的任务。 从技术上讲，我们已经有了这方面的接口。 但是，让我们把注册作为新的身份验证模块的一部分。 因此，让我们使用一个注册接口来创建 auth 模块。

```
// src/auth/routes.rs
use crate::api_error::ApiError;
use crate::user::{User, UserMessage};
use actix_web::{post, get, web, HttpResponse};

#[post("/register")]
async fn register(user: web::Json<UserMessage>) -> Result<HttpResponse, ApiError> {
    let user = User::create(user.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
}
```

我们还需要为模块之外的路由提供 init 方法。

```
// src/auth/mod.rs
mod routes;

pub use routes::init_routes;
```

然后，我们可以将新模块与应用程序的其余部分连接起来。

```
// src/main.rs
// ...
mod auth;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // ...
    let mut server = HttpServer::new(|| 
        App::new()
            .configure(auth::init_routes)
    );
    // ...
}
```

现在，我们的新注册接口的工作方式应该与用户创建端点的工作方式相同。

### 登入和退出

很多教程建议使用 json web 令牌来处理会话，这些令牌应该比在 cookie 中存储会话数据更安全。 但是你要把你的 token 放在哪里呢？ 也许放在 cookie 里？ 通过这样做，我们仍然可以成为 CSRF 攻击的目标，我听说很多人说可以通过使用 json web 标记来避免这种攻击。 另一种方法是将令牌存储在 session storage 中。 现在您可能会安全地抵御 CSRF 攻击，但是您现在可能会暴露在 XSS 攻击之下，这可能更加危险。

一种更安全的存储会话数据的方法是在服务器端的键值存储(如 Redis)中。 我们仍然可以成为 CSRF 攻击的对象，因为我们需要在 cookie 中存储会话密钥。 因此，在将应用程序部署到生产环境之前，一定要保护自己免受这类攻击。 CSRF 攻击是一个更复杂的话题，所以我将在以后的博客文章中更深入地讨论这个问题。

对于处理会话，我们将需要另外两个依赖项。

```
[dependencies]
actix-redis = { version = "0.8", features = ["web"] }
actix-session = "0.3"
```

为了让我们的应用程序知道在哪里可以找到 Redis，我们将添加另外两个环境变量。

```
REDIS_HOST=127.0.0.1
REDIS_PORT=6379
```

我们还必须设置中间件来为我们处理会话。

```
// src/main.rs
// ...
use actix_redis::RedisSession;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // ...
    let redis_port = env::var("REDIS_PORT").expect("Redis port not set");
    let redis_host = env::var("REDIS_HOST").expect("Redis host not set");

    let mut server = HttpServer::new(move|| 
        App::new()
            .wrap(RedisSession::new(format!("{}:{}", redis_host, redis_port), &[0; 32]))
            .configure(auth::init_routes)
    );
    // ...
}
```

对于登录，我们需要能够找到一个用户只知道的电子邮件。 通过电子邮件找到用户的一种方法是通过一种新的方法扩展我们的用户 API。

```
// src/user/model.rs
// ...
impl User {
    pub fn find_by_email(email: String) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = user::table
            .filter(user::email.eq(email))
            .first(&conn)?;

        Ok(user)
    }
    // ...
}
```

同样为了方便起见，我们将使用一种默认的方式来处理 activex 错误。 通过这种方式，我们可以使用问号来处理这些类型的错误，而不必显式地将每个错误映射到 ApiError。

```
// src/api_error.rs
use actix_web::error::Error as ActixError;
// ...

impl From<ActixError> for ApiError {
    fn from(error: ActixError) -> ApiError {
        ApiError::new(500, error.to_string())
    }
}
```

现在我们可以创建登录和退出的接口。

```
// src/auth/routes.rs
use actix_session::Session;
use serde_json::json;
use uuid::Uuid;
// ...

#[post("/sign-in")]
async fn sign_in(credentials: web::Json<UserMessage>, session: Session) -> Result<HttpResponse, ApiError> {
    let credentials = credentials.into_inner();

    let user = User::find_by_email(credentials.email)
        .map_err(|e| {
            match e.status_code {
                404 => ApiError::new(401, "Credentials not valid!".to_string()),
                _ => e,
            }
        })?;
  
    let is_valid = user.verify_password(credentials.password.as_bytes())?;

    if is_valid == true {
        session.set("user_id", user.id)?;
        session.renew();

        Ok(HttpResponse::Ok().json(user))
    }
    else {
        Err(ApiError::new(401, "Credentials not valid!".to_string()))
    }
}

#[post("/sign-out")]
async fn sign_out(session: Session) -> Result<HttpResponse, ApiError> {
    let id: Option<Uuid> = session.get("user_id")?;

    if let Some(_) = id {
        session.purge();
        Ok(HttpResponse::Ok().json(json!({ "message": "Successfully signed out" })))
    }
    else {
        Err(ApiError::new(401, "Unauthorized".to_string()))
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
    cfg.service(sign_in);
    cfg.service(sign_out);
}

```

如果我们现在达到了登录接口，那么如果我们已经注册了用户，那么我们应该能够看到会话键作为 cookie 的一部分。 我们也可以尝试注销，然后我们将看到 cookie 将被删除。

### 使用会话数据

现在，让我们使用会话数据获取用户信息。 因此，让我们创建一个接口，为登录用户提供用户数据。

```
// src/auth/routes.rs
// ...
#[get("/who-am-i")]
async fn who_am_i(session: Session) -> Result<HttpResponse, ApiError> {
    let id: Option<Uuid> = session.get("user_id")?;

    if let Some(id) = id {
        let user = User::find(id)?;
        Ok(HttpResponse::Ok().json(user))
    }
    else {
        Err(ApiError::new(401, "Unauthorized".to_string()))
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
    cfg.service(sign_in);
    cfg.service(sign_out);
    cfg.service(who_am_i);
}
```

现在，如果我们碰到了新创建的接口，并且登录了，我们应该得到用户数据; 如果没有登录，我们应该得到状态401。

在下一个章节，我们将学习如何通过电子邮件来验证用户。