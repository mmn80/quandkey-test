use std::time::{Instant};
use sled;
use zerocopy::{AsBytes, byteorder::U16};

pub mod quadkey;

pub fn init() -> () {
    println!("opening sled db...");
    let now = Instant::now();
    let tree = sled::open("./sled-db").expect("open");
    println!("elapsed: {}, recovered: {}", now.elapsed().as_secs(), tree.was_recovered());
    if let Err(err) = gen_entities(&tree) {
        panic!(err)
    }
    tree.flush().expect("flush error");
}

pub fn gen_entities(tree: &sled::Db) -> Result<(), sled::Error> {
    let max_size_meters = 100.0;
    let max_size = (quadkey::MAX_COORD as f64 + 1.0) * (max_size_meters / quadkey::MAP_SIZE);
    let max_size = max_size as u32;
    let entities = 100000;
    println!("generating {} random entities at most {} m...", entities, max_size_meters);
    let now = Instant::now();
    let mut elapsed = 0;
    let mut rng = rand::thread_rng();
    let mut duplicates = 0;
    let mut first: bool;
    for x in 0..entities {
        let bbox = quadkey::BoundingBox::mk_random(&mut rng, max_size);
        let mut key = quadkey::DbKey::from_bbox(&bbox);
        first = true;
        while tree.contains_key(sled::IVec::from(key.as_bytes())).expect("contains_key") {
            //println!("found duplicate: {:?}", key);
            if first { duplicates += 1; }
            first = false;
            key.entity = U16::new(key.entity.get() + 1);
        }
        let val = quadkey::DbValue { bbox, is_black: 0 };
        tree.insert(sled::IVec::from(key.as_bytes()), sled::IVec::from(val.as_bytes()))?;
        let new_elapsed = now.elapsed().as_secs();
        if new_elapsed - elapsed >= 1 {
            elapsed = new_elapsed;
            println!("#{}. elapsed: {} sec, duplicates: {}", x, elapsed, duplicates);
            tree.flush().expect("flush error");
        }
    }
    println!("elapsed: {} sec, duplicates: {}", now.elapsed().as_secs(), duplicates);
    Ok(())
}

