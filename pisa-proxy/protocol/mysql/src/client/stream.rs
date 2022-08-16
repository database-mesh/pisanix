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
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_native_tls::{native_tls::TlsConnector, TlsStream};

use crate::err::ProtocolError;

#[pin_project(project=LSProj)]
#[derive(Debug)]
pub enum LocalStream {
    Plain(Option<TcpStream>),
    Secure(TlsStream<TcpStream>),
}

impl LocalStream {
    //pub fn new(wrapper: StreamWrapper) -> LocalStream {
    //    LocalStream { wrapper }
    //}

    pub async fn close(&mut self) {
        match self {
            Self::Plain(ref mut stream) => {
                stream.take().unwrap();
            }
            _ => unreachable!(),
        }
    }

    pub async fn make_tls(&mut self) -> Result<(), ProtocolError> {
        let tlsconn = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .use_sni(false)
            .build()?;

        match self {
            Self::Plain(ref mut try_plain) => {
                let connector = tokio_native_tls::TlsConnector::from(tlsconn);
                let plain_stream = try_plain.take().unwrap();

                let tls_stream = connector
                    .connect(&plain_stream.peer_addr().unwrap().to_string(), plain_stream)
                    .await
                    .unwrap();

                LocalStream::from(tls_stream)
            }

            _ => unreachable!(),
        };

        Ok(())
    }
}

impl AsyncRead for LocalStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), tokio::io::Error>> {
        let mut this = self.project();

        match this {
            LSProj::Plain(ref mut stream) => {
                Pin::new(stream.as_mut().unwrap()).poll_read(cx, buf)
            }

            LSProj::Secure(ref mut stream) => Pin::new(stream).as_mut().poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for LocalStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, tokio::io::Error>> {
        let mut this = self.project();

        match this {
            LSProj::Plain(ref mut stream) => {
                Pin::new(stream.as_mut().unwrap()).poll_write(cx, buf)
            }

            LSProj::Secure(ref mut stream) => Pin::new(stream).as_mut().poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
        let mut this = self.project();

        match this {
            LSProj::Plain(ref mut stream) => {
                Pin::new(stream.as_mut().unwrap()).poll_flush(cx)
            }

            LSProj::Secure(ref mut stream) => Pin::new(stream).as_mut().poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
        let mut this = self.project();

        match this {
            LSProj::Plain(ref mut stream) => {
                Pin::new(stream.as_mut().unwrap()).poll_shutdown(cx)
            }

            LSProj::Secure(ref mut stream) => Pin::new(stream).as_mut().poll_shutdown(cx),
        }
    }
}

impl From<TcpStream> for LocalStream {
    fn from(stream: TcpStream) -> Self {
        LocalStream::Plain(Some(stream))
    }
}

impl From<TlsStream<TcpStream>> for LocalStream {
    fn from(stream: tokio_native_tls::TlsStream<TcpStream>) -> Self {
        LocalStream::Secure(stream)
    }
}

impl LocalStream {
    pub fn get_inner(&self) -> &TcpStream {
        match self {
            Self::Plain(stream) => {
                stream.as_ref().unwrap()
            },

            Self::Secure(stream) => {
                stream.get_ref().get_ref().get_ref()
            }
        }
    }
}

