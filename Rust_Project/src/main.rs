use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Dummy {
    id: u32,
    name: String,
}

fn main() {
    println!("Hello, world with Serde!");
}
