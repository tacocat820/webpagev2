mod tokens;

use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use rustls::{ServerConfig, ServerConnection, Stream, pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}};
use rustls::server::Acceptor;

use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::time::Duration;

use std::collections::HashMap;
use std::fs::read_to_string;

pub trait Streamable: std::io::Read + std::io::Write {}
impl<T: ?Sized + std::io::Read + std::io::Write> Streamable for T {}

impl dyn Streamable + '_ {
    fn read_line(&mut self, s : &mut String) -> Result<usize, String> {
        let mut buf = [0; 1];
        let mut r = vec![];

        while buf != *b"\n" {
            match self.read(&mut buf) {
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
        } // SHUTDOWN WITH EVERYTHING ELSE
    } );

    let listener = TcpListener::bind(data.conf.addr.clone()).await?;
    println!("hello listener");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            socket.read(&mut [0; 1024]).await.expect("asdasdasda");
            if let Err(e) = socket.write_all(b"asdasdasd").await {
                eprintln!("failed to write to socket; err = {:?}", e);
                return;
            }
        });
    }
}
