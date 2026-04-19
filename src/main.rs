mod modules;
mod tokens;
mod ls;

use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

//use rustls::{ServerConfig, ServerConnection, Stream, pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}};
use tokio_rustls::rustls::{ServerConfig, 
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}
};
use tokio_rustls::server::TlsAcceptor;

use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;

use std::fs::read_to_string;
use std::fs::read;

//  TODO
// ContentType enum?
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

    async fn respond(&mut self, content : Vec<u8>, status : &str, content_type : Option<&str>) -> Result<(), String> {
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
            Err(e) => Err(e.to_string()),
        }
    }

    async fn respond_file(&mut self, path : &str, status : &str) -> Result<(), String> {
        let content = match read(path) {
            Ok(v) => v,
            Err(e) => { return Err(e.to_string()) },
        };
        match self.respond(content, status, content_type_from_ext(
                    match path.rsplit_once(".") {
                        Some(v) => v.1,
                        None => { return Err("Cannot split".to_string()); },
                    })
                ).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    } 
}

fn content_type_from_ext(ext : &str) -> Option<&str> {
    match ext {
        "png" => Some("image/png"),
        "apng" => Some("image/apng"),
        "gif" => Some("image/gif"),
        "ico" => Some("image/x-icon"),
        "css" => Some("text/css"),
        "html" => Some("text/html"),
        "js" => Some("text/javascript"),
        "txt" => Some("text/plain"),
        "pdf" => Some("application/pdf"),
        "json" => Some("application/json"),
        "mp3" => Some("audio/mpeg"),
        "mp4" => Some("video/mp4"),
        "gz" => Some("application/gzip"),
        _ => None,
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
        //let _ = handle(StreamableWrapper(Box::new(stream)), data).await;
        println!("{:?}", handle(StreamableWrapper(Box::new(stream)), data).await);

    } else {

        println!("{:?} insecure", httpscheck);
        let _ = handle(StreamableWrapper(Box::new(tcp_stream)), data).await;

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
        match key.to_lowercase().as_str() {
            "content-type" => { self.content_type = Some(v.to_string()); },
            "cookie" => { self.cookies = v.split(";").map(|s| {
                let a = s.split_once("=").unwrap_or(("", ""));
                (a.0.trim().to_string(), a.1.trim().to_string())
            }).collect(); },
            "content-length" => { self.content_length = Some(match v.trim().parse::<u64>() {
                Ok(v) => v,
                Err(e) => { return Err(format!("{}", e)); },
            }); },
            _ => {  }
        }
        Ok(())
    } 
}

async fn handle<T : Streamable>(mut stream : StreamableWrapper<T>, data : Arc<Data>) -> Result<(), String> {

    let mut act = String::new();
    let mut h = Headers::default();
    _ = stream.read_line(&mut act).await?;

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

    let act : Vec<&str> = act.split(' ').collect();
    if act.len() != 3 { return Err("Invalid act".to_string()); }

    let method = act.first().unwrap();
    let mut page : Vec<&str> = act.get(1).unwrap().split("/").collect();
    page.remove(0);
    
    println!("{:?}", page);

    match page[0] {
        "assets" => modules::assets::handle(&mut stream, data, &h, method, &page).await?,
        "project" => modules::projects::project(&mut stream, data, &h, method, &page).await?,
        "projects" => modules::projects::projects(&mut stream, data, &h, method, &page).await?,
        "blog" => modules::blog::handle(&mut stream, data, &h, method, &page).await?,
        "auth" => modules::auth::handle(&mut stream, data, &h, method, &page).await?,
        "mystuff" => modules::mystuff::handle(&mut stream, data, &h, method, &page).await?,
        _ => modules::home::handle(&mut stream, data, &h, method, &page).await?,
    }




    Ok(())
}