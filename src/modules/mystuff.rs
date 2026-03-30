use std::path::PathBuf;
use std::sync::Arc;

use crate::Data;
use crate::ls::ls;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;

use std::fs::read_to_string;
use std::fs;

pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, data : Arc<Data>, headers : &Headers, method : &str, page : &Vec<&str>) -> Result<(), String> {

    println!("{:?}", headers.cookies);

    if !headers.cookies.contains_key("token") || data.tokens.lock().await.get(headers.cookies.get("token").unwrap()).is_none() {
        stream.respond(vec![], "403 FORBIDDEN", None).await?;
        return Ok(())      
    }

    match method {
        "GET" => {
            if page.len() < 2 {
                stream.respond_file("assets/mystuff.html", "200 OK").await?;
            } else if page[1] == "drafts" {
                previews(stream, data, headers, method, page).await?;
            } else if page[1] == "editor" {
                stream.respond_file("assets/editor.html", "200 OK").await?;
            } else if page[1] == "draft" {
                stream.respond_file("assets/editor.html", "200 OK").await?;
            }
        },
        "POST" => {
            
        },
        _ => {}
    }

    
    Ok(())
    
}

pub async fn draft<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 3 {
        let content = fs::read("assets/404.png").unwrap();
        stream.respond(content, "400 BAD REQUEST", Some("text/plain")).await?;
        return Err("What is this".to_string());
    }

    let content = match preview(page[2]) {
        Ok(v) => v,
        Err(e) => {
            println!("{:#?} {}", e, page[2]);
            stream.respond(b"asdasd".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?;
            return Err("Cannot draft".to_string());
        },
    };

    stream.respond(match serde_json::to_string(&content) {
        Ok(v) => v.bytes().collect(),
        Err(e) => {
            println!("{:#?}", e);
            stream.respond(b"asdasd".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?;
            return Err("Cannot draft".to_string());     
        },
    }, "200 OK", Some("text/plain")).await?;


    Ok(())
}

#[derive(serde::Deserialize,Debug,serde::Serialize, Clone)]
pub struct Draft {
    name : String,
    img : String,
    date : String,
    id : Option<String>,
    pub content : Option<String>
}

pub fn preview(name : &str) -> Result<Draft, String> {
    let read = match read_to_string(format!("drafts/{}", name)) {
        Ok(v) => v,
        Err(e) => { return Err(format!("{}", e)) },
    };

    let mut info = &read[0..match read.find("-->") {
        Some(v) => v,
        None => { return Err(format!("Comment ending sequence not found")); },
    }];
    
    info = match info.strip_prefix("<!--") {
        Some(v) => v,
        None => { return Err(format!("Unable to strip comment prefix")); },
    };

    let mut parsed : Draft = match toml::from_str(info) {
        Ok(v) => v,
        Err(e) => { return Err(format!("Cannot parse as toml: {}", e)); },
    };
    parsed.id = Some(name.to_string());

    return Ok(parsed);
     

}

pub fn ls_drafts(left : usize, right : usize) -> Result<Vec<Draft>, String> {
    let list = ls(&PathBuf::from("drafts/"), PathBuf::from("drafts/"));
    
    let left = std::cmp::max(0, left);
    let right = std::cmp::min(list.len(), right); 

    //println!("{} {}", left, right);
    if left > right { return Ok(vec![]); }

    let inlist = &list[left .. right];



    let mut projects = vec![];

    for i in inlist.iter() {
        
        projects.push(match preview(i) {
            Ok(v) => v,
            Err(e) => { return Err(format!("Error fetching project {} : {}", i, e)); },
        });
        
    }    
    return Ok(projects);
    //let list = 
}


pub async fn previews<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    let range : Vec<&str> = page[2].splitn(2, "-").collect();

    let left : usize = match match range.get(0) {
            Some(v) => v,
            None => {
                stream.respond(b"Invalid range".to_vec(), "400 Bad Request", Some("text/plain")).await?; return Err("Bad request".to_string()); 
            },
        }.parse() {
        Ok(v) => v,
        Err(_e) => { 
            stream.respond(b"Invalid range".to_vec(), "400 Bad Request", Some("text/plain")).await?; return Err("Bad request".to_string());
        },
    };
    let right : usize = match match range.get(1) {
            Some(v) => v,
            None => {
                stream.respond(b"Invalid range".to_vec(), "400 Bad Request", Some("text/plain")).await?; return Err("Bad request".to_string()); 
            },
        }.parse() {
        Ok(v) => v,
        Err(_e) => {
            stream.respond(b"Invalid range".to_vec(), "400 Bad Request", Some("text/plain")).await?; return Err("Bad request".to_string()); 
        },
    };
    
    let p = match ls_drafts(left, right) {
        Ok(v) => v,
        Err(e) => {
            stream.respond(b"Cannot ls projects".to_vec(), "500 Internal Server Error", Some("text/plain")).await?; return Err("Cannot ls projects".to_string());
        },
    };
    let json = match serde_json::to_string(&p) {
        Ok(v) => v,
        Err(e) => {
            stream.respond(b"Cannot convert to json".to_vec(), "500 Internal Server Error", Some("text/plain")).await?; return Err("Cannot ls projects".to_string());
        },
    };

    //println!("{}", json);
    
    stream.respond(json.as_bytes().to_vec(), "200 OK", Some("application/json")).await?;

    Ok(())
}