use tokio_core::io::{Codec, EasyBuf};
use std::io;
use byteorder::{BigEndian, ByteOrder};
use bincode;
use serde;

use dispatcher::{Request, Response};

struct BincodeCodec<I: serde::Deserialize, O: serde::Serialize> {
    _phantom_input: ::std::marker::PhantomData<I>,
    _phantom_output: ::std::marker::PhantomData<O>,    
}

type ServerCodec = BincodeCodec<Request, Response>;
type ClientCodec = BincodeCodec<Response, Request>; 

impl<I: serde::Deserialize, O: serde::Serialize> Codec for BincodeCodec<I, O> {

    type In = I;
    type Out = O;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        if buf.len() <= 4 {
            return Ok(None);
        }

        let request_length = BigEndian::read_u32(&buf.as_ref()[0..4]);

        if buf.len() < request_length as usize + 4 {
            return Ok(None);
        }

        let payload = buf.drain_to(request_length as usize +4);

        Ok(bincode::deserialize(payload.as_slice()).expect("TODO: deserialization error"))
    }

    fn encode(&mut self, msg: O, buf: &mut Vec<u8>) -> io::Result<()> {
        let serialized = bincode::serialize(&msg, bincode::SizeLimit::Infinite).expect("Unreachable exception. Out of memory?");
        let mut len = [0u8; 4];
        BigEndian::write_u32(&mut len[..], serialized.len() as u32);
        buf.extend(serialized);
        Ok(())
    }
}