use tokio::sync::mpsc;
use warp::ws::Message;

pub async fn send_message(
    msg: Message,
    user: &mpsc::UnboundedSender<Result<Message, warp::Error>>,
) {
    if msg.to_str().is_ok() && user.send(Ok(msg.clone())).is_err() {}
}
