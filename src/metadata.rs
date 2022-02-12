use std::fmt::Debug;

use crate::bencode::{decode, BEncodedType};
use anyhow::Result;

pub struct Metadata<'a> {
    be: BEncodedType<'a>,
    pub announce: &'a str,
    pub name: &'a str,
}

impl<'a> Metadata<'a> {
    pub fn parse(buf: &'a [u8]) -> Result<Metadata<'a>> {
        let be = decode(buf)?;
        let announce = be.dict_get("announce")?.as_str()?;
        let info = be.dict_get("info")?;
        let name = info.dict_get("name")?.as_str()?;

        Ok(Metadata {
            be: be,
            announce,
            name,
        })
    }
}

impl Debug for Metadata<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metadata")
            .field("announce", &self.announce)
            .field("name", &self.name)
            .finish()
    }
}

struct InfoMetadata<'a> {
    piece_length: u32,
    pieces: Vec<&'a [u8]>,
}
