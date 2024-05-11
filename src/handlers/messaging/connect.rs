use futures::StreamExt;
use regex::Regex;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::Message;
use warp::ws::WebSocket;

use super::broadcast::broadcast_msg;
use super::disconnect::disconnect;
use super::send_message::send_message;

static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

// Usize -> User ID
// String -> Session ID
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

pub async fn connect(
    ws: WebSocket,
    users: Users,
    sessions: Sessions,
    users_to_sessions: UserSessions,
) {
    let re = Regex::new(r"<(.+)>").unwrap();

    // Bookkeeping
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Establishing a connection
    let (user_tx, mut user_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    let rx = UnboundedReceiverStream::new(rx);

    tokio::spawn(rx.forward(user_tx));

    users.write().await.insert(my_id, tx.clone());

    let initial_message = Message::text("Please submit a chat group id");
    let joining_message = Message::text("You have joined the chat!");
    let chat_join_notification = Message::text("User has joined the chat!");

    send_message(initial_message.clone(), &tx).await;

    let my_session;

    // Establishing shared state and session. Initial messaging section.
    loop {
        if !users_to_sessions.read().await.contains_key(&my_id) {
            if let Some(Ok(message)) = StreamExt::next(&mut user_rx).await {
                if let Ok(string_message) = message.to_str() {
                    if let Some(ses_num) = re.captures(string_message) {
                        my_session = ses_num[0].to_string();
                        users_to_sessions
                            .write()
                            .await
                            .insert(my_id, ses_num[0].to_string());

                        let mut session_lock = sessions.write().await;
                        if let Some(session_list) = session_lock.get_mut(&ses_num[0].to_string()) {
                            session_list.push(tx.clone());

                            send_message(joining_message.clone(), &tx).await;
                            break;
                        };

                        session_lock.insert(ses_num[0].to_string(), vec![tx.clone()]);

                        send_message(joining_message.clone(), &tx).await;

                        break;
                    } else {
                        send_message(initial_message.clone(), &tx).await;
                    }
                }
            }
        }
    }

    // Send the user joining message
    broadcast_msg(chat_join_notification, &sessions, my_session.clone(), &tx).await;

    // Reading and broadcasting messages
    while let Some(result) = user_rx.next().await {
        if let Ok(message) = result {
            if message.is_close() {
                break;
            }
            broadcast_msg(message, &sessions, my_session.clone(), &tx).await;

            continue;
        } else {
            break;
        }
    }

    // Disconnect
    disconnect(
        my_session.clone(),
        my_id,
        &tx,
        &users,
        &sessions,
        &users_to_sessions,
    )
    .await;
}
