use std::fs::File;
use std::io::{Read};

mod bencode;

fn main() -> std::io::Result<()> {
    let mut file = File::open("./test.torrent").unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).unwrap();
    bencode::decode(&contents);
    Ok(())
}
