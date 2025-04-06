# Cursor Tracking server

A WebSocket cursor tracking server written in Rust used for showing live cursors on my [portfolio website](https://miguelhiguera.dev). Currently not implemented in the website.
 
It works by exchanging binary messages between client and server.

## Binary protocol

Each message sent by the server contains a header byte to signal the client what type of message it is and how to handle it correctly.

**Message types:**

1. InitialConnection (0x1): Contains an extra byte with the user ID of the user that just connected.

2. InitialUserList (0x2): Contains a list of all the users currently connected to the server. Each user is represented by a byte with their ID.

3. UserJoined (0x3): Contains an extra byte with the user ID of the user that just joined.

4. UserLeft (0x4): Contains an extra byte with the user ID of the user that just left.

5. CursorUpdate (0x5): Contains the user ID of the user that sent the update, followed by the x and y coordinates of the cursor (2 bytes each, uint16).

6. CursorStartedClicking (0x6): Contains an extra byte with the user ID of the user that started clicking.

7. CursorStoppedClicking (0x7): Contains an extra byte with the user ID of the user that stopped clicking.

Clients send a message with the CursorUpdate header (0x5) and the x and y coordinates to the server every time the cursor moves. The server then broadcasts this message to all connected clients, except the one that sent it.

## Building from source

### Prerequisites
- Rust 1.86 or later
- Cargo (Rust's package manager)

### Local development build
```bash
# Clone the repository
git clone https://github.com/yourusername/cursor-tracking-rs.git
cd cursor-tracking-rs

# Build in debug mode
cargo build

# Run the server
cargo run
```

### Production build
```bash
cargo build --release
```
The binary will be available at target/release/cursor-tracking-rs.

## Docker deployment

Using docker-compose (recommended)

```bash
# Build and start the container
docker-compose up -d

# To stop the container
docker-compose down
```

Manual Docker commands

```bash
# Build the Docker image
docker build -t cursor-tracking-rs .

# Run the container
docker run -d -p 8080:8080 --name cursor-tracking cursor-tracking-rs

# Stop the container
docker stop cursor-tracking
```

The WebSocket server will be available at ws://your-server-ip:8080.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.