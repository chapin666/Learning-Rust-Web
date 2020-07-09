## 排序与筛选

在本教程中，我们将允许对 API 结果进行过滤和排序。为了帮助我们解决这个问题，我们将使用 Diesel 的查询生成器来构建一个条件查询。本教程基于我的构建 [REST API](https://cloudmaker.dev/how-to-create-a-rest-api-in-rust) 的教程，因此我强烈推荐首先阅读这个教程，或者从 github 中克隆代码。

我们首先为允许过滤和排序的参数创建一个结构。然后，在构建条件查询之前，我们将过滤器传递给 API 以寻找用户。

```
// src/user/model.rs
//..
#[derive(Debug, Deserialize)]
pub struct Params {
    pub email: Option<String>,
    pub sort_by: Option<String>,
    #[serde(rename = "created_at[gte]")]
    pub created_at_gte: Option<NaiveDateTime>,
    #[serde(rename = "created_at[lte]")]
    pub created_at_lte: Option<NaiveDateTime>,
    #[serde(rename = "updated_at[gte]")]
    pub updated_at_gte: Option<NaiveDateTime>,
    #[serde(rename = "updated_at[lte]")]
    pub updated_at_lte: Option<NaiveDateTime>,
}

impl User {
    pub fn find_all(params: Params) -> Result<Vec<Self>, ApiError> {
        let conn = db::connection()?;

        let mut query = user::table.into_boxed();

        if let Some(email) = params.email {
            query = query.filter(user::email.like(email));
        }
        if let Some(created_at_gte) = params.created_at_gte {
            query = query.filter(user::created_at.ge(created_at_gte));
        }
        if let Some(created_at_lte) = params.created_at_lte {
            query = query.filter(user::created_at.le(created_at_lte));
        }
        if let Some(updated_at_gte) = params.updated_at_gte {
            query = query.filter(user::updated_at.ge(updated_at_gte));
        }
        if let Some(updated_at_lte) = params.updated_at_lte {
            query = query.filter(user::updated_at.le(updated_at_lte));
        }
        if let Some(sort_by) = params.sort_by {
            query = match sort_by.as_ref() {
                "id" => query.order(user::id.asc()),
                "id.asc" => query.order(user::id.asc()),
                "id.desc" => query.order(user::id.desc()),
                "email" => query.order(user::email.asc()),
                "email.asc" => query.order(user::email.asc()),
                "email.desc" => query.order(user::email.desc()),
                "created_at" => query.order(user::created_at.asc()),
                "created_at.asc" => query.order(user::created_at.asc()),
                "created_at.desc" => query.order(user::created_at.desc()),
                "updated_at" => query.order(user::updated_at.asc()),
                "updated_at.asc" => query.order(user::updated_at.asc()),
                "updated_at.desc" => query.order(user::updated_at.desc()),
                _ => query,
            };
        }

        let users = query
            .load::<User>(&conn)?;

        Ok(users)
    }
    //..
}
```

请注意，创建初始查询时，我们使用的是 .into_boxed() 。 这用于将查询打包为单一类型，以便编译器在构建条件查询时知道如何处理。

您可能还注意到，我们必须非常明确地说明 API 的行为。对于每一个单独的参数，我们必须定义预期的行为，并且我们还必须定义每一个可能的排序参数。这样做的好处是，我们知道从 API 中期望得到什么，并且最终不会出现任何令人惊讶的行为。

如果您和我观点一样，您可能会认为这种代码有点混乱。 我觉得我必须在编写多个 API 时一遍又一遍地编写这种行为，以免自己重复太多。 为了解决这个问题，我们可以使用宏，我将在下一篇教程中介绍它。 接下来，我们需要从请求中获取参数，并将其传递给我们的API。

```
// src/user/routes.rs
//..
use crate::user::{User, UserMessage, Params};

#[get("/users")]
async fn find_all(params: web::Query<Params>) -> Result<HttpResponse, ApiError> {
    let users = User::find_all(params.into_inner())?;
    Ok(HttpResponse::Ok().json(users))
}
//..
```

现在我们可以尝试一下我们的 API。下面是一些示例请求，您可以尝试，也可以组合这些参数。

```
$ curl 'http://localhost:5000/users?email=john%'
$ curl 'http://localhost:5000/users?created_at[lte]=2019-12-11T00:00:00'
$ curl 'http://localhost:5000/users?sort_by=created_at.desc'
```

本教程的完整源代码可以在 [github](https://github.com/thecloudmaker/actix_tutorials/tree/master/filtering_and_sorting) 上找到，以备不时之需。

### 下一节

接下来，我将制作一个教程，介绍如何连接Diesel的查询构建器以扩展API的分页功能

