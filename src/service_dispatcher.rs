use bincode;
use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;
use futures::{BoxFuture, Future};

use dispatcher::{ServiceId, IpcInterface, IpcSocket, Request, ServiceRequest, TransportError};

struct ServiceError;

struct ServiceDispatcher<T: IpcSocket + Send> {
    services: RwLock<HashMap<ServiceId, Arc<IpcInterface>>>,
    socket: Mutex<T>,
}

impl<T: IpcSocket + Send> ServiceDispatcher<T> {

    fn new(socket: T) -> Self {
        ServiceDispatcher {
            socket: Mutex::new(socket),
            services: Default::default(),
        }
    }

    fn run(&self) -> BoxFuture<(), TransportError> {
        self.socket.lock().unwrap().for_each(|payload| {
            let request: Request = bincode::deserialize(&payload).expect("TODO: serialization error");
            let service = self.services.read().unwrap().get(&request.service_id).expect("TODO: missing service error");
            let service_request = ServiceRequest { method_id: request.method_id, payload: request.payload };
            service.dispatch(service_request);
            Ok(())
        }).boxed()
    }

    fn register_service(&self, service_id: ServiceId, service: Arc<IpcInterface>) -> Result<(), ServiceError> {
        let mut services = self.services.write().unwrap();
        if let Some(_) = services.get(&service_id) {
            return Err(ServiceError);
        }

        services.insert(service_id, service);
        Ok(())
    }

    fn deregister_service(&self, service_id: ServiceId) -> Result<(), ServiceError> {
        let mut services = self.services.write().unwrap();
        if services.remove(&service_id).is_none() {
            Err(ServiceError)
        } else {
            Ok(())
        }
    }

}