pub mod handlers;
pub mod models;

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::reject::Rejection;
use warp::ws::Message;
use warp::Filter;

// use crate::handlers::auth::get::get_users;
use crate::handlers::messaging::connect::connect;
use crate::handlers::misc::health_check::health_check_;
use crate::handlers::misc::not_found::not_found;

// Usize -> User ID
// String -> Session ID
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

// Type for the auth routes
type WebResult<T> = std::result::Result<T, Rejection>;

// TODO: .unwrap_or_else(|| "0.0.0.0:3000".to_string());
#[tokio::main]
async fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let socket_address: SocketAddr = addr.parse().expect("valid socket Address");

    let users = Users::default();
    let users = warp::any().map(move || users.clone());

    let users_to_sessions = UserSessions::default();
    let users_to_sessions = warp::any().map(move || users_to_sessions.clone());

    let sessions = Sessions::default();
    let sessions = warp::any().map(move || sessions.clone());

    let api = warp::path!("api" / "v1");

    // Health check
    let health_check = warp::path!("api" / "v1" / "health_check");
    health_check.map(|| health_check_);

    // let auth_router = api.and(warp::path!("users"));
    // let auth_router_id = auth_router.and(warp::path!(String));

    // The auth routes
    // let get_users = auth_router
    //     .and(warp::get())
    //     .and(warp::path::end())
    //     .and_then(get_users);

    // GET /ws
    let chat = warp::path!("ws")
        // Passing all the references to the shared state.
        .and(warp::ws())
        .and(users)
        .and(sessions)
        .and(users_to_sessions)
        .map(|ws: warp::ws::Ws, users, sessions, users_to_sessions| {
            ws.on_upgrade(move |socket| connect(socket, users, sessions, users_to_sessions))
        });

    // "../var/www/static/404.html"
    let res_404 = warp::any().map(|| not_found);
    api.and(chat).and(health_check);

    let routes = api.or(res_404);
    let server = warp::serve(routes).try_bind(socket_address);
    println!("Running server at {}!", addr);

    server.await
}
