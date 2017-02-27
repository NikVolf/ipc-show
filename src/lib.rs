#[macro_use] extern crate serde_derive;
extern crate futures;
extern crate serde;
extern crate bincode;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate byteorder;

mod common;
mod dispatcher;
mod proto;
mod service;

use std::sync::{Mutex, Arc};
use futures::{future, Future, BoxFuture};
use tokio_core::io::Io;
use tokio_proto::multiplex;

pub use common::{Block, BlockError};
pub use dispatcher::{Dispatcher, ServiceId, Request, Response, IpcInterface, ServiceRequest};
pub use service::ServiceError;
use proto::IpcProto;

pub type IpcFuture<T, E> = futures::BoxFuture<T, E>;
pub type IpcStream<T, E> = futures::stream::BoxStream<T, E>;

pub trait Client {
    fn load(&self, number: u64) -> IpcFuture<Block, BlockError>;
}

pub struct ClientService;

impl Client for ClientService {
    fn load(&self, number: u64) -> IpcFuture<Block, BlockError> {
        future::ok(Block { number: number, data: vec![0u8; 128] }).boxed()
    }
}

pub struct IpcClient<T: Io + 'static> {
    service_id: ServiceId,
    dispatcher: Dispatcher<T>,
    core: Mutex<::tokio_core::reactor::Core>,
}

// CODEGEN CONSTRUCTOR
impl<T: Io + 'static> IpcClient<T> {
    fn new(service_id: ServiceId, client_service: multiplex::ClientService<T, IpcProto>) -> Self {
        IpcClient {
            dispatcher: Dispatcher::new(client_service),
            service_id: service_id,
            core: Mutex::new(::tokio_core::reactor::Core::new().unwrap()),
        }
    }
}
// CODEGEN CONSTRUCTOR END

impl<T: Io + 'static> Client for IpcClient<T> {
    // CODEGEN METHOD
    fn load(&self, number: u64) -> IpcFuture<Block, BlockError> {
        let payload = bincode::serialize(&number, bincode::SizeLimit::Infinite).expect("Known serializable");

        self.dispatcher.invoke(Request {
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
    // CODEGEN METHOD END
}

// CODEGEN DISPATCH STRUCT
pub struct ClientDispatch<T: Client>(Arc<T>);
// CODEGEN DISPATCH STRUCT END

impl<T: Client> IpcInterface for ClientDispatch<T> {
    fn dispatch(&self, request: ServiceRequest) -> BoxFuture<Response, ServiceError> {
        // CODEGEN RESULT START
        match request.method_id {
            0 => {
                let param: u64 = bincode::deserialize(&request.payload).unwrap();
                self.0.load(param).then(|res| {
                    match res {
                        Ok(response) => {
                            Ok(Response { 
                                success: true, 
                                payload: bincode::serialize(&response, bincode::SizeLimit::Infinite).unwrap(),
                            })
                        },
                        Err(err) => {
                            Ok(Response { 
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

    use std::sync::{Arc, Mutex};
    use std::thread;
    use tokio_proto::{TcpServer, TcpClient};
    use service::{ServiceDispatcher, ServiceError};
    use proto::{IpcProto};
    use super::{IpcClient, Client, ClientService, ClientDispatch};
    use dispatcher::IpcInterface;
    use tokio_core::reactor::Core;   
    use futures::{future, task, Future, Async};

    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn run_server(addr: &'static str) {
        let addr = addr.parse().unwrap();
        let tcp_server = TcpServer::new(IpcProto, addr);
        thread::spawn(move || tcp_server.serve(|| {
            println!("Connected peer");
            let mut service_dispatcher = ServiceDispatcher::new();
            let client_service = Arc::new(ClientService);
            let client_dispatch = Arc::new(ClientDispatch(client_service));
            service_dispatcher.register(0, client_dispatch as Arc<IpcInterface>).unwrap();
            println!("Created service");            
            Ok(service_dispatcher)
        }));
    }

    #[test]
    fn server() {
        run_server("0.0.0.0:22000");
        thread::sleep(::std::time::Duration::from_millis(100));
    }

    #[test]
    fn conn() {
        run_server("0.0.0.0:22010");
        thread::sleep(::std::time::Duration::from_millis(100));
    
        let mut core = Core::new().unwrap();

        println!("Connecting...");
        let client = TcpClient::new(IpcProto);
        let handle = core.handle();
        let client_service = core.run(client.connect(&"127.0.0.1:22010".parse().unwrap(), &handle)).unwrap();

        println!("Invoking...");
        let client = IpcClient::new(0, client_service);
        let block = core.run(client.load(5)).unwrap();

        assert_eq!(block.number, 5);
    }

    #[test]
    fn local() {
        let svc = ClientService;

        assert_eq!(5, svc.load(5).wait().unwrap().number);
    }

    type TaskQueue = Arc<Mutex<Vec<task::Task>>>;

    struct WaitFuture {
        ready: Arc<AtomicBool>,
        val: Arc<AtomicUsize>,
        queue: TaskQueue,
    }

    impl Future for WaitFuture {
        type Item = usize;
        type Error = ();

        fn poll(&mut self) -> Result<Async<Self::Item>, ()> {
            if self.ready.load(Ordering::SeqCst) {
                Ok(Async::Ready(self.val.load(Ordering::SeqCst)))
            } else {
                self.queue.lock().unwrap().push(task::park());
                Ok(Async::NotReady)
            }
        }
    }

    fn push_queue(queue: &TaskQueue, ready: &AtomicBool, val: &AtomicUsize) {
        ready.store(true, Ordering::SeqCst);
        val.store(5, Ordering::SeqCst);
        let mut queue_lock = queue.lock().unwrap(); 
        for task in queue_lock.drain(..) {
            task.unpark();
        }
    }

    #[test]
    fn future() {
        let wait_future = WaitFuture { ready: Arc::default(), val: Arc::default(), queue: Arc::default()};
        let thread_ready = wait_future.ready.clone();
        let thread_val = wait_future.val.clone();
        let thread_queue = wait_future.queue.clone();
        thread::spawn(move || {
            thread::sleep(::std::time::Duration::from_millis(100));
            push_queue(&thread_queue, &thread_ready, &thread_val);
        });

        assert_eq!(5, wait_future.wait().unwrap());
    }

}
