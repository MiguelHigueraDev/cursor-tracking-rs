use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type ClientId = u8;
type WsStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

struct SharedState {
    used_ids: HashSet<ClientId>,
    clients: HashMap<ClientId, WsStream>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            used_ids: HashSet::new(),
            clients: HashMap::new(),
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

    fn register(&mut self, id: ClientId, stream: WsStream) {
        self.used_ids.insert(id);
        self.clients.insert(id, stream);
    }

    fn unregister(&mut self, id: ClientId) {
        self.used_ids.remove(&id);
        self.clients.remove(&id);
    }

    async fn broadcast_user_list(&mut self) {
        let user_ids: Vec<ClientId> = self.used_ids.iter().copied().collect();
        let mut msg = vec![MessageType::InitialUserList.as_byte()];
        msg.extend(user_ids);

        for (_, stream) in &mut self.clients {
            if let Err(e) = stream.send(Message::Binary(msg.clone().into())).await {
                eprintln!("Failed to send user list: {}", e);
            }
        }
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
}

impl MessageType {
    fn as_byte(&self) -> u8 {
        *self as u8
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(SharedState::new()));
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("WebSocket server running on ws://127.0.0.1:8080");

    while let Ok((stream, addr)) = listener.accept().await {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            handle_connection(stream, addr, state).await;
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    _addr: SocketAddr,
    state: Arc<Mutex<SharedState>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during WebSocket handshake: {}", e);
            return;
        }
    };

    let client_id = {
        let mut state_lock = state.lock().await;
        match state_lock.get_next_id() {
            Some(id) => {
                state_lock.register(id, ws_stream);
                id
            }
            None => {
                eprintln!("Server is full, rejecting connection.");
                return;
            }
        }
    };

    println!("Client {} connected", client_id);

    let mut ws = {
        let mut state_lock = state.lock().await;
        state_lock.clients.remove(&client_id).unwrap()
    };

    // Send the initial connection message to the client
    // Contains the message header and the client ID (2 bytes)
    let init_msg = vec![MessageType::InitialConnection.as_byte(), client_id];
    if let Err(e) = ws.send(Message::Binary(init_msg.into())).await {
        eprintln!("Failed to send initial connection message: {}", e);
        return;
    }

    // Broadcast the user list to all clients
    // This is done after the initial message to avoid sending an empty list
    {
        let mut state_lock = state.lock().await;
        state_lock.clients.insert(client_id, ws);
        state_lock.broadcast_user_list().await;
    }

    let mut ws = {
        let mut state_lock = state.lock().await;
        state_lock.clients.remove(&client_id).unwrap()
    };

    while let Some(Ok(msg)) = ws.next().await {
        if msg.is_text() || msg.is_binary() {
            println!("Received from client {}: {:?}", client_id, msg);
            ws.send(Message::Text(
                format!("Echo from server: {}", msg.into_text().unwrap()).into(),
            ))
            .await
            .unwrap();
        }
    }

    println!("Client {} disconnected", client_id);

    let mut state_lock = state.lock().await;
    state_lock.unregister(client_id);
}
