use std::io;
use std::default::Default;
use tokio_core::io::{Codec, EasyBuf, Framed, Io};
use tokio_proto::multiplex::{RequestId, ServerProto, ClientProto};
use byteorder::{LittleEndian, ByteOrder};
use bincode;
use serde;

use dispatcher::{Request, Response};

pub struct BincodeCodec<I: serde::Deserialize, O: serde::Serialize> {
    _phantom_input: ::std::marker::PhantomData<I>,
    _phantom_output: ::std::marker::PhantomData<O>,    
}

impl<I: serde::Deserialize, O: serde::Serialize> Default for BincodeCodec<I, O> {
    fn default() -> Self {
        BincodeCodec {
            _phantom_input: ::std::marker::PhantomData,
            _phantom_output: ::std::marker::PhantomData,
        }
    }
}

pub type IpcCodec = BincodeCodec<Request, Response>;
pub type IpcClientCodec = BincodeCodec<Response, Request>;

impl<I: serde::Deserialize, O: serde::Serialize> Codec for BincodeCodec<I, O> {

    type In = (RequestId, I);
    type Out = (RequestId, O);

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        println!("Decoding {:?}", buf);

        if buf.len() <= 12 {
            return Ok(None);
        }

        let request_id: RequestId = LittleEndian::read_u64(&buf.as_ref()[0..8]);
        let request_length = LittleEndian::read_u32(&buf.as_ref()[8..12]);

        if buf.len() < request_length as usize + 12 {
            return Ok(None);
        }

        let payload = buf.drain_to(request_length as usize + 12);

        Ok(Some((request_id, bincode::deserialize(&payload.as_slice()[12..]).expect("TODO: deserialization error"))))
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        let (request_id, msg) = msg;
        println!("request_id {}, encoding buffer start {:?}", request_id, buf);

        let mut header = [0u8; 12];
        let serialized = bincode::serialize(&msg, bincode::SizeLimit::Infinite).expect("Unreachable exception. Out of memory?");
        LittleEndian::write_u64(&mut header[0..8], request_id);
        LittleEndian::write_u32(&mut header[8..12], serialized.len() as u32);
        buf.extend(header.to_vec());
        buf.extend(serialized);

        println!("encoding buffer end {:?}", buf);

        Ok(())
    }
}

pub struct IpcProto;

impl<T: Io + 'static> ServerProto<T> for IpcProto {
    type Request = Request;
    type Response = Response;
    type Transport = Framed<T, IpcCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(IpcCodec::default()))
    }
}

impl<T: Io + 'static> ClientProto<T> for IpcProto {
    type Request = Request;
    type Response = Response;
    type Transport = Framed<T, IpcClientCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(IpcClientCodec::default()))
    }
}