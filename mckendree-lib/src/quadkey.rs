use byteorder::{BigEndian, LittleEndian};
use zerocopy::{
    byteorder::U64, AsBytes, FromBytes, Unaligned, U16, U32,
};
use std::convert::TryInto;
use rand::distributions::{Distribution, Uniform};

#[derive(Debug, PartialEq)]
#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct BoundingBox
{
    x: U32<LittleEndian>,
    y: U32<LittleEndian>,
    w: U32<LittleEndian>,
    h: U32<LittleEndian>
}

impl BoundingBox {
    pub fn mk_random(rng: &mut rand::prelude::ThreadRng, max_size: u32) -> BoundingBox {
        let max = MAX_COORD + 1;

        let pos_dist = Uniform::new(0, max);
        let sz_dist = Uniform::new(0, max_size);

        let x: u32 = pos_dist.sample(rng);
        let y: u32 = pos_dist.sample(rng);
        let mut w: u32 = sz_dist.sample(rng);
        let mut h: u32 = sz_dist.sample(rng);

        if x + w > MAX_COORD { w = MAX_COORD - x; }
        if y + h > MAX_COORD { h = MAX_COORD - y; }

        BoundingBox {
            x: U32::new(x), y: U32::new(y), w: U32::new(w), h: U32::new(h)
        }
    }

    pub fn contains(&self, bbox: &BoundingBox) -> bool {
        self.x.get() <= bbox.x.get() &&
        self.y.get() <= bbox.y.get() &&
        self.x.get() + self.w.get() >= bbox.x.get() + bbox.w.get() &&
        self.y.get() + self.h.get() >= bbox.y.get() + bbox.h.get()
    }
}

#[derive(Debug)]
#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct DbKey
{
    pub quadkey: U64<BigEndian>,
    pub entity: U16<BigEndian>
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct DbValue {
    pub bbox: BoundingBox,
    pub is_black: u8,
}

pub const MAX_COORD: u32 = (1 << 29) - 1;
pub const MAP_SIZE: f64 = 10000000.0;

impl DbKey {
    pub fn from_bbox(bbox: &BoundingBox) -> DbKey {
        let mut key: u64 = 0;
        let x1 = bbox.x.get();
        let y1 = bbox.y.get();
        let w = bbox.w.get();
        let h = bbox.h.get();
        let x2 = x1 + w;
        let y2 = y1 + h;
        assert!(x2 <= MAX_COORD);
        assert!(y2 <= MAX_COORD);
        let mut zoom = 0;
        while zoom < 29 {
            let shift = 28 - zoom;
            let x1_b = ((x1 >> shift) & 1) == 1;
            let y1_b = ((y1 >> shift) & 1) == 1;
            let x2_b = ((x2 >> shift) & 1) == 1;
            let y2_b = ((y2 >> shift) & 1) == 1;
            if x1_b != x2_b || y1_b != y2_b { break; }
            key = key << 1;
            if x1_b { key += 1; }
            key = key << 1;
            if y1_b { key += 1; }
            zoom += 1;
        };
        key = key.checked_shl(64 - 2 * zoom).unwrap_or(0);
        key += zoom as u64;
        DbKey { quadkey: U64::new(key), entity: U16::new(0) }
    }

    pub fn to_bbox(&self) -> BoundingBox {
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let k = self.quadkey.get();
        let zoom = (k & 63) as u32;
        for bit in 0..zoom {
            x = x << 1;
            y = y << 1;
            let shift_x = (63 - 2 * bit).try_into().unwrap();
            let shift_y = (62 - 2 * bit).try_into().unwrap();
            if (k.checked_shr(shift_x).unwrap_or(0)) & 1 == 1 { x += 1; }
            if (k.checked_shr(shift_y).unwrap_or(0)) & 1 == 1 { y += 1; }
        }
        x = x.checked_shl(29 - zoom).unwrap_or(0);
        y = y.checked_shl(29 - zoom).unwrap_or(0);
        let w = (1 << (29 - zoom)) - 1;
        BoundingBox { x: U32::new(x), y: U32::new(y), w: U32::new(w), h: U32::new(w) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_to_quad_key_and_back() {
        let mut rng = rand::thread_rng();
        let max = MAX_COORD + 1;
        for i in 0..10000 {
            let bbox = BoundingBox::mk_random(&mut rng,
                if i > 9000 {max} else {max / 1024});
            let key = DbKey::from_bbox(&bbox);
            let key_box = key.to_bbox();
            assert!(key_box.contains(&bbox),
                "#{}: {:?} ⊄ {:?}", i, bbox, key_box);
        }
    }
}