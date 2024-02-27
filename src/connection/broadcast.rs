use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

// Usize -> User ID
// String -> Session ID
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

pub async fn broadcast_msg(
    msg: Message,
    sessions: &Sessions,
    my_session: String,
    my_user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
) {
    if msg.to_str().is_ok() {
        if let Some(recipients) = sessions.read().await.get(&my_session) {
            for tx in recipients {
                if !tx.same_channel(my_user) && tx.send(Ok(msg.clone())).is_err() {
                    tx.closed().await;
                }
            }
        }
    }
}
