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

use std::{cmp::Ordering, ptr::copy_nonoverlapping};

use byteorder::{ByteOrder, LittleEndian};
use bytes::{Buf, BufMut, BytesMut};
use chrono::prelude::*;
use crypto::{self, digest::Digest};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::mysql_const::{EOF_HEADER, OK_HEADER};

// random_buf: generate random byte vector
#[inline]
pub fn random_buf(size: i64) -> Vec<u8> {
    let mut buf = vec![];
    let mut r = StdRng::seed_from_u64(Utc::now().timestamp_subsec_nanos().into());
    let mut i: usize = 0;

    while i < size as usize {
        buf.push(r.gen_range(0..127));
        if buf[i] == 0 || buf[i] as char == '$' {
            buf[i] += 1;
        }
        i += 1;
    }
    buf
}

// calc_password: Hash password use sha1
pub fn calc_password(scramble: &[u8], password: &[u8]) -> Vec<u8> {
    if password.is_empty() {
        return vec![];
    }
    let mut crypt = crypto::sha1::Sha1::new();
    crypt.input(password);
    let mut stage1 = vec![0; 20];
    crypt.result(&mut stage1);

    crypt.reset();
    crypt.input(&stage1);
    let mut hash = vec![0; 20];
    crypt.result(&mut hash);

    crypt.reset();
    crypt.input(scramble);
    crypt.input(&hash);
    let mut scramble = vec![0; 20];
    crypt.result(&mut scramble);

    for i in 0..20 {
        scramble[i as usize] ^= stage1[i]
    }
    scramble
}

// calc_caching_sha2password: Hash password using MySQL 8+ method (SHA256)
pub fn calc_caching_sha2password(scramble: &[u8], password: &[u8]) -> Vec<u8> {
    if password.is_empty() {
        return vec![];
    }

    let mut crypt = crypto::sha2::Sha256::new();
    crypt.input(password);
    let mut message1 = vec![0; 32];
    crypt.result(&mut message1);

    crypt.reset();
    crypt.input(&message1);
    let mut message1_hash = vec![0; 32];
    crypt.result(&mut message1_hash);

    crypt.reset();
    crypt.input(&message1_hash);
    crypt.input(scramble);
    let mut message2 = vec![0; 32];
    crypt.result(&mut message2);

    for i in 0..32 {
        message1[i as usize] ^= message2[i];
    }

    message1
}

pub fn compare(a: &[u8], b: &[u8]) -> bool {
    for (ai, bi) in a.iter().zip(b.iter()) {
        match ai.cmp(bi) {
            Ordering::Equal => continue,
            _ => return false,
        }
    }

    /* if every single element was equal, compare length */
    a.len().cmp(&b.len()) == Ordering::Equal
}

#[inline]
pub fn length_encode_int(data: &[u8]) -> (u64, bool, u64) {
    match data[0] {
        0xfb => (0, true, 1),
        0xfc => (LittleEndian::read_uint(data, 2), false, 3),
        0xfd => (LittleEndian::read_uint(data, 3), false, 4),
        0xfe => (LittleEndian::read_uint(data, 8), false, 9),
        x => (x as u64, false, 1),
    }
}

pub trait BufExt: Buf {
    fn get_lenc_int(&mut self) -> (u64, bool, u64) {
        let first = self.get_u8();
        match first {
            0xfb => (0, true, 1),
            0xfc => (self.get_uint_le(2), false, 3),
            0xfd => (self.get_uint_le(3), false, 4),
            0xfe => (self.get_uint_le(8), false, 9),
            _ => (first as u64, false, 1),
        }
    }

    fn get_lenc_str_bytes(&mut self) -> (Vec<u8>, bool);
}

// Implemet BufExt
impl BufExt for BytesMut {
    fn get_lenc_str_bytes(&mut self) -> (Vec<u8>, bool) {
        let (num, is_null, _) = self.get_lenc_int();

        if num < 1 {
            return (vec![num as u8], is_null);
        }

        if !self.has_remaining() {
            return (vec![], false);
        }

        (self.split_to(num as usize).to_vec(), is_null)
    }
}

pub trait BufMutExt: BufMut {
    fn put_lenc_int(&mut self, n: u64) {
        if n <= 250 {
            self.put_u8(n as u8);
        } else if n <= 0xffff {
            self.put_u8(0xfc);
            self.put_uint(n, 2);
        } else if n <= 0xffffff {
            self.put_u8(0xfd);
            self.put_uint_le(n, 3);
        } else {
            self.put_u8(0xfe);
            self.put_uint_le(n, 8);
        }
    }
}

impl BufMutExt for Vec<u8> {}

pub fn length_encoded_string(data: &mut BytesMut) -> (Vec<u8>, bool) {
    let (num, is_null, pos) = length_encode_int(data);

    let _ = data.split_to(pos as usize);

    if num < 1 {
        return (vec![0xfb], is_null);
    }

    if data.is_empty() {
        return (vec![], false);
    }

    (data.split_to(num as usize).to_vec(), false)
}

#[inline]
pub fn is_eof(data: &[u8]) -> bool {
    data.len() < 9 + 4 && *unsafe { data.get_unchecked(4) } == EOF_HEADER
}

#[inline]
pub fn is_ok(data: &[u8]) -> bool {
    data.len() > 7 + 4 && *unsafe { data.get_unchecked(4) } == OK_HEADER
}

#[inline]
pub fn get_length(buf: &[u8]) -> usize {
    let mut out = 0u64;
    let ptr_out = &mut out as *mut u64 as *mut u8;
    unsafe {
        copy_nonoverlapping(buf.as_ptr(), ptr_out, 3);
    }
    out.to_le() as usize
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;

    use super::{length_encoded_string, BufExt};

    #[test]
    fn test_length_enc_string() {
        let data = [0x04, 0x55, 0x73, 0x65, 0x72];
        let mut buf = BytesMut::from(&data[..]);

        let (info, _is_null) = length_encoded_string(&mut buf);
        let name = std::str::from_utf8(&info).unwrap();
        assert_eq!(name, "User");
    }

    #[test]
    fn test_buf_length_enc_string() {
        let data = [0x04, 0x55, 0x73, 0x65, 0x72];
        let mut buf = BytesMut::from(&data[..]);

        let (info, _is_null) = buf.get_lenc_str_bytes();
        let name = std::str::from_utf8(&info).unwrap();
        assert_eq!(name, "User");
    }
}
