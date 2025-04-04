use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

#[repr(u8)]
#[derive(Copy, Clone)]
enum MessageType {
    InitialConnection = 0x01,
    UserJoined = 0x02,
    UserLeft = 0x03,
    UserMovedCursor = 0x04,
}

impl MessageType {
    fn as_byte(&self) -> u8 {
        *self as u8
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from {}", addr);

        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Error during the websocket handshake");
            println!("WebSocket connection established: {}", addr);

            let (mut write, mut read) = ws_stream.split();

            // Send UUID to the client on initial connection
            let uuid_bytes = generate_uuid_bytes();
            let mut payload = Vec::with_capacity(1 + uuid_bytes.len());
            payload.push(MessageType::InitialConnection.as_byte());
            payload.extend_from_slice(&uuid_bytes);

            let response = Message::Binary(payload.into());
            if let Err(e) = write.send(response).await {
                println!("Failed to send initial UUID: {}", e);
                return;
            }

            // Echo incoming messages back to the client
            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        if msg.is_text() || msg.is_binary() {
                            write.send(msg).await.expect("Failed to send message");
                        }
                    }
                    Err(e) => {
                        println!("Error processing message: {}", e);
                        break;
                    }
                }
            }

            println!("Connection closed: {}", addr);
        });
    }

    Ok(())
}

fn generate_uuid_bytes() -> [u8; 16] {
    *Uuid::new_v4().as_bytes()
}
