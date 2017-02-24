# ipc-show
Rust IPC Prototype

- Multiplexing
- Bincode as serializer
- Abstract sockets (just implement `Stream+Sink`) 
- Several services over one socket
- Futures-based, supposed to be run with tokio core event loop
