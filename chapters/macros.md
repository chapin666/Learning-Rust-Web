
## 应用 Macros

在本教程中，我们将创建几个宏，以避免总是重复的工作，并使我们的代码更加清晰。我们将要清理的代码来自于[排序和筛选的教程](https://cloudmaker.dev/sorting-and-filtering-with-diesel/)，所以我强烈建议先读取这些代码，或者从 [github 克隆这些代码](https://github.com/thecloudmaker/actix_tutorials/tree/master/filtering_and_sorting)。

### 用于排序的宏

我们将创建用于帮助我们完成排序和筛选等数据库功能的宏。我们将首先在 db 文件夹中创建一个新文件，然后创建第一个宏来帮助我们进行排序。

```
// src/db/macros.rs
#[macro_export]
macro_rules! sort_by {
   ($query:expr, $sort_by:expr, $(($param:expr, $column:expr)),*) => {
       {
           if let Some(sort_by) = $sort_by {
               $query = match sort_by.as_ref() {
                   $(
                       $param => $query.order($column.asc()),
                       concat!($param, ".asc") => $query.order($column.asc()),
                       concat!($param, ".desc") => $query.order($column.desc()),
                   )*
                   _ => $query,
               }
           }
           $query
       }
   };
}
```

这可能有很多新的语法，但我不会深入讨论细节，因为[官方文档](https://doc.rust-lang.org/1.7.0/book/macros.html)已经很好地解释了这一点。但简而言之，我们正在定义一个需要查询的宏以及用于排序的参数。此外，我们还需要定义允许参数存在的值以及该值引用的列。然后，我们只需使用这些信息来重新生成一个更通用的查询。

在继续之前，不要忘记将宏模块添加到 db/mod.rs 文件中:

```
// src/db/mod.rs
mod connection;
mod paginate;
mod macros;

pub use connection::*;
pub use paginate::*;
```

现在让我们替换现有的排序实现，使用我们的新宏:

```
// src/user/model.rs
use crate::sort_by;
// ..

impl User {
   pub fn find_all(params: Params) -> Result<(Vec<Self>, i64), ApiError> {
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

       query = sort_by!(query, params.sort_by,
           ("id", user::id),
           ("email", user::email),
           ("created_at", user::created_at),
           ("updated_at", user::updated_at)
       );
      
       let (users, total_pages) = query
           .load_with_pagination(&conn, params.page, params.page_size)?;
      
       Ok((users, total_pages))
   }
   // ..

```

对于排序，我们现在从17行下降到6行。这是一个相当大的进步，并使它更容易阅读。我们允许按越多的值进行排序，我们将保存越多的行，因为我们现在只需要为每个额外的值添加一个新行，而不是没有宏的树。还要注意，当我们将宏添加到名称空间时，它被放置在 crate 的根目录而不是数据库模块中。

### 用于筛选的宏

用于筛选的宏将会更复杂一些，因为我们需要有一个选项来选择是否要使用 gt、 le 或者 like 来比较值。但是我们也可以用一点创造力来做到这一点。

```
// src/db/macros.rs
#[macro_export]
macro_rules! filter {
   ($query:expr, $(($column:expr, @$expression_method:ident, $param:expr)),*) => {
       {
           $(
               if let Some(item) = $param {
                   let filter = filter!($column, @$expression_method, item);
                   $query = $query.filter(filter);
               }
           )*
           $query
       }
   };
   ($column:expr, @like, $item:expr) => { $column.like($item) };
   ($column:expr, @ge, $item:expr) => { $column.ge($item) };
   ($column:expr, @le, $item:expr) => { $column.le($item) };
}
```

与上一个宏一样，我们还必须将查询传递到宏中以进行筛选。 对于我们想要的每个筛选器，我们还需要传递列，将要使用的表达式方法以及要与之比较的参数。 您可能会注意到，这个新宏有多个分支。 它的工作原理与match语句有点相似，只是对于宏，我们正在比较完整的语句而不是单个变量。

此外，我们还创建了自己的关键字@like,@ge 和@le，以区分不同的表达方法。我们使用@作为前缀的原因是它不用于前缀的位置，这意味着它不会与任何东西发生冲突。

现在让我们使用新的宏。

```
use crate::{filter, sort_by};
// ..
impl User {
   pub fn find_all(params: Params) -> Result<(Vec<Self>, i64), ApiError> {
       let conn = db::connection()?;

       let mut query = user::table.into_boxed();

       query = filter!(query,
           (user::email, @like, params.email),
           (user::created_at, @ge, params.created_at_gte),
           (user::created_at, @le, params.created_at_lte),
           (user::updated_at, @ge, params.updated_at_gte),
           (user::updated_at, @le, params.updated_at_lte)
       );

       query = sort_by!(query, params.sort_by,
           ("id", user::id),
           ("email", user::email),
           ("created_at", user::created_at),
           ("updated_at", user::updated_at)
       );

       let (users, total_pages) = query
           .load_with_pagination(&conn, params.page, params.page_size)?;
      
       Ok((users, total_pages))
   }
   // ..
```

在这种情况下，我们可以将筛选从15行减少到7行。但我觉得更大的优势在于可读性。我们的自定义关键字使得查看每个列使用哪个表达式以及与哪个参数进行比较变得非常容易。

所以希望现在一切都能像以前一样工作，但是提高可读性总是一个优点。

和往常一样，您将能够在 [github](https://github.com/thecloudmaker/actix_tutorials/tree/master/macros) 上找到完整的代码。

