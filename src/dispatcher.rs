use std::sync::{RwLock, Arc, Mutex};
use std::collections::HashMap;

use futures::{self, task, Future, Async, BoxFuture};
use futures::stream::Stream;
use futures::sink::Sink;

use bincode;

#[derive(Debug)]
pub struct TransportError;

pub trait IpcSocket : Sink<SinkItem=Vec<u8>, SinkError=TransportError> + Stream<Item=Vec<u8>, Error=TransportError> {
} 

pub struct Dispatcher<T: IpcSocket> {
    socket: Mutex<T>,
    requests: RwLock<HashMap<RequestId, Request>>,
    response_queue: Arc<ResponseQueue>,
    task_queue: Arc<ResponseTaskQueue>,
}

pub type MethodId = u16;
pub type ServiceId = u16;
pub type RequestId = u32;

type ResponseTaskQueue = Mutex<Vec<task::Task>>;
type ResponseQueue = Mutex<HashMap<RequestId, Response>>;

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub id: RequestId,
    pub service_id: ServiceId,
    pub method_id: MethodId,
    pub payload: Vec<u8>,
}

pub struct ServiceRequest {
    pub method_id: MethodId,
    pub payload: Vec<u8>,
}

pub struct ServiceResponse {
    pub success: bool,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub id: RequestId,
    pub success: bool,
    pub payload: Vec<u8>,
}


impl<T: IpcSocket> Dispatcher<T> {
    pub fn invoke(&self, request: Request) -> ResponseFuture {
        let id = request.id;
        let request_bytes = bincode::serialize(&request, bincode::SizeLimit::Infinite).unwrap();

        self.requests.write().unwrap().insert(id, request);
        self.socket.lock().unwrap().start_send(request_bytes).expect("Error writing to the socket");

        ResponseFuture { 
            task_queue: self.task_queue.clone(), 
            response_queue: self.response_queue.clone(), 
            request_id: id,
        }
    } 
}

pub struct ResponseFuture {
    response_queue: Arc<ResponseQueue>,
    task_queue: Arc<ResponseTaskQueue>,
    request_id: RequestId,
}

pub struct Error;
pub struct ServiceError;

impl Future for ResponseFuture {
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Result<futures::Async<Self::Item>, Self::Error> {
        let response = self.response_queue.lock().unwrap().remove(&self.request_id);
        match response {
            None => {
                self.task_queue.lock().unwrap().push(task::park());
                return Ok(Async::NotReady)
            },
            Some(response) => {
                return Ok(Async::Ready(response))
            },
        }
    }
} 

pub trait IpcInterface {
    fn dispatch(&self, request: ServiceRequest) -> BoxFuture<ServiceResponse, ServiceError>;
}