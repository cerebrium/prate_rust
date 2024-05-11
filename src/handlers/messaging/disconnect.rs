use async_recursion::async_recursion;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

// Usize -> User ID
// String -> Session ID
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

#[async_recursion]
pub async fn disconnect(
    session_id: String,
    my_id: usize,
    my_user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
    users: &Users,
    sessions: &Sessions,
    user_sessions: &UserSessions,
) {
    // There is a case here where the tx is being sent in the disconnect function.
    // the tx, doesn't neccesarily track to my session. It could be a different session.
    // To fix this, I need to track the session id of the user, and then remove the user from the session.

    let mut session_lock = sessions.write().await;
    if let Some(session_list) = session_lock.get_mut(&session_id) {
        session_list.retain(|user| !user.is_closed());
    };

    let mut users_lock = users.write().await;
    users_lock.remove(&my_id);

    let mut user_sessions_lock = user_sessions.write().await;
    user_sessions_lock.remove(&my_id);

    drop(user_sessions_lock);
    drop(session_lock);

    if let Some(recipients) = sessions.write().await.get_mut(&session_id) {
        let leaving_message = Message::text("User is leaving");

        // This filters out the user that is leaving from the recipients list.
        // Which should not actually exist in the list.
        let filtered_recipients: Vec<&mut mpsc::UnboundedSender<Result<Message, warp::Error>>> =
            recipients
                .iter_mut()
                .filter(|user| !user.same_channel(my_user))
                .collect();

        for tx in filtered_recipients {
            if tx.send(Ok(leaving_message.clone())).is_err() {
                tx.closed().await;
                drop(users_lock.clone());
            }
        }
    } else {
        drop(users_lock);
    }
}
