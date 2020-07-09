## 缓存

如果您的 web 应用程序变得非常流行，那么您可能会发现自己正处于数据库的极限状态。您可以随时尝试扩展数据库，以及随之而来的所有复杂性。但是，在这条道路上行驶之前，看看是否有可能减轻负荷也是有意义的。

如果你正在制作一个天气应用程序，你可能会收到很多像柏林或伦敦这样的大城市的请求。这样就不需要一遍又一遍地请求数据库提供相同的数据，因为它不会在每次请求之间更改。相反，我们可以将频繁请求的数据存储在缓存中，将负载从数据库转移到缓存中。

对于本教程，我们将使用 Redis 作为缓存，因此您应该在计算机上安装 Redis。我还假设您知道如何[创建一个 rest api](https://cloudmaker.dev/how-to-create-a-rest-api-in-rust/)，因为我将在本教程中使用它作为入门。

现在让我们开始在 Cargo.toml 中添加 Redis 作为一个依赖项。

```
[dependencies]
redis = { version = "0.15", features = ["r2d2"] }
```

我们还需要将 Redis URL 添加到环境变量中。

```
# .env
REDIS_URL=redis://localhost
```

接下来，我们将为 Redis 建立一个连接池，就像我们为数据库所做的那样。

```
// src/cache.rs
use crate::api_error::ApiError;
use lazy_static::lazy_static;
use r2d2;
use redis::{Client, ConnectionLike};
use std::env;

type Pool = r2d2::Pool<Client>;
pub type CacheConnection = r2d2::PooledConnection<Client>;

lazy_static! {
    static ref POOL: Pool = {
        let redis_url = env::var("REDIS_URL").expect("Redis url not set");
        let client = redis::Client::open(redis_url).expect("Failed to create redis client");
        Pool::new(client).expect("Failed to create redis pool")
    };
}

pub fn init() {
    info!("Initializing Cache");
    lazy_static::initialize(&POOL);
    let mut conn = connection().expect("Failed to get redis connection");
    assert_eq!(true, conn.check_connection(), "Redis connection check failed");
}

pub fn connection() -> Result<CacheConnection, ApiError> {
    POOL.get()
        .map_err(|e| ApiError::new(500, format!("Failed getting db connection: {}", e)))
}
```

我们还必须先初始化缓存，然后才能在我们的API中使用它。

```
// src/main.rs
// ..
mod cache;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    db::init();
    cache::init();
    // ..
```

为了方便起见，让我们为 ApiError 实现 From<RedisError> ，这样我们就可以用 ? 来处理这些错误。

```
// src/api_error.rs
use redis::RedisError;

impl From<RedisError> for ApiError {
    fn from(error: RedisError) -> ApiError {
        ApiError::new(500, format!("Redis error: {}", error))
    }
}
```

现在我们已经设置了缓存，可以将它用于我们的用户 API。

```
// src/user/model.rs
use crate::cache;
use redis::Commands;

impl User {
    pub fn find(id: Uuid) -> Result<Self, ApiError> {
        if let Some(user) = User::cache_find(id)? {
            return Ok(user);
        }

        let conn = db::connection()?;
        let user = user::table
            .filter(user::id.eq(id))
            .first::<User>(&conn)?;

        user.cache_set()?;

        Ok(user)
    }

    pub fn update(id: Uuid, user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;
        let user = diesel::update(user::table)
            .filter(user::id.eq(id))
            .set(user)
            .get_result::<User>(&conn)?;

        user.cache_set()?;

        Ok(user)
    }

    pub fn delete(id: Uuid) -> Result<usize, ApiError> {
        let conn = db::connection()?;
        let res = diesel::delete(
                user::table
                    .filter(user::id.eq(id))
            )
            .execute(&conn)?;

        User::cache_delete(id)?;

        Ok(res)
    }

    fn cache_find(id: Uuid) -> Result<Option<Self>, ApiError> {
        let cache_key = format!("user.{}", id);
        let mut cache = cache::connection()?;
        let res: Vec<u8> = cache.get(&cache_key)?;
        match serde_json::from_slice::<User>(&res).ok() {
            Some(user) => Ok(Some(user)),
            None => Ok(None),
        }
    }

    fn cache_set(&self) -> Result<(), ApiError> {
        let cache_key = format!("user.{}", self.id);
        let mut cache = cache::connection()?;
        if let Some(cache_user) = serde_json::to_vec(self).ok() {
            let _: () = cache.set_ex(&cache_key, cache_user, 3600)?;
        }
        Ok(())
    }

    fn cache_delete(id: Uuid) -> Result<(), ApiError> {
        let cache_key = format!("user.{}", id);
        let mut cache = cache::connection()?;
        let _: () = cache.del(cache_key)?;
        Ok(())
    }
}
```

我们用于查找单个用户的 API 现在会首先检查缓存，如果用户已经在那里，并且只会在用户在缓存中没有找到的情况下询问数据库。

我们还需要确保在更新和删除时使缓存无效，因为更新和删除将使缓存中的数据无效。

我们还为缓存中的所有条目设置了超时时间为一小时，这样如果数据不经常使用，就不会在缓存中停留太长时间。

您还可以在 github 上找到本教程的[完整代码](https://github.com/thecloudmaker/actix_tutorials/tree/master/caching)。






