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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;

use std::fs::read_to_string;

//  TODO
// ContentType enum?
// Headers struct?
// also ResponseHeaders

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
                Err(e) => { return Err(e.to_string()) },
            }
            r.push(buf[0]);
        }

        let len = r.len();

        *s = match String::from_utf8(r) {
            Ok(v) => v,
            Err(e) => { return Err(e.to_string()) },
        };

        Ok(len)
    } 

    async fn respond(&mut self, content : Vec<u8>, status : &str, content_type : Option<&str>) -> Result<(), std::io::Error> {
        let length = content.len();
        let content_type = match content_type {
            Some(v) => format!("Content-Type: {}\r\n", v),
            None => String::new()
        };
    
        let response = format!("HTTP/1.1 {}\r\nContent-Length: {}\r\n{}\r\n", status, length, content_type);
        //let response = "".to_string();
        let responsebytes = response.as_bytes();

        let reply = [responsebytes, &content].concat(); 
        match self.write_all(&reply).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
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

async fn socket(tcp_stream : TcpStream, data : Arc<Data>, accp : Arc<TlsAcceptor>) {

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
        let _ = handle(StreamableWrapper(Box::new(stream))).await;

    } else {

        println!("{:?} insecure", httpscheck);
        let _ = handle(StreamableWrapper(Box::new(tcp_stream))).await;

    }


}

#[derive(Default, Debug)]
struct Headers {
    content_type : Option<String>,
    cookies : HashMap<String, String>,
    content_length : Option<u64>,
}
impl Headers {
    fn set_from_str(&mut self, key : &str, v : &str) -> Result<(), String> {
        match key {
            "content-type" => { self.content_type = Some(v.to_string()); },
            "cookies" => { self.cookies = v.split(";").map(|s| {
                match s.split_once("=") {
                    Some(v) => (v.0.to_string(), v.1.to_string()),
                    None => { ("".to_string(), "".to_string()) },
                }
            }).collect(); },
            "content-length" => { self.content_length = Some(match v.parse::<u64>() {
                Ok(v) => v,
                Err(e) => { return Err(format!("{}", e)); },
            }); },
            _ => {  }
        }
        Ok(())
    } 
}

async fn handle<T : Streamable>(mut stream : StreamableWrapper<T>) -> Result<(), String> {
    let mut act = String::new();
    let mut h = Headers::default();
    _ = stream.read_line(&mut act).await?;
    

    match stream.respond(b"fuck".to_vec(), "200 OK", Some("text/plain")).await {
        Ok(_) => {},
        Err(e) => { return Err(e.to_string()) },
    };

    loop {
        let mut l = String::new();

        let len = stream.read_line(&mut l).await?;

        if len == 0 { break; }
        if l.trim().is_empty() { break; }

        if let Some((key, v)) = l.split_once(":") {
            h.set_from_str(key, v)?;
        } 
    }

    

    println!("{:#?}", h);



    Ok(())
}