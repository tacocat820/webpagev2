use std::collections::HashMap;

use rand::distr::SampleString;



struct Token {
    expires : u32
}

pub struct Tokens {
    ls : HashMap<String, Token>
}

impl Tokens {
    pub fn new() -> Tokens {
        Tokens { ls: HashMap::new() }
    }

    pub fn cleanup(&mut self) {

        let mut rm = vec![];
        for i in self.ls.iter_mut() {
            i.1.expires -= 1;
            
            if i.1.expires == 0 { rm.push(i.0.clone()); }
        }

        for i in rm.iter() {
            println!("rm {}", i);
            self.ls.remove(i).unwrap();
        }
    }

    pub fn get(&self, token : &str) -> Option<()> {
        if self.ls.contains_key(token) {
            Some(())
        } else {
            None
        }
    }

    pub fn new_token(&mut self) -> String {
        let mut rand;
        
        loop {
            rand = rand::distr::Alphanumeric.sample_string(&mut rand::rng(), 32);

            if !self.ls.contains_key(&rand) {
                break;
            }
        }

        self.ls.insert(rand.clone(), Token { expires: 10 });
        rand
    }
}