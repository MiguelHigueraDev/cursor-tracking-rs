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

Clients send a message with the CursorUpdate header (0x5) and the x and y coordinates to the server every time the cursor moves. The server then broadcasts this message to all connected clients, except the one that sent it.