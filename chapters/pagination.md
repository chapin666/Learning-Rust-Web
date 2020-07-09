## 分页

在本教程中，我们将为 API 结果构建分页。为了解决这个问题，我们需要连接到 Diesel 的查询构建器，所以我还将介绍如何做到这一点。本教程基于我的[构建 REST API](https://cloudmaker.dev/how-to-create-a-rest-api-in-rust) 的教程，因此我强烈推荐首先阅读这个教程，或者从 [github](https://github.com/thecloudmaker/actix_tutorials/tree/master/rest_api) 中克隆代码。

### 扩展查询生成器

因为我们要扩展我们的数据库 API，所以为它建立一个单独的文件夹是有意义的。那么让我们重命名并移动 src/db.rs 文件到 src/db/connection.rs。 我们还需要记住创建一个 mod 文件来继续公开模块之外的方法。

```
mod connection;
mod paginate;

pub use connection::*;
pub use paginate::*;
```

分页模块当然是用于我们的分页，我们将继续这个过程。由于 diesel 不支持开箱即用的分页，我们必须自己扩展查询生成器。

为了简单起见，我们将使用偏移分页，尽管这对于较大的数据集来说并不是最有效的。但是希望您能够使用您在本教程中学到的知识，通过查询分页来扩展 Diesel 的查询生成器，以适应您的用例。我们要执行的查询应该能够限制获得的条目数量，并计算总条目数量。我们可以这样查询:

```
SELECT *, COUNT(*) OVER () FROM (subselect t) LIMIT $1 OFFSET $2
```

要使用这个查询扩展查询构建器，我们需要创建一个实现 QueryFragment trait 的结构。实现 QueryFragment 的结构还需要实现 QueryId，我们可以使用 derive 属性实现这个结构。

Struct 表示一个可执行的查询，因此我们还将实现 RunQueryDsl，它将添加诸如 execute 和 load 之类的函数。该查询还有一个返回类型，我们可以通过实现 Query trait 来声明这个类型。

```
use diesel::prelude::*;
use diesel::pg::Pg;
use diesel::query_builder::*;
use diesel::sql_types::BigInt;

const DEFAULT_PAGE_SIZE: i64 = 10;

#[derive(QueryId)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    page_size: i64,
}

pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            page_size: DEFAULT_PAGE_SIZE,
            page,
        }
    }
}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.page_size)?;
        out.push_sql(" OFFSET ");
        let offset = (self.page - 1) * self.page_size;
        out.push_bind_param::<BigInt, _>(&offset)?;
        Ok(())
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}
```

现在我们可以对查询使用 paginate 函数并将它们加载到 Vec<(T, i64)> 中。让我们在用户 API 中尝试一下。

```
// src/user/model.rs
use crate::db::Paginate;
//..

#[derive(Debug, Deserialize)]
pub struct Params {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    // ..
}

impl User {
    pub fn find_all(params: Params) -> Result<(Vec<Self>, i64), ApiError> {
        let conn = db::connection()?;
        let mut query = user::table.into_boxed();
        // ..

        let (users, total_pages) = match params.page {
            Some(page) => {
                let res = query.paginate(page).load::<(User, i64)>(&conn)?;

                let total = res.get(0).map(|x| x.1).unwrap_or(0);
                let users = res.into_iter().map(|x| x.0).collect();
                let total_pages = (total as f64 / 10 as f64).ceil() as i64;
                
                (users, total_pages)
            },
            None => (query.load(&conn)?, 1),
        };
        
        Ok((users, total_pages))
    }
    // ..
```

现在，我们只需要将总页数传递回该路由，然后再尝试即可。

```
// src/user/routes.rs
// ..
#[get("/users")]
async fn find_all(filters: web::Query<Params>) -> Result<HttpResponse, ApiError> {
    let (users, total_pages) = User::find_all(filters.into_inner())?;
    Ok(HttpResponse::Ok().json(json!({"users": users, "total_pages": total_pages})))
}
// ..

```
现在我们应该可以使用 page 参数测试接口了，但是您可能注意到我们仍然不能更改页面大小。如果我们在每次添加分页时不必编写所有这些样板代码，那不是更好吗。我们可以添加另一个 trait 和一些函数来处理它。

```
// src/db/paginate.rs
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::HasSqlType;
// ..
impl<T> Paginated<T> {
    pub fn page_size(self, page_size: i64) -> Self {
        Paginated { page_size, ..self }
    }

    pub fn load_and_count_pages<U>(self, conn: &PgConnection) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<PgConnection, (U, i64)>,
    {
        let page_size = self.page_size;
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.get(0).map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total as f64 / page_size as f64).ceil() as i64;
        Ok((records, total_pages))
    }
}

pub trait LoadPaginated<U>: Query + QueryId + QueryFragment<Pg> + LoadQuery<PgConnection, U> {
    fn load_with_pagination(self, conn: &PgConnection, page: Option<i64>, page_size: Option<i64>) -> QueryResult<(Vec<U>, i64)>;
}

impl<T, U> LoadPaginated<U> for T
where
    Self: Query + QueryId + QueryFragment<Pg> + LoadQuery<PgConnection, U>,
    U: Queryable<Self::SqlType, Pg>,
    Pg: HasSqlType<Self::SqlType>,
{
    fn load_with_pagination(self, conn: &PgConnection, page: Option<i64>, page_size: Option<i64>) -> QueryResult<(Vec<U>, i64)> {
        let (records, total_pages) = match page {
            Some(page) => {
                let mut query = self.paginate(page);
                if let Some(page_size) = page_size {
                    query = query.page_size(page_size);
                }

                query.load_and_count_pages::<U>(conn)?
            },
            None => (self.load::<U>(conn)?, 1),
        };

        Ok((records, total_pages))
    }
}

```

现在，使用 LoadPaginated trait 添加分页应该会容易一些，这也允许我们添加页面大小的参数。

```
// src/user/model.rs
use crate::db::LoadPaginated;
// ..
impl User {
    pub fn find_all(params: Params) -> Result<(Vec<Self>, i64), ApiError> {
        let conn = db::connection()?;
        let mut query = user::table.into_boxed();
        // ..
        
        let (users, total_pages) = query
            .load_with_pagination(&conn, params.page, params.page_size)?;
             
        Ok((users, total_pages))
    }
    // ..
```

现在，我们也应该能够使用带有页面大小参数的 API 了。如果您需要它，您可以在 github 上找到完整的代码。

### 下一步

在前面关于排序和过滤的文章中，我们做了一些混乱和相当重复的代码。因此，在下一个教程中，我将展示如何使用宏来清理代码并避免不必要的重复。



