
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

mod api_error;
mod db;
mod schema;
mod user;


#[cfg(test)]
mod test;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {

    dotenv().ok();
    env_logger::init();

    db::init();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(move || {
        App::new()
            .configure(user::init_routes)
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
