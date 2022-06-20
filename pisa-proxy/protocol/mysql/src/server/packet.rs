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

use std::{
    io::{Error, ErrorKind},
    ptr::copy_nonoverlapping,
};

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};

use super::stream::LocalStream;
use crate::{mysql_const::*, server::err::MySQLError, util::get_length};

#[derive(Debug)]
pub struct Packet {
    pub sequence: u8,
    pub conn: BufStream<LocalStream>,
    pub header: [u8; 4],
}

impl Packet {
    pub fn new(conn: BufStream<LocalStream>) -> Self {
        Packet { sequence: 0, conn, header: [0; 4] }
    }

    #[inline]
    pub async fn read_header(&mut self) -> Result<usize, Error> {
        let size = self.conn.read(&mut self.header).await?;

        // if no data in the header buffer, means that connection is
        // closed successful, otherwise means that the peer has closed
        if size == 0 {
            if self.header.iter().all(|&x| x == 0) {
                return Ok(0);
            } else {
                return Err(Error::new(ErrorKind::ConnectionReset, "Connection reset by peer"));
            }
        }

        let sequence = unsafe { *self.header.get_unchecked(3) };

        if sequence != self.sequence {
            return Err(Error::new(ErrorKind::Other, "invalid sequence"));
        }

        self.sequence = self.sequence.wrapping_add(1);

        let length = get_length(&self.header);
        Ok(length as usize)
    }

    pub async fn read_packet_buf(&mut self, buf: &mut BytesMut) -> Result<usize, Error> {
        let length = self.read_header().await?;
        if length == 0 {
            return Ok(0);
        }

        if length < MAX_PAYLOAD_LEN {
            let add = length - buf.len();
            if add > 0 {
                buf.reserve(add);
            }

            unsafe {
                buf.set_len(length);
            }

            let _ = self.conn.read_exact(buf).await?;
            return Ok(buf.len());
        }

        let mut tmp_buf = BytesMut::new();

        loop {
            let length = self.read_header().await?;
            tmp_buf.resize(length, 0);
            self.conn.read_exact(&mut tmp_buf).await?;

            buf.extend_from_slice(&tmp_buf);

            if length < MAX_PAYLOAD_LEN {
                break;
            }
        }

        Ok(buf.len())
    }

    // write_packet: write packet to socket
    pub async fn construct_packet_buf(&mut self, data: &mut BytesMut, buf: &mut BytesMut) {
        let mut length: usize = data.len() - 4;
        while length >= MAX_PAYLOAD_LEN as usize {
            data[0] = 0xff;
            data[1] = 0xff;
            data[2] = 0xff;

            data[3] = self.sequence;
            // Reset to 0 while exceed 255
            self.sequence = self.sequence.wrapping_add(1);

            buf.extend_from_slice(&data.split_to(MAX_PAYLOAD_LEN + 4));

            length -= MAX_PAYLOAD_LEN;
        }

        self.make_packet_header(length, data);

        buf.extend_from_slice(data);
    }

    #[inline]
    pub fn make_packet_header(&mut self, length: usize, data: &mut [u8]) {
        // we have confirmed length is 3bytes, so we can use unsafe block
        unsafe {
            let bytes = *(&(length as u64).to_le() as *const u64 as *const [u8; 8]);
            let data_ptr = data.as_mut_ptr();
            copy_nonoverlapping(bytes.as_ptr(), data_ptr, 3);
        }

        self.set_seq_id(data)
    }

    pub async fn write_buf(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        let _ = self.conn.write_all_buf(&mut buf).await;
        self.conn.flush().await
    }

    #[inline]
    pub async fn write_ok(&mut self) -> Result<(), Error> {
        let mut data = [7, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0];
        self.set_seq_id(&mut data);
        self.write_buf(&data).await
    }

    #[inline]
    pub fn make_eof_packet(&mut self) -> [u8; 9] {
        let mut eof = [5, 0, 0, 0, 0xfe, 0, 0, 2, 0];
        self.set_seq_id(&mut eof);
        eof
    }

    #[inline]
    pub async fn write_eof(&mut self) -> Result<(), Error> {
        let mut eof = [5, 0, 0, 0, 0xfe, 0, 0, 2, 0];
        self.set_seq_id(&mut eof);
        self.write_buf(&eof).await
    }

    #[inline]
    pub fn make_err_packet(&mut self, err: MySQLError) -> Vec<u8> {
        let mut data = BytesMut::with_capacity(128);
        data.extend_from_slice(&[0; 4]);
        data.put_u8(0xff);
        data.extend_from_slice(&[err.code as u8, (err.code >> 8) as u8]);
        data.put_u8(b'#');
        data.extend_from_slice(&err.state);
        data.put_u8(b' ');
        data.extend_from_slice(err.msg.as_bytes());

        self.make_packet_header(data.len() - 4, &mut data);

        data.to_vec()
    }

    #[inline]
    fn set_seq_id(&mut self, buf: &mut [u8]) {
        unsafe {
            let data_ptr = buf.as_mut_ptr();
            *data_ptr.add(3) = self.sequence;
        }

        self.sequence = self.sequence.wrapping_add(1)
    }

    pub async fn write_auth_switch_request(
        &mut self,
        salt: &[u8],
        plugin_name: String,
    ) -> Result<(), Error> {
        let mut data = BytesMut::with_capacity(128);
        data.extend_from_slice(&[0; 4]);
        data.put_u8(EOF_HEADER);
        data.extend_from_slice(plugin_name.as_bytes());
        data.put_u8(0);
        data.extend_from_slice(salt);
        data.put_u8(0);

        self.make_packet_header(data.len() - 4, &mut data);
        self.write_buf(&data).await
    }
}
