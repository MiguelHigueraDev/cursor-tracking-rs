use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    sync::{Mutex, broadcast},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type ClientId = u8;

#[derive(Clone, Debug)]
enum ServerMessage {
    UserJoined(ClientId),
    UserLeft(ClientId),
    UserList(Vec<ClientId>),
    CursorMoved(ClientId, u16, u16),
    CursorStartedClicking(ClientId),
    CursorStoppedClicking(ClientId),
}

struct SharedState {
    used_ids: HashSet<ClientId>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            used_ids: HashSet::new(),
        }
    }

    fn get_next_id(&self) -> Option<ClientId> {
        for id in 0..=u8::MAX {
            if !self.used_ids.contains(&id) {
                return Some(id);
            }
        }
        None
    }

    fn register(&mut self, id: ClientId) {
        self.used_ids.insert(id);
    }

    fn unregister(&mut self, id: ClientId) {
        self.used_ids.remove(&id);
    }

    fn get_user_list(&self) -> Vec<ClientId> {
        self.used_ids.iter().copied().collect()
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum MessageType {
    InitialConnection = 0x01,
    InitialUserList = 0x02,
    UserJoined = 0x03,
    UserLeft = 0x04,
    UserMovedCursor = 0x05,
    UserStartedClicking = 0x06,
    UserStoppedClicking = 0x07,
}

impl MessageType {
    fn as_byte(&self) -> u8 {
        *self as u8
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(SharedState::new()));
    let (broadcast_tx, _) = broadcast::channel::<ServerMessage>(100);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("WebSocket server running on ws://0.0.0.0:8080");

    while let Ok((stream, addr)) = listener.accept().await {
        let state = Arc::clone(&state);
        let broadcast_tx = broadcast_tx.clone();
        tokio::spawn(async move {
            handle_connection(stream, addr, state, broadcast_tx).await;
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    _addr: SocketAddr,
    state: Arc<Mutex<SharedState>>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during WebSocket handshake: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let client_id = {
        let mut state_lock = state.lock().await;
        match state_lock.get_next_id() {
            Some(id) => {
                state_lock.register(id);
                id
            }
            None => {
                eprintln!("Server is full, rejecting connection.");
                return;
            }
        }
    };

    println!("Client {} connected", client_id);

    async fn send_binary_message(
        sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            Message,
        >,
        data: Vec<u8>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        sender.send(Message::Binary(data.into())).await
    }

    // Send the initial connection message to the client
    let init_msg = vec![MessageType::InitialConnection.as_byte(), client_id];
    if let Err(e) = send_binary_message(&mut ws_sender, init_msg).await {
        eprintln!("Error sending initial message: {}", e);
        return;
    }

    // Get the current user list and send it to the new client
    let user_list = {
        let state_lock = state.lock().await;
        state_lock.get_user_list()
    };

    let mut user_list_msg = vec![MessageType::InitialUserList.as_byte()];
    user_list_msg.extend(user_list.iter().copied());
    if let Err(e) = send_binary_message(&mut ws_sender, user_list_msg).await {
        eprintln!("Error sending user list: {}", e);
        return;
    }

    // Notify other clients about the new user
    let _ = broadcast_tx.send(ServerMessage::UserJoined(client_id));

    // Subscribe to the broadcast channel to receive updates from other clients
    let mut broadcast_rx = broadcast_tx.subscribe();

    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle messages from the broadcast channel
                Ok(msg) = broadcast_rx.recv() => {
                    match msg {
                        ServerMessage::UserJoined(id) => {
                            if id != client_id {
                                let msg = vec![MessageType::UserJoined.as_byte(), id];
                                if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                    eprintln!("Error sending join message: {}", e);
                                    break;
                                }
                            }
                        },
                        ServerMessage::UserLeft(id) => {
                            if id != client_id {
                                let msg = vec![MessageType::UserLeft.as_byte(), id];
                                if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                    eprintln!("Error sending leave message: {}", e);
                                    break;
                                }
                            }
                        },
                        ServerMessage::UserList(ids) => {
                            let mut msg = vec![MessageType::InitialUserList.as_byte()];
                            msg.extend(ids);
                            if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                eprintln!("Error sending user list: {}", e);
                                break;
                            }
                        },
                        ServerMessage::CursorMoved(id, x, y) => {
                            if id != client_id {
                                let mut msg = vec![MessageType::UserMovedCursor.as_byte(), id];
                                msg.extend_from_slice(&x.to_be_bytes());
                                msg.extend_from_slice(&y.to_be_bytes());
                                if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                    eprintln!("Error sending cursor message: {}", e);
                                    break;
                                }
                            }
                        },
                        ServerMessage::CursorStartedClicking(id) => {
                            if id != client_id {
                                let msg = vec![MessageType::UserStartedClicking.as_byte(), id];
                                if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                    eprintln!("Error sending cursor click start message: {}", e);
                                    break;
                                }
                            }
                        },
                        ServerMessage::CursorStoppedClicking(id) => {
                            if id != client_id {
                                let msg = vec![MessageType::UserStoppedClicking.as_byte(), id];
                                if let Err(e) = send_binary_message(&mut ws_sender, msg).await {
                                    eprintln!("Error sending cursor click stop message: {}", e);
                                    break;
                                }
                            }
                        },
                    }
                },

                else => break,
            }
        }

        println!("Sender task for client {} ended", client_id);
    });

    // Process incoming messages from the WebSocket
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if msg.is_binary() {
            let data = msg.into_data();
            if !data.is_empty() {
                match data[0] {
                    // Cursor moved
                    0x05 => {
                        if data.len() >= 5 {
                            let x = u16::from_be_bytes([data[1], data[2]]);
                            let y = u16::from_be_bytes([data[3], data[4]]);
                            let _ = broadcast_tx.send(ServerMessage::CursorMoved(client_id, x, y));
                        }
                    }
                    // Cursor started clicking
                    0x06 => {
                        let _ = broadcast_tx.send(ServerMessage::CursorStartedClicking(client_id));
                    }
                    // Cursor stopped clicking
                    0x07 => {
                        let _ = broadcast_tx.send(ServerMessage::CursorStoppedClicking(client_id));
                    }
                    // Discard other message types
                    _ => {}
                }
            }
        }
    }

    println!("Client {} disconnected", client_id);

    sender_task.abort();

    {
        let mut state_lock = state.lock().await;
        state_lock.unregister(client_id);
    }
    let _ = broadcast_tx.send(ServerMessage::UserLeft(client_id));
}
