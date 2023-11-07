use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let local_port = listener.local_addr().unwrap().port();
    println!("listening on port: {:?}", local_port);

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();

        let (reader, mut writer) = socket.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            let bytes_per_line = reader.read_line(&mut line).await.unwrap();
            if bytes_per_line == 0 {
                break;
            }
            println!("res: {:?} {:?}", bytes_per_line, addr);

            writer.write_all(&line.as_bytes()).await.unwrap();
            line.clear();
        }
    }
    /*
    let local_port = listener.local_addr()?.port();

    println!("listening on port: {:?}", local_port);

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // Socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read from socket: {:?}", e);
                        return;
                    }
                };

                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("Failed to read from socket: {:?}", e);
                    return;
                }
            }
        });
    }
    */
}
