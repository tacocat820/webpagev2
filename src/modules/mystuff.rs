use std::path::PathBuf;
use std::sync::Arc;

use crate::Data;
use crate::ls::ls;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;

use std::fs::read_to_string;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;

pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, data : Arc<Data>, headers : &Headers, method : &str, page : &Vec<&str>) -> Result<(), String> {

    println!("{:?}", headers.cookies);

    if !headers.cookies.contains_key("token") || data.tokens.lock().await.get(headers.cookies.get("token").unwrap()).is_none() {
        stream.respond_file("assets/auth.html", "403 FORBIDDEN").await?;
        //stream.respond(vec![], "403 FORBIDDEN", None).await?;
        return Ok(())      
    }

    match method {
        "GET" => {
            if page.len() < 2 {
                stream.respond_file("assets/mystuff.html", "200 OK").await?;
            } else if page[1] == "drafts" {
                previews(stream, data, headers, method, page).await?;
            } else if page[1] == "draft" {
                load(stream, data, headers, method, page).await?;
            } else if page[1] == "editor" {
                stream.respond_file("assets/editor.html", "200 OK").await?;
            } else if page[1] == "upload" {
                stream.respond_file("assets/upload.html", "200 OK").await?;
            } else if page[1] == "delete" {
                delete(stream, data, headers, method, page).await?;
            }
        },
        "POST" => {
            if page.len() < 2 {
                stream.respond(b"gex".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?;
            } else if page[1] == "upload" {
                upload(stream, data, headers, method, page).await?;
            } else if page[1] == "post" {
                post(stream, data, headers, method, page).await?;
            } else if page[1] == "draft" {
                mkdraft(stream, data, headers, method, page).await?;
            }
            
        },
        _ => {}
    }

    
    Ok(())
    
}

pub async fn delete<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 3 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("Which one?".to_string());
    }

    let name = page[2];

    match tokio::fs::remove_file(format!("drafts/{}", name)).await {
        Ok(_) => {},
        Err(_) => { stream.respond(b"No such file".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Ok(()); },
    }

    stream.respond(format!("removed {}", name).bytes().collect(), "200 OK", Some("text/plain")).await?;
    Ok(())
}

pub async fn post<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 3 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("Where?".to_string());
    }

    let name = page[2];

    let mut len = match headers.content_length {
        Some(v) => v as usize,
        None => { return Err("No file".to_string()); },
    };
    match tokio::fs::write(format!("blog/{}.html", name), "").await {
        Ok(_) => {},
        Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("okary".to_string()); },
    }

    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(format!("blog/{}.html", name))
        .await.expect("Unable to open file");


    while len > 0 {
        let mut buf = [0; 30000];
        let a = match stream.read(&mut buf).await {
            Ok(v) => v,
            Err(e) => { println!("{:#?}", e); return Err("error readig file".to_string()); },
        };
        if a == 0 { stream.respond(b"asadas".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?; return Err("bad request".to_string()); }

        match file.write_all(&buf[0..a]).await {
            Ok(_) => {},
            Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; },
        }
        len -= a;
    }

    let mut f =  match tokio::fs::File::open("blog/all.posts").await {
        Ok(v) => v,
        Err(e) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("Cannot open all.posts".to_string()); },
    };
    let mut c : Vec<u8> = format!("{}\n", name).bytes().collect();
    match f.read_to_end(&mut c).await {
        Ok(v) => v,
        Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("Cannot read all.posts".to_string()); },
    };

    let mut f = match tokio::fs::File::create("blog/all.posts").await {
        Ok(v) => v,
        Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("Cannot create all.posts".to_string()); },
    };
    match f.write_all(&c.as_slice()).await {
        Ok(_) => {},
        Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("Cannot write to all.posts".to_string()); },
    };

    println!("fl : {:?}", name);

    stream.respond(format!("{}", name).bytes().collect(), "200 OK", Some("text/plain")).await?;
    Ok(())
}

pub async fn mkdraft<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    if page.len() < 3 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("Where?".to_string());
    }

    let name = page[2];

    let mut len = match headers.content_length {
        Some(v) => v as usize,
        None => { return Err("No file".to_string()); },
    };
    match tokio::fs::write(format!("drafts/{}.html", name), "").await {
        Ok(_) => {},
        Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; return Err("okary".to_string()); },
    }

    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(format!("drafts/{}.html", name))
        .await.expect("Unable to open file");

    while len > 0 {
        let mut buf = [0; 30000];
        let a = match stream.read(&mut buf).await {
            Ok(v) => v,
            Err(e) => { println!("{:#?}", e); return Err("error readig file".to_string()); },
        };
        if a == 0 { stream.respond(b"asadas".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?; return Err("bad request".to_string()); }

        match file.write_all(&buf[0..a]).await {
            Ok(_) => {},
            Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; },
        }
        len -= a;
    }

    println!("dfl : {:?}", name);

    stream.respond(format!("{}", name).bytes().collect(), "200 OK", Some("text/plain")).await?;
    Ok(())
}

pub async fn upload<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {


    let dir = ls(&PathBuf::from("assets/uploaded/"), PathBuf::from("assets/uploaded/"));
    let name = dir.len();

    let mut len = match headers.content_length {
        Some(v) => v as usize,
        None => { return Err("No file".to_string()); },
    };

    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(format!("assets/uploaded/{}.png", name))
        .await.expect("Unable to open file");

    while len > 0 {
        let mut buf = [0; 30000];
        let a = match stream.read(&mut buf).await {
            Ok(v) => v,
            Err(e) => { println!("{:#?}", e); return Err("error readig file".to_string()); },
        };
        if a == 0 { stream.respond(b"asadas".to_vec(), "400 BAD REQUEST", Some("text/plain")).await?; return Err("bad request".to_string()); }

        match file.write_all(&buf[0..a]).await {
            Ok(_) => {},
            Err(_) => { stream.respond(b"asdasd".to_vec(), "500 INTERNAL SERVER ERROR", Some("text/plain")).await?; },
        }
        len -= a;
    }

    println!("fl : {:?}", name);

    stream.respond(format!("{}", name).bytes().collect(), "200 OK", Some("text/plain")).await?;
    Ok(())
}


pub async fn load<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {


    if page.len() < 2 { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }
    let path = page[2..].join("/");

    let dir = ls(&PathBuf::from("drafts/"), PathBuf::from("drafts/"));
    println!("paf {:?} {:?}", dir, path);

    if !dir.contains(&path) { 
        stream.respond_file("assets/404.png", "200 OK").await?;
        return Err("What page?".to_string());
    }

    let template = read_to_string("assets/editor.html").unwrap();
    let content = draft(&path).await?;

    let res = template
        .replace("%EVERYTHING%", &content.content.unwrap().replace("<br>", "\n"))
        .replace("%NAME%", &content.name)
        .replace("%THUMB%", &content.img);

    stream.respond(res.bytes().collect(), "200 OK", Some("text/html")).await?;
    
    Ok(())
    
}

pub async fn draft(name : &str) -> Result<Draft, String> {
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
    parsed.content = Some( read[read.find("-->").unwrap() + 3..].to_string());

    return Ok(parsed);

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