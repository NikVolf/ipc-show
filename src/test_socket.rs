use std::sync::RwLock;
use futures::stream::Stream;
use futures::sink::Sink;
use futures::{Async, AsyncSink};
use dispatcher::{TransportError};

struct ReadPayload {
    data: Vec<u8>,
    is_delayed: bool,
}

#[derive(Default)]
pub struct TestSocket {
    reads: RwLock<Vec<ReadPayload>>,
    read_idx: RwLock<usize>,
    writes: RwLock<Vec<Vec<u8>>>,
}

impl TestSocket {
    pub fn with_replies(repl: Vec<Vec<u8>>) -> Self {
        let mut socket = TestSocket::default();
        for reply in repl {
            socket.reads.write().unwrap().push(ReadPayload { data: reply, is_delayed: false });
        }

        socket
    }
}

impl Stream for TestSocket {
    type Item = Vec<u8>;
    type Error = TransportError;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let mut reads = self.reads.write().unwrap();
        let mut read_idx = self.read_idx.write().unwrap();
        if *read_idx >= reads.len() { return Ok(Async::Ready(None)); }

        let mut payload = reads.get_mut(*read_idx).expect("Length checked above");
        if payload.is_delayed {
            payload.is_delayed = false;
            Ok(Async::NotReady)
        } else {
            *read_idx += 1;
            Ok(Async::Ready(Some(payload.data.clone())))
        }
    }
}

impl Sink for TestSocket {
    type SinkItem = Vec<u8>;
    type SinkError = TransportError;

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        self.writes.write().unwrap().push(item);
        Ok(AsyncSink::Ready)
    }
 
    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        Ok(Async::Ready(()))        
    }
}

#[cfg(test)]
mod tests {
}