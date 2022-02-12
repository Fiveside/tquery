use anyhow::Result;
use std::fs::File;
use std::io::Read;

mod bencode;

fn main() -> Result<()> {
    let mut file = File::open("./test.torrent").unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).unwrap();
    let decoded = bencode::decode(&contents)?;
    println!("Top level type is {:?}", decoded.type_str());
    println!("Keys of the top level thing: {:?}", decoded.dict_keys()?);
    let info = decoded.dict_get("info")?;
    println!("Keys of the `info` object: {:?}", info.dict_keys()?);

    for k in info.dict_keys()? {
        let sub = info.dict_get(k)?;
        println!("Type of {k} is {}", sub.type_str());
    }
    Ok(())
}
