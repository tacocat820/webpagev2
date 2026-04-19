use std::path::PathBuf;
use std::fs;

pub fn ls(initial : &PathBuf, path : PathBuf) -> Vec<String> {
    let mut files : Vec<String> = vec![];

    let entries = fs::read_dir(&path)
        .unwrap_or_else(|_| panic!("Failed reading files in a directory {}", path.display()));

    for entry in entries {
        if entry.is_err() || entry.as_ref().unwrap().metadata().is_err() { continue; }

        let entry = entry.unwrap();
        let metadata = entry.metadata()
            .unwrap_or_else(|_| panic!("Couldn't get file / directory metadata {}", entry.path().display()));

        if metadata.is_dir() {
            files.append(&mut ls(initial, entry.path()));
        }  else if metadata.is_file() {
            let entrypath = entry.path();
            let entry = match entrypath.to_str() {
                Some(val) => {val.strip_prefix(initial.to_str().unwrap()).expect("Found doesn't start with root path")},
                None => { continue; }
            };
            files.push(entry.to_string()); // TODO   
        }
    }

    files
}