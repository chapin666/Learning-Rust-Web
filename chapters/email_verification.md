
## 邮件确认

我们将用电子邮件确认来验证 API 用户。 本教程基于前面两个关于创建 REST API 和身份验证的教程。

在本教程中，我们将使用 [Sendinblue](https://www.sendinblue.com/)，我建议立即激活该帐户，因为事务性电子邮件需要手动激活。 不过不要担心，当你注册了，你只需要发送一个简短的电子邮件，让他们知道你想激活交易电子邮件。 我只需要等待两个半小时，我发现这对于一个我甚至不付费的服务来说是相当快的。


### 为电子邮件验证令牌创建一个 Model

我们将开始为电子邮件验证令牌创建一个新的迁移。 我们当然会用 diesel-cli 。

```
diesel migration generate email_verification_token
```

在生成的两个文件中，我们添加了迁移脚本。

```
// up.sql
CREATE TABLE email_verification_token (
    id BYTEA PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);    
```

```
// down.sql
DROP TABLE email_verification_token;
```

现在我们已经创建了迁移，我们可以使用 ```diesel migration run``` 命令执行它。

接下来我们要创建一个模型，以便我们在 Rust 中有一个 email 令牌的表示。 我们还将实现查找、创建和删除令牌的方法。

```
// src/email_verification_token/model.rs
use crate::api_error::ApiError;
use crate::db;
use crate::schema::email_verification_token;
use chrono::{NaiveDateTime, Utc, Duration};
use diesel::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct EmailVerificationTokenMessage {
    pub id: Option<String>,
    pub email: String,
}

#[derive(Deserialize, Serialize, Queryable, Insertable)]
#[table_name = "email_verification_token"]
pub struct EmailVerificationToken {
    pub id: Vec<u8>,
    pub email: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

impl EmailVerificationToken {
    pub fn find(id: &Vec<u8>) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let token = email_verification_token::table
            .filter(email_verification_token::id.eq(id))
            .first(&conn)?;

        Ok(token)
    }

    pub fn create(body: EmailVerificationTokenMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let id = rand::thread_rng().gen::<[u8; 32]>().to_vec();
        let email = body.email;
        let created_at = Utc::now().naive_utc();
        let expires_at = created_at + Duration::hours(12);
        let token = EmailVerificationToken { id, email, expires_at, created_at };

        let token = diesel::insert_into(email_verification_token::table)
            .values(&token)
            .on_conflict(email_verification_token::email)
            .do_update()
            .set((
                email_verification_token::id.eq(&token.id),
                email_verification_token::created_at.eq(&token.created_at),
                email_verification_token::expires_at.eq(&token.expires_at),
            ))
            .get_result(&conn)?;

        Ok(token)
    }

    pub fn delete(id: &Vec<u8>) -> Result<usize, ApiError> {
        let conn = db::connection()?;

        let res = diesel::delete(
                email_verification_token::table
                    .filter(email_verification_token::id.eq(id))
            )
            .execute(&conn)?;

        Ok(res)
    }
}
```

我们不希望每封邮件都有几个 token，所以我们使用 on_conflict() 处理这些冲突，并在创建新 token 时使用该 token 覆盖旧 token。 通过使用这种方法，我们还可以让用户创建一个新的 token，以防他错误地删除了确认邮件或者等了太长时间才激活用户。

### 简短的泛型示例

为了保持简单，我一直避免使用泛型，但是对于我们的电子邮件 API，我们真的可以使我们更容易一点使用它。 泛型允许我们为同一个参数创建可以接受多个类型的函数，只要类型具有我们需要的 trait。

我们可以以 ApiError::new() 方法为例。 你可能已经注意到我们已经写了很多。 在我们的 ApiError::new() 函数中使用 String () ，因为消息必须是 String。 但是，如果我们让消息是泛型类型呢？ 我们并不真的在乎，只要我们能把消息变成字符串，不是吗？

```
// src/api_error.rs
impl ApiError {
    pub fn new<T: Into<String>>(status_code: u16, message: T) -> ApiError {
        ApiError { status_code, message: message.into() }
    }
}
```

这不会破坏我们的代码，因为 String 也实现了 Into<String>。 但是现在如果你移除了。 在 ApiError::new() 修正方法中使用 String() 方法，这样我们将给出一个 String literal 作为参数，而不是 String。 它似乎仍然可以工作，因为 &str 也实现了 Into<String>。

### 电子邮件 API

为了调用 Sendinblue API，我们需要能够进行 http 调用。 为此，我们将使用 reqwest。 我们还需要一种方法来解码和编码我们的令牌到一些东西，可以通过电子邮件发送和传回的 json 请求。 为此，我们可以使用十六进制箱，它将帮助我们将一块字节转换为十六进制。 因此，让我们添加这些依赖项。

```
[dependencies]
hex = "0.4"
reqwest = "0.9"
```

现在已经安装了依赖项，我们可以继续创建我们的电子邮件发送 API。

```
// src/email/api.rs
use crate::api_error::ApiError;
use serde::Serialize;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref SENDINBLUE_API_KEY: String = std::env::var("SENDINBLUE_API_KEY").unwrap_or("".to_string());
}

#[derive(Debug, Serialize)]
pub struct Contact {
    email: String,
    name: Option<String>,
}

impl Contact {
    pub fn new<T: Into<String>>(email: T, name: T) -> Self {
        Contact { email: email.into(), name: Some(name.into()) }
    }
}

impl<T: Into<String>> From<T> for Contact {
    fn from(email: T) -> Self {
        Contact { email: email.into(), name: None }
    }
}

#[derive(Debug, Serialize)]
pub struct Email {
    sender: Contact,
    #[serde(rename = "to")]
    recipients: Vec<Contact>,
    subject: String,
    #[serde(rename = "htmlContent")]
    html: Option<String>
}

impl Email {
    pub fn new(sender: Contact) -> Self {
        Email {
            sender,
            recipients: Vec::new(),
            subject: "".to_string(),
            html: None,
        }
    }

    pub fn add_recipient<T: Into<Contact>>(mut self, recipient: T) -> Self {
        self.recipients.push(recipient.into());
        self
    }

    pub fn set_subject<T: Into<String>>(mut self, subject: T) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn set_html<T: Into<String>>(mut self, html: T) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn send(self) -> Result<String, ApiError> {
        let client = reqwest::Client::new();
        let mut response = client.post("https://api.sendinblue.com/v3/smtp/email")
            .header("Accept", "application/json")
            .header("api-key", SENDINBLUE_API_KEY.as_str())
            .json(&self)
            .send()
            .map_err(|e| ApiError::new(500, format!("Failed to send email: {}", e)))?;

        let status = response.status().as_u16();
        let mut body: HashMap<String, String> = response
            .json()
            .map_err(|e| ApiError::new(500, format!("Failed to read sendinblue response: {}", e)))?;

        match status {
            201 => Ok(body.remove("messageId").unwrap_or("".to_string())),
            _ => {
                let message = body.remove("message").unwrap_or("Unknown error".to_string());
                Err(ApiError::new(500, format!("Failed to send email: {}", message)))
            }
        }
    }
}
```
如果你是 Rust 的新手，我猜你可能会在这一行上有些磕磕绊绊: ```impl<T: Into<String>> From<T> for Contact```。它并不比我们上一个例子复杂多少，但是让我们把它分解成几个部分，以便更容易理解。

如果我们这样写: impl From <String> for Contact。这种希望似乎有点耳熟。这里我们实现了 Contact 的 From trait，这样我们就可以很容易地将 String 转换为 Contact。但是我们也想对 &str 做同样的事情。因此，为了避免重复，我们定义一个泛型 <T: Into<String>>，我们可以使用它来代替 String。现在我们可以确定，无论我们得到的是什么类型，我们都可以使用 .into() 方法。

如果您查看一下 Sendinblue API 文档，就会发现它看起来并不完全像我们的模型。Sendinblue API 表示他们希望收件人在 to 字段中，HTML 在 htmlContent 字段中。Serde 很容易让我们用 #[serde(rename = “name”)] 属性来改变它。

最后，我们将调用 Sendinblue API 发送我们的电子邮件。如果成功，我们将把消息 id 返回给调用者。

### 邀请和注册节点

现在我们有了用于发送电子邮件和创建令牌的 API，我们可以创建用于发送确认电子邮件和注册用户的接口。

```
// src/auth/routes.rs
// ..
use crate::email::{Email, Contact};
use crate::email_verification_token::{EmailVerificationToken, EmailVerificationTokenMessage};
use chrono::Utc;
use hex;
use serde::Deserialize;

#[post("/invite")]
async fn invite(body: web::Json<EmailVerificationTokenMessage>) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let token = EmailVerificationToken::create(body.clone())?;
    let token_string = hex::encode(token.id);

    Email::new(Contact::new("tore@cloudmaker.dev", "Cloudmaker"))
        .add_recipient(body.email)
        .set_subject("Confirm your email")
        .set_html(format!("Your confirmation code is: {}", &token_string))
        .send()?;

    Ok(HttpResponse::Ok().json(json!({"message": "Verification email sent"})))
}

#[derive(Deserialize)]
struct RegistrationMessage {
    token: String,
    email: String,
    password: String,
}

#[post("/register")]
async fn register(body: web::Json<RegistrationMessage>) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let token_id = hex::decode(body.token)
        .map_err(|e| ApiError::new(403, "Invalid token"))?;
    
    let token = EmailVerificationToken::find(&token_id)
        .map_err(|e| {
            match e.status_code {
                404 => ApiError::new(403, "Invalid token"),
                _ => e,
            }
        })?;

    if token.email != body.email {
        return Err(ApiError::new(403, "Invalid token"));
    }

    if token.expires_at < Utc::now().naive_utc() {
        return Err(ApiError::new(403, "Token expired"));
    }
 
    let user = User::create(UserMessage { email: body.email, password: body.password })?;

    Ok(HttpResponse::Ok().json(json!({"message": "Successfully registered", "user": user})))
}

// ..

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(invite);
    cfg.service(register);
    // ..
}
```

对于我们的邀请接口，我们只是创建一个令牌，对其进行编码并通过电子邮件发送到。充其量，我们应该有一个前端，我们可以导向用户，所以他可以只点击一个链接，而不是必须复制和粘贴激活令牌。

我现在不会创建一个前端，因为我仍然有更多的主题，我首先想涵盖 Rust。但是将来如果有兴趣的话，我可能会为我们的应用程序创建一个前端。在这种情况下，我可能会使用 elm，因为它旨在可靠，快速和容易重构，就像 Rust。

对于注册接口，我们只是搜索令牌并验证它是否匹配电子邮件，以及它是否过期。为了不向攻击者透露太多信息，我们只需让用户知道，如果不是所有内容都正确，那么这个令牌就是无效的。唯一的例外情况是令牌过期了，因为这对用户来说是有用的信息。

现在让我们来试一试。首先使用邀请接口向自己发送确认邮件。然后用你在电子邮件中收到的令牌注册。

![](/covers/registered.png)

本教程的完整代码示例可以在 [github](https://github.com/thecloudmaker/actix_tutorials/tree/master/email_verification) 上找到。

### 下一步？

既然您知道了如何在 Rust 中创建 REST API，那么您可能会开发出一个获得大量用户的应用程序。我们查询所有用户并他们返回。因此，下一个教程将介绍如何通过过滤缩小结果范围，并允许对结果进行排序。
