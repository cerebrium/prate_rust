use futures::StreamExt;
use regex::Regex;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
type Sessions =
    Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Result<Message, warp::Error>>>>>>;
type UserSessions = Arc<RwLock<HashMap<usize, String>>>;

// TODO: .unwrap_or_else(|| "0.0.0.0:3000".to_string());
#[tokio::main]
async fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let socket_address: SocketAddr = addr.parse().expect("valid socket Address");

    let users = Users::default();
    let users = warp::any().map(move || users.clone());

    let users_to_sessions = UserSessions::default();
    let users_to_sessions = warp::any().map(move || users_to_sessions.clone());

    let sessions = Sessions::default();
    let sessions = warp::any().map(move || sessions.clone());

    // GET /ws
    let chat = warp::path("ws")
        .and(warp::ws())
        .and(users)
        .and(sessions)
        .and(users_to_sessions)
        .map(|ws: warp::ws::Ws, users, sessions, users_to_sessions| {
            ws.on_upgrade(move |socket| connect(socket, users, sessions, users_to_sessions))
        });

    let res_404 = warp::any().map(|| {
        warp::http::Response::builder()
            .status(warp::http::StatusCode::NOT_FOUND)
            .body(
                fs::read_to_string(
                    env::current_dir()
                        .unwrap()
                        .join("../var/www/static/404.html"),
                )
                .expect("404 404?"),
            )
    });

    let routes = chat.or(res_404);
    let server = warp::serve(routes).try_bind(socket_address);
    println!("Running server at {}!", addr);

    server.await
}

async fn connect(ws: WebSocket, users: Users, sessions: Sessions, users_to_sessions: UserSessions) {
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
                println!("Message: {:?}", message);
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
                } else {
                    println!("Could not convert message to str");

                    if let my_sess = Some(my_session.clone()) {
                        disconnect(
                            my_session.clone(),
                            my_id,
                            &tx,
                            &users,
                            &sessions,
                            &users_to_sessions,
                        );
                    }
                }
            }
        }
    }

    // Send the user joining message
    broadcast_msg(
        chat_join_notification,
        &sessions,
        my_session.clone(),
        &tx,
        my_id,
        &users,
        &users_to_sessions,
    )
    .await;

    // Reading and broadcasting messages
    while let Some(result) = user_rx.next().await {
        if let Ok(message) = result {
            broadcast_msg(
                message,
                &sessions,
                my_session.clone(),
                &tx,
                my_id,
                &users,
                &users_to_sessions,
            )
            .await;
        } else {
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

async fn send_message(msg: Message, user: &mpsc::UnboundedSender<Result<Message, warp::Error>>) {
    if msg.to_str().is_ok() && user.send(Ok(msg.clone())).is_err() {
        println!("error at send message, reset be.")
    }
}

async fn broadcast_msg(
    msg: Message,
    sessions: &Sessions,
    my_session: String,
    my_user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
    my_id: usize,
    users: &Users,
    users_to_sessions: &UserSessions,
) {
    if msg.to_str().is_ok() {
        if let Some(recipients) = sessions.read().await.get(&my_session) {
            for tx in recipients {
                if !tx.same_channel(my_user) && tx.send(Ok(msg.clone())).is_err() {
                    disconnect(
                        my_session.clone(),
                        my_id,
                        tx,
                        users,
                        sessions,
                        users_to_sessions,
                    )
                    .await;

                    println!("disconnecting user: {:?}", tx);
                }
            }
        }
    }
}

async fn disconnect(
    session_id: String,
    my_id: usize,
    my_user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
    users: &Users,
    sessions: &Sessions,
    user_sessions: &UserSessions,
) {
    users.write().await.remove(&my_id);
    user_sessions.write().await.remove(&my_id);

    if let Some(recipients) = sessions.write().await.get_mut(&session_id) {
        let leaving_message = Message::text("User is leaving");

        let filtered_recipients: Vec<&mut mpsc::UnboundedSender<Result<Message, warp::Error>>> =
            recipients
                .iter_mut()
                .filter(|user| !user.same_channel(my_user))
                .collect();

        for tx in filtered_recipients {
            if tx.send(Ok(leaving_message.clone())).is_err() {
                println!("Error in disconnection message.")
            }
        }
    };
}
