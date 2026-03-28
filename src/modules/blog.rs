use std::path::PathBuf;
use std::sync::Arc;

use crate::Data;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;
use crate::ls::ls;
use crate::modules::projects::Button;

use std::fs::read_to_string;

pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 2 { 
        stream.respond_file("assets/blog.html", "200 OK").await?; 
    } else if page.len() == 3 && page[1] == "post" {
        post(stream, _data, _headers, _method, page).await?;
    } else if page.len() == 4 && page[1] == "previews" {
        previews(stream, _data, _headers, _method, page).await?;
    }
    Ok(())
    
}

pub async fn post<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 2 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }
    let path = page[2..].join("/") + ".html";

    let dir = ls(&PathBuf::from("blog/"), PathBuf::from("blog/"));
    println!("{:?}", path);

    if !dir.contains(&path) { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }

    let template = read_to_string("assets/project_template.html").unwrap();
    let content = read_to_string(format!("blog/{}", &path)).unwrap();

    let res = template.replace("%EVERYTHING%", &content);

    stream.respond(res.bytes().collect(), "200 OK", Some("text/html")).await?;
    
    Ok(())
    
}


#[derive(serde::Deserialize,Debug,serde::Serialize, Clone)]
pub struct Post {
    name : String,
    img : String,
    date : String,
    id : Option<String>,
    pub content : Option<String>,
    buttons : Option<Vec<Button>>
}

pub fn preview(name : &str) -> Result<Post, String> {
    let read = match read_to_string(format!("blog/{}.html", name)) {
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

    let mut parsed : Post = match toml::from_str(info) {
        Ok(v) => v,
        Err(e) => { return Err(format!("Cannot parse as toml: {}", e)); },
    };
    parsed.id = Some(name.to_string());

    return Ok(parsed);
     

}

pub fn posts(list : &str, left : usize, right : usize) -> Result<Vec<Post>, String> {
    let r = match read_to_string(format!("blog/{}.posts", list)) {
        Ok(v) => v,
        Err(e) => { return Err(format!("{}", e)); },
    };
    let list : Vec<&str> = r.lines().collect();

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

    if page.len() < 4 {
        stream.respond(b"What are you doing?".to_vec(), "400 Bad Request", Some("text/plain")).await?; return Err("Bad request".to_string());
    }
    
    let list = page[2];
    let range : Vec<&str> = page[3].splitn(2, "-").collect();

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
    
    let p = match posts(list, left, right) {
        Ok(v) => v,
        Err(e) => {
            stream.respond(b"Cannot ls posts".to_vec(), "500 Internal Server Error", Some("text/plain")).await?; return Err("Cannot ls projects".to_string()); 
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