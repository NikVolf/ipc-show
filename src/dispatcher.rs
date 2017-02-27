use std::io;

use futures::{Future, BoxFuture};
use tokio_proto::multiplex::ClientService;
use tokio_core::io::Io;
use tokio_service::Service;

use service::ServiceError;
use proto::IpcProto;

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
    service: ClientService<T, IpcProto>,
}

impl<T: Io + 'static> Dispatcher<T> {
    pub fn new(service: ClientService<T, IpcProto>) -> Self {
        Dispatcher { service: service }
    }

    pub fn invoke(&self, request: Request) -> BoxFuture<Response, io::Error> {
        self.service.call(request).boxed()
    } 
}