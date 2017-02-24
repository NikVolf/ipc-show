#[macro_use] extern crate serde_derive;
extern crate futures;
extern crate serde;
extern crate bincode;

mod common;
mod dispatcher;
mod test_socket;

use futures::{future, Future, BoxFuture};
use std::sync::Arc;

pub use common::{Block, BlockError};
pub use dispatcher::{Dispatcher, IpcSocket, ServiceId, Request, Response, IpcInterface, ServiceError, ServiceRequest, ServiceResponse};
use test_socket::TestSocket;

pub type IpcFuture<T, E> = futures::BoxFuture<T, E>;
pub type IpcStream<T, E> = futures::stream::BoxStream<T, E>;

pub trait Client {
    fn load(&self, number: u64) -> IpcFuture<Block, BlockError>;
}

pub struct ClientService {
    client: Box<Client>,
}

pub struct IpcClient<T: IpcSocket> {
    service_id: ServiceId,
    dispatcher: Arc<Dispatcher<T>>,
}

impl<T: IpcSocket> Client for IpcClient<T> {

    // CODEGEN METHOD
    fn load(&self, number: u64) -> IpcFuture<Block, BlockError> {
        let payload = bincode::serialize(&number, bincode::SizeLimit::Infinite).expect("Known serializable");

        self.dispatcher.invoke(Request {
            id: 0,
            method_id: 0,
            service_id: self.service_id,
            payload: payload,
        }).then(|response| {
            match response {
                Ok(response) => {
                    if response.success {
                        let block: Block = bincode::deserialize(&response.payload).unwrap();
                        Ok(block)
                    } else {
                        let block_error: BlockError = bincode::deserialize(&response.payload).unwrap();
                        Err(block_error)
                    }
                },
                Err(_) => panic!(),
            }
        }).boxed()
    }
}

impl IpcInterface for Client {
    fn dispatch(&self, request: ServiceRequest) -> BoxFuture<ServiceResponse, ServiceError> {
        // CODEGEN RESULT START
        match request.method_id {
            0 => {
                let param: u64 = bincode::deserialize(&request.payload).unwrap();
                self.load(param).then(|res| {
                    match res {
                        Ok(response) => {
                            Ok(ServiceResponse { 
                                success: true, 
                                payload: bincode::serialize(&response, bincode::SizeLimit::Infinite).unwrap(),
                            })
                        },
                        Err(err) => {
                            Ok(ServiceResponse { 
                                success: false, 
                                payload: bincode::serialize(&err, bincode::SizeLimit::Infinite).unwrap() 
                            })
                        }
                    }
                }).boxed()
            },
            _ => future::err(ServiceError).boxed(),
        }
        // CODEGEN RESULT END
    }
}

#[cfg(test)]
mod tests {
}
