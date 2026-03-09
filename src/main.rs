mod tokens;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream, tcp};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

//use rustls::{ServerConfig, ServerConnection, Stream, pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}};
use tokio_rustls::rustls::{ServerConfig, ServerConnection, Stream, 
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}
};
use tokio_rustls::server::TlsAcceptor;

use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::time::Duration;

use std::collections::HashMap;
use std::fs::read_to_string;

pub trait Streamable: AsyncReadExt + AsyncWriteExt + std::marker::Unpin {}
impl<T: ?Sized + AsyncReadExt + AsyncWriteExt + std::marker::Unpin> Streamable for T {}
pub struct StreamableWrapper<T: Streamable>(Box<T>);

impl<T : AsyncReadExt + AsyncWriteExt + std::marker::Unpin> StreamableWrapper<T> {
    async fn read_line(&mut self, s : &mut String) -> Result<usize, String> {
        let mut buf = [0; 1];
        let mut r = vec![];

        while buf != *b"\n" {
            match self.0.read(&mut buf).await {
                Ok(v) => { if v == 0 { break; } },
                Err(e) => { return Err(format!("{}", e)) },
            }
            r.push(buf[0]);
        }

        let len = r.len();

        *s = match String::from_utf8(r) {
            Ok(v) => v,
            Err(e) => { return Err(format!("{}", e)) },
        };

        return Ok(len);
    } 
}
impl<T: Streamable> std::ops::Deref for StreamableWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl<T: Streamable> std::ops::DerefMut for StreamableWrapper<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

struct Data {
    conf : Config,
    tokens : Mutex<tokens::Tokens>
}

#[derive(Deserialize)]
struct Config {
    addr : String,
    passwd : String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let data = Arc::new(Data {
        conf: toml::from_str(
            &read_to_string("conf.toml").expect("can't read conf.toml"))
            .expect("can't load conf.toml"),
        tokens: Mutex::new(tokens::Tokens::new())
    });

    let certs = CertificateDer::pem_file_iter("./cert.crt").expect("cannot load certs")
        .map(|c| c.expect("cannot laod certificate")).collect();
    let prkey = PrivateKeyDer::from_pem_file("./private.key").expect("cannot load private key");

    let conf = Arc::new(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, prkey)
        .expect("cannot create server config"));

    let cdata = data.clone();
    tokio::spawn(async move {
        loop {
            cdata.tokens.lock().await.cleanup();
            tokio::time::sleep(Duration::from_hours(1)).await;
        } // TODO SHUTDOWN WITH EVERYTHING ELSE
    } );

    let listener = TcpListener::bind(data.conf.addr.clone()).await?;
    let acceptor = Arc::new(TlsAcceptor::from(conf.clone()));

    println!("hello listener");

    loop {
        let (tcp_stream, _) = listener.accept().await?;

        let data = data.clone();
        let accp = acceptor.clone();
        tokio::spawn(async move {
            socket(tcp_stream, data, accp).await;
        });
    }
}

async fn socket(mut tcp_stream : TcpStream, data : Arc<Data>, accp : Arc<TlsAcceptor>) {

    println!("hello client");

    //tcp_stream.read(&mut [0; 1024]).await.expect("asdasdasda");

    let mut httpscheck : [u8; 1] = [0];
    _ = match tcp_stream.peek(&mut httpscheck).await {
        Ok(v) => v,
        Err(e) => { eprintln!("{}", e); return; },
    };

    if httpscheck == [22] {

        println!("{:?} secure", httpscheck);

        let stream = match accp.accept(tcp_stream).await {
            Ok(v) => v,
            Err(_e) => { return; },
        };
        handle(StreamableWrapper(Box::new(stream))).await;

    } else {

        println!("{:?} insecure", httpscheck);
        handle(StreamableWrapper(Box::new(tcp_stream))).await;

    }


}

struct Headers {
    content_type : String   
}

async fn handle<T : Streamable>(mut stream : StreamableWrapper<T>) {
    let mut act = String::new();
    match stream.read_line(&mut act).await {
        Ok(_) => {},
        Err(e) => { println!("{}", e); return; },
    }
    
    println!("{}", act);
    stream.write(b"asd").await.unwrap();
    stream.flush().await.unwrap();
    println!("goodbye");
}