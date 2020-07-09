
#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use actix_web::{App, HttpServer};
use listenfd::ListenFd;
use dotenv::dotenv;
use std::env;
use actix_redis::RedisSession;

mod api_error;
mod db;
mod schema;
mod user;
mod auth;
mod email;
mod email_verification_token;


#[actix_rt::main]
async fn main() -> std::io::Result<()> {

    dotenv().ok();
    env_logger::init();

    db::init();


    let redis_port = env::var("REDIS_PORT").expect("Redis port not set");
    let redis_host = env::var("REDIS_HOST").expect("Redis host not set");


    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(RedisSession::new(format!("{}:{}", redis_host, redis_port), &[0; 32]))
            .configure(user::init_routes)
            .configure(auth::init_routes)
    });

    // 获取监听器
    // 如果有，则坚挺
    // 否则新创建
    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("Host not set");
            let port = env::var("PORT").expect("Port not set");
            server.bind(format!("{}:{}", host, port))?
        },
    };

    
    info!("Starting server");
    server.run().await
}
