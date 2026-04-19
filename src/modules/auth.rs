use std::sync::Arc;

use crate::Data;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;


pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, data : Arc<Data>, headers : &Headers, method : &str, page : &Vec<&str>) -> Result<(), String> {

    println!("hello tk");

    match method {
        "GET" => stream.respond_file("assets/auth.html", "200 OK").await?,
        "POST" => {

            println!("hello tk");
       
            let len = headers.content_length.unwrap_or(0) as usize;
            println!("len {:?}", len);

            let mut buf = vec![0; len];
            _ = match stream.read_exact(&mut buf).await {
                Ok(v) => v,
                Err(e) => { return Err(e.to_string()); },
            };
            let tk = String::from_utf8(buf[0..len].to_vec());
            
            println!("tk : {:#?}", tk);

            if tk.is_ok() && tk.unwrap() == *data.conf.passwd {
                let new = data.tokens.lock().await.new_token();
                
                stream.respond(new.bytes().collect(), "200 OK", Some("text/plain")).await?;
            } else {
                stream.respond(vec![], "403 FORBIDDEN", None).await?;
            }
 
        }
        _ => {
            
        }
    }

    Ok(())
    
}