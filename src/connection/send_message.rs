use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

// Usize -> User ID
// String -> Session ID
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

pub async fn send_message(
    msg: Message,
    user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
) {
    if msg.to_str().is_ok() && user.send(Ok(msg.clone())).is_err() {}
}
