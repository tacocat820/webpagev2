use std::path::PathBuf;
use std::sync::Arc;

use crate::Data;
use crate::ls::ls;
use crate::Headers;

use crate::Streamable;
use crate::StreamableWrapper;

use chrono::Datelike;
use chrono::Timelike;
use chrono::prelude::Utc;
use rand::RngExt;
use rand::SeedableRng;

pub async fn handle<T : Streamable>(stream : &mut StreamableWrapper<T>, _data : Arc<Data>, _headers : &Headers, _method : &str, page : &Vec<&str>) -> Result<(), String> {

    println!("{:?}", page);

    match page[0] {
        "" => stream.respond_file("assets/main.html", "200 OK").await?,
        "favicon.ico" => stream.respond_file("assets/icon.ico", "200 OK").await?,
        "any_pfp.png" => stream.respond_file("assets/pfps/1.png", "200 OK").await?,
        "any_bg.png" => stream.respond_file(&format!("assets/bgs{}", random_bg()), "200 OK").await?,
        _ => stream.respond_file("assets/404.png", "200 OK").await?,
    }
    
    Ok(())
    
}

fn random_bg() -> String {
    let today = Utc::now().day();
    let bgs = ls(&PathBuf::from("assets/bgs"), PathBuf::from("assets/bgs")); 

    let mut rng = rand::rngs::StdRng::seed_from_u64(today as u64); 
    let generated = rng.random_range(0..bgs.len());

    println!("{}", bgs[generated]);

    bgs[generated].clone()
 
}