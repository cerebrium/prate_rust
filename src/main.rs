use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

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
    let mut sessions: Arc<RwLock<HashMap<&str, Vec<SocketAddr>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let re = re.clone();

        // Store the sessions:
        // Map of group id's with the addr to send to.
        let mut session_writer = sessions.clone();

        // The text that is being sent
        let mut line = String::new();

        tokio::spawn(async move {
            // Splits the socket into read section and write section to allow for
            // ownership to be used in different places
            let (reader, mut writer) = socket.split();

            // Buf Reader is an automagic memory managed buffer (ring buffer? look into it)
            let mut reader = BufReader::new(reader);
            let mut user_group = String::new();

            loop {
                // Allows for concurrent actions to race and whichever finishes
                // first is acted on
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 {
                          break;
                        }

                        if user_group.is_empty() {
                           if let Some(ses_num) = re.captures(&line.clone()) {
                               user_group.push_str(&ses_num[0]);
                               // let map = session_writer.write();
                               // map.unwrap().entry(&ses_num[0]).or_insert(Vec::new()).push(addr);
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

                        // Check for the user group
                        if !user_group.is_empty() {
                            if let Ok(map) = session_writer.read() {
                               if let Some(chat_list) = map.get(&user_group) {
                                  for add in chat_list {
                                    if add != other_addr {
                                        writer.write_all(msg.as_bytes()).await.unwrap();
                                    }
                                  }
                               }
                            }
                        }
                    }
                }
            }
        });
    }
}
