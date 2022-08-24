// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{ptr::copy_nonoverlapping, sync::atomic::AtomicU32};

use bytes::{BufMut, BytesMut, Buf};
use chrono::offset;
use futures::SinkExt;
use rsa::rand_core::le;
use tokio_util::codec::{Decoder, Encoder};

use crate::{err::ProtocolError, mysql_const::*, util::get_length};

use super::auth::ServerHandshakeCodec;

pub trait CommonPacket {
    fn make_packet_header(&mut self, length: usize, data: &mut [u8], offset: usize);
    fn reset_seq(&mut self);
    fn get_session(&mut self) -> &mut ServerHandshakeCodec;
}

/// Used to reading packet from client side
pub struct PacketCodec {
    session: ServerHandshakeCodec,
    buf: BytesMut,
    is_complete: bool,
    // Whether the payload is greater than MAX_PAYLOAD_LEN
    is_max: bool,
    seq: u8,
}

impl PacketCodec {
    pub fn new(session: ServerHandshakeCodec, init_size: usize) -> Self {
        Self { 
            session,
            buf: BytesMut::with_capacity(init_size), 
            is_complete: false, 
            is_max: false, 
            seq: 0 
        }
    }


    #[inline]
    fn set_seq_id(&mut self, buf: &mut [u8], offset: usize) {
        unsafe {
            let data_ptr = buf.as_mut_ptr().add(offset);
            *data_ptr.add(3) = self.seq;
        }

        self.seq = self.seq.wrapping_add(1)
    }

    #[inline]
    fn encode_packet(&mut self, item: &[u8], dst: &mut BytesMut) {
        self.encode_packet_offset(item, dst, 0);
    }

    #[inline]
    fn encode_packet_offset(&mut self, item: &[u8], dst: &mut BytesMut, offset: usize) {
        let mut length = item.len();
        dst.reserve(length);

        let num = length / MAX_PAYLOAD_LEN;
        let remain = length % MAX_PAYLOAD_LEN;
        let mut offset = offset;

        for i in 0 .. num {
            dst.put_bytes(0, 4);
            dst.extend_from_slice(&item[i * MAX_PAYLOAD_LEN .. (i + 1) * MAX_PAYLOAD_LEN]);
            self.make_packet_header(MAX_PAYLOAD_LEN, dst, offset);

            offset += MAX_PAYLOAD_LEN + 4;
        }

        dst.put_bytes(0, 4);
        dst.extend_from_slice(&item[num * MAX_PAYLOAD_LEN ..]);

        self.make_packet_header(remain, dst, offset);
    }
}

impl CommonPacket for PacketCodec {
    #[inline]
    fn make_packet_header(&mut self, length: usize, data: &mut [u8], offset: usize) {
        // we have ensured length is 3bytes, so we can use unsafe block
        unsafe {
            let bytes = *(&(length as u64).to_le() as *const u64 as *const [u8; 8]);
            let data_ptr = data.as_mut_ptr().add(offset);
            copy_nonoverlapping(bytes.as_ptr(), data_ptr, 3);
        }

        self.set_seq_id(data, offset)
    }

    #[inline]
    fn reset_seq(&mut self) {
        self.seq = 0
    }

    fn get_session(&mut self) -> &mut ServerHandshakeCodec {
        &mut self.session
    }
}

impl Decoder for PacketCodec {
    type Item = BytesMut;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() || src.len() < 3 {
            return Ok(None);
        }

        let length = get_length(&*src) as usize;

        if 4 + length > src.len() {
            return Ok(None);
        }

        if length == MAX_PAYLOAD_LEN {
            self.is_max = true
        }

        self.seq = self.seq.wrapping_add(1);
        let _ = src.split_to(4);
        self.buf.extend_from_slice(&src.split_to(length));

        if length < MAX_PAYLOAD_LEN {
            return Ok(Some(self.buf.split()));
        }

        self.decode(src)

    }
}

pub enum SendType {
    Origin
}
pub enum PacketSend<T> {
    Origin(T),
    Encode(T),
    EncodeOffset(T, usize),
}

impl<T: AsRef<[u8]>> Encoder<PacketSend<T>> for PacketCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: PacketSend<T>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            PacketSend::Origin(data) => dst.extend_from_slice(data.as_ref()),
            PacketSend::Encode(data) => self.encode_packet(data.as_ref(), dst),
            PacketSend::EncodeOffset(data, offset) => self.encode_packet_offset(data.as_ref(), dst, offset),
        };

        Ok(())
    }
}

//impl Encoder<BytesMut> for PacketCodec {
//    type Error = ProtocolError;
//
//    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
//        self.encode_packet(&item, dst);
//        Ok(())
//    }
//}
//
//impl Encoder<&[u8]> for PacketCodec {
//    type Error = ProtocolError;
//
//    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
//        self.encode_packet(item, dst);
//        Ok(())
//    }
//}

#[inline]
pub fn make_eof_packet() -> [u8; 9] {
    let mut eof = [5, 0, 0, 0, 0xfe, 0, 0, 2, 0];
    eof
}

#[inline]
pub fn ok_packet() -> [u8; 11] {
    [7, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0]
}

#[cfg(test)]
mod test {
    use bytes::{BufMut, BytesMut};
    use futures::{SinkExt, StreamExt};
    use tokio::io::{AsyncWriteExt, AsyncReadExt};
    use tokio_util::codec::Framed;

    use super::PacketCodec;
    use crate::{mysql_const::MAX_PAYLOAD_LEN, server::codec::{PacketSend, CommonPacket}};

    #[tokio::test]
    async fn test_packetcodec_normal() {
        let packet = PacketCodec::new(8196);
        //let mut data = 16_u32.to_le_bytes()[0..3].to_vec();
        //data.put_u8(0);
        let data = vec![3; 16];

        let (client, server) = tokio::io::duplex(128);

        let mut framed = Framed::new(client, packet);
        let _ = framed.send(PacketSend::Encode(&data[..])).await;
        let mut parts = framed.into_parts();
        parts.io = server;
        let mut framed = Framed::from_parts(parts);

        let framed_data = framed.next().await.unwrap().unwrap();

        assert_eq!(data, framed_data);
    }

    #[tokio::test]
    async fn test_packetcodec_max() {
        let packet = PacketCodec::new(8196);
        let length: u32 = MAX_PAYLOAD_LEN as u32 * 2 + 16 + 12;

        let (client, mut server) = tokio::io::duplex(length as usize);

        let mut payload_data: Vec<u8> = vec![];
        payload_data.extend_from_slice(&vec![1; MAX_PAYLOAD_LEN]);
        payload_data.extend_from_slice(&vec![2; MAX_PAYLOAD_LEN]);
        payload_data.extend_from_slice(&vec![3; 16]);

        let mut framed = Framed::new(client, packet);
        let _ = framed.send(PacketSend::Encode(&payload_data[..])).await;
        let mut parts = framed.into_parts();
        parts.io = server;
        let mut framed = Framed::from_parts(parts);

        let framed_data = framed.next().await.unwrap().unwrap();

        assert_eq!(payload_data, framed_data);
    }

    #[test]
    fn test_encode_offset() {
        let packet = PacketCodec::new(8196);
        let (client, _) = tokio::io::duplex(100);

        let mut framed = Framed::new(client, packet);
        let mut buf = BytesMut::with_capacity(10);
        let data = vec![1,2,3,4,5];

        let length = buf.len();
        framed.codec_mut().encode_packet_offset(&data[..], &mut buf, length);

        let data = vec![7,8,9];
        let length = buf.len();
        framed.codec_mut().encode_packet_offset(&data[..], &mut buf, length);

        assert_eq!(&buf[..], &[5, 0, 0, 0, 1, 2, 3, 4, 5, 3, 0, 0, 1, 7, 8, 9])
    }
}
