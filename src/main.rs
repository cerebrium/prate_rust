use std::collections::HashMap;

use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let local_port = listener.local_addr().unwrap().port();
    println!("listening on port: {:?}", local_port);

    let (tx, _rx) = broadcast::channel(10);

    let re = Regex::new(r"<(.+)>").unwrap();

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let re = re.clone();
        let sessions = HashMap::new();

        tokio::spawn(async move {
            // Splits the socket into read section and write section to allow for
            // ownership to be used in different places
            let (reader, mut writer) = socket.split();

            // Buf Reader is an automagic memory managed buffer (ring buffer? look into it)
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let mut user_session = String::new();

            loop {
                // Allows for concurrent actions to race and whichever finishes
                // first is acted on
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 {
                          break;
                        }

                        if user_session.is_empty() {
                           if let Some(ses_num) = re.captures(&line.clone()) {
                               user_session.push_str(&ses_num[0]);
                           } else {
                              line.clear();
                              writer.write_all("Please submit a connection number: ex: <1>\n".as_bytes()).await.unwrap();
                           }
                        } else {
                          tx.send((line.clone(), addr)).unwrap();
                          line.clear();
                        };

                                        }

                    result = rx.recv() => {
                        let (msg, other_addr) = result.unwrap();
                        if addr != other_addr {
                            println!("What is the user session: {:?}", user_session);
                            writer.write_all(msg.as_bytes()).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}
