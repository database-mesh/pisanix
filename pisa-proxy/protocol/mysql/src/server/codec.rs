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

use bytes::{BufMut, BytesMut};
use futures::SinkExt;
use tokio_util::codec::{Decoder, Encoder};

use crate::{err::ProtocolError, mysql_const::*, util::get_length};

/// Used to reading packet from client side
pub struct PacketCodec {
    buf: BytesMut,
    is_complete: bool,
    // Whether the payload is greater than MAX_PAYLOAD_LEN
    is_max: bool,
    seq: u8,
}

impl PacketCodec {
    pub fn new(init_size: usize) -> Self {
        Self { buf: BytesMut::with_capacity(init_size), is_complete: false, is_max: false, seq: 0 }
    }

    #[inline]
    pub fn make_packet_header(&mut self, length: usize, data: &mut [u8], offset: usize) {
        // we have ensured length is 3bytes, so we can use unsafe block
        unsafe {
            let bytes = *(&(length as u64).to_le() as *const u64 as *const [u8; 8]);
            let data_ptr = data.as_mut_ptr().add(offset);
            copy_nonoverlapping(bytes.as_ptr(), data_ptr, 3);
        }

        self.set_seq_id(data, offset)
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
        let mut length = item.len();
        dst.reserve(length);

        let mut idx: usize = 0;
        while length >= MAX_PAYLOAD_LEN + 4 {
            dst.extend_from_slice(&item[idx..idx + MAX_PAYLOAD_LEN + 4]);
            self.make_packet_header(MAX_PAYLOAD_LEN, dst, idx);

            length -= MAX_PAYLOAD_LEN + 4;
            idx += MAX_PAYLOAD_LEN + 4;
        }

        dst.extend_from_slice(&item[idx..]);
        self.make_packet_header(length - 4, dst, idx);
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

        self.seq = self.seq.wrapping_add(1);

        if length < MAX_PAYLOAD_LEN {
            self.is_complete = true;
            if self.is_max == false {
                return Ok(Some(src.split()));
            } else {
                self.is_max = false;
                self.buf.extend_from_slice(&src.split());
                return Ok(Some(self.buf.split()));
            }
        }

        self.is_max = true;

        self.buf.extend_from_slice(&src.split_to(length + 4));

        src.reserve(MAX_PAYLOAD_LEN);

        self.decode(src)
    }
}

impl Encoder<BytesMut> for PacketCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.encode_packet(&item, dst);
        Ok(())
    }
}

impl Encoder<&[u8]> for PacketCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.encode_packet(item, dst);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BufMut;
    use futures::{SinkExt, StreamExt};
    use tokio::io::AsyncWriteExt;
    use tokio_util::codec::Framed;

    use super::PacketCodec;
    use crate::mysql_const::MAX_PAYLOAD_LEN;

    #[tokio::test]
    async fn test_packetcodec_normal() {
        let packet = PacketCodec::new(8196);
        let mut data = 16_u32.to_le_bytes()[0..3].to_vec();
        data.put_u8(0);
        data.extend_from_slice(&vec![1; 16]);

        let (client, server) = tokio::io::duplex(128);

        let mut framed = Framed::new(client, packet);
        let _ = framed.send(&data[..]).await;
        let mut parts = framed.into_parts();
        parts.io = server;
        let mut framed = Framed::from_parts(parts);

        let framed_data = framed.next().await.unwrap().unwrap();

        assert_eq!(data, framed_data);
    }

    #[tokio::test]
    async fn test_packetcodec_max() {
        let packet = PacketCodec::new(8196);
        let length: u32 = MAX_PAYLOAD_LEN as u32 * 2 + 16;

        let (client, server) = tokio::io::duplex((length + 12) as usize);

        let mut data = MAX_PAYLOAD_LEN.to_le_bytes()[0..3].to_vec();
        data.put_u8(0);
        data.extend_from_slice(&vec![0; MAX_PAYLOAD_LEN]);

        data.extend_from_slice(&MAX_PAYLOAD_LEN.to_le_bytes()[0..3]);
        data.put_u8(1);
        data.extend_from_slice(&vec![0; MAX_PAYLOAD_LEN]);

        data.extend_from_slice(&16_u32.to_le_bytes()[0..3]);
        data.put_u8(2);
        data.extend_from_slice(&vec![0; 16]);

        let mut framed = Framed::new(client, packet);
        let _ = framed.send(&data[..]).await;
        let mut parts = framed.into_parts();
        parts.io = server;
        let mut framed = Framed::from_parts(parts);

        let framed_data = framed.next().await.unwrap().unwrap();

        assert_eq!(data, framed_data);
    }
}
