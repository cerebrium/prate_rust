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

    if let Ok(mut map) = sessions.clone().write() {
        map.insert("<1>", vec![]);
        map.insert("<2>", vec![]);
        map.insert("<3>", vec![]);
    } else {
        panic!("rust is broken");
    };

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let re = re.clone();

        // Store the sessions:
        // Map of group id's with the addr to send to.
        let session_writer = sessions.clone();

        // The text that is being sent
        let mut line = String::new();
        let mut user_group = String::new();

        tokio::spawn(async move {
            // Splits the socket into read section and write section to allow for
            // ownership to be used in different places
            let (reader, mut writer) = socket.split();

            // Buf Reader is an automagic memory managed buffer (ring buffer? look into it)
            let mut reader = BufReader::new(reader);

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
                               println!("the user wants: {:?} ", &ses_num[0]);

                               user_group.push_str(&ses_num[0]);
                                if let Ok(mut map) = session_writer.write() {
                                    if let Some(mut group) = map.get_mut(&ses_num[0]) {
                                        group.push(addr)
                                    }
                                } else {
                                    panic!("panic at trying to obtain the write lock")
                                }
                            } else {
                               line.clear();
                               writer.write_all("Please submit a connection number: ex: <1> .. <3>\n".as_bytes()).await.unwrap();
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
                            let mut escape_thread_list_copy = None;
                            if let Ok(map) = session_writer.read() {
                                println!("the map: {:?}", map);
                               if let Some(chat_list) = map.get(&*user_group) {
                                  escape_thread_list_copy = Some(chat_list.clone());
                               }
                            }

                            if let Some(local_list) = escape_thread_list_copy {
                                println!("local list: {:?}", local_list);
                                for add in local_list {
                                    if add != addr && add != other_addr {
                                        writer.write_all(msg.as_bytes()).await.unwrap();
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
