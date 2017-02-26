use bincode;
use std::io;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use futures::{future, BoxFuture, Future};
use tokio_service::Service;
use tokio_proto::multiplex::ServerProto;

use dispatcher::{ServiceId, IpcInterface, Request, Response, ServiceRequest, TransportError};

pub struct ServiceError;

pub type Services = HashMap<ServiceId, Arc<IpcInterface>>;

pub struct ServiceDispatcher {
    services: Services,
}

impl ServiceDispatcher {
    pub fn new() -> Self {
        ServiceDispatcher {
            services: Default::default(),
        }
    }

    pub fn register_service(&mut self, service_id: ServiceId, service: Arc<IpcInterface>) -> Result<(), ServiceError> {
        if let Some(_) = self.services.get(&service_id) {
            return Err(ServiceError);
        }

        self.services.insert(service_id, service);
        Ok(())
    }

    pub fn deregister_service(&mut self, service_id: ServiceId) -> Result<(), ServiceError> {
        if self.services.remove(&service_id).is_none() {
            Err(ServiceError)
        } else {
            Ok(())
        }
    }

    pub fn service(&self, service_id: ServiceId) -> &IpcInterface {
        &*self.services[&service_id]
    }
}

impl Service for ServiceDispatcher {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = BoxFuture<Response, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let service = self.service(req.service_id);
        let service_request = ServiceRequest { method_id: req.method_id, payload: req.payload };
        service.dispatch(service_request).map_err(|_| io::Error::new(io::ErrorKind::Other, "TODO: service error")).boxed()
    }
}