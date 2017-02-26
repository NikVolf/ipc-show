use std::io;
use std::sync::{RwLock, Arc, Mutex};
use std::collections::HashMap;

use futures::{self, task, Future, Async, BoxFuture};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_proto::multiplex::ClientService;
use tokio_core::io::Io;
use tokio_service::Service;

use bincode;

use service::ServiceError;
use proto::{IpcProto, IpcClientProto};

#[derive(Debug)]
pub struct TransportError;

pub type MethodId = u16;
pub type ServiceId = u16;

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub service_id: ServiceId,
    pub method_id: MethodId,
    pub payload: Vec<u8>,
}

pub struct ServiceRequest {
    pub method_id: MethodId,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub payload: Vec<u8>,
}

pub trait IpcInterface {
    fn dispatch(&self, request: ServiceRequest) -> BoxFuture<Response, ServiceError>;
}

pub struct Dispatcher<T: Io + 'static> {
    client_service: ClientService<T, IpcClientProto>,
}

impl<T: Io + 'static> Dispatcher<T> {
    pub fn invoke(&self, request: Request) -> BoxFuture<Response, io::Error> {
        self.client_service.call(request).boxed()
    } 
}