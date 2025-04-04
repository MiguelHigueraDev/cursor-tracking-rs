# Cursor Tracking server

A WebSocket cursor tracking server used for showing live cursors on my [portfolio website](https://miguelhiguera.dev).
 
It works by exchanging binary messages between client and server.

## Binary protocol

Each message sent by the server contains a header byte to signal the client what type of message it is and how to handle it correctly.

**Message types:**

1. InitialConnection (0x1): TODO