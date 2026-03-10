use std::path::PathBuf;
use std::sync::Arc;

use crate::Data;
use crate::ls::ls;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;

pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 2 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }
    let path = page[1..].join("/");

    let dir = ls(&PathBuf::from("assets/"), PathBuf::from("assets/"));
    println!("{:?}", path);

    if !dir.contains(&path) { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }

    stream.respond_file(&format!("assets/{}", &path), "200 OK").await?;
    
    Ok(())
    
}