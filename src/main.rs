extern crate image;
extern crate rayon;

use rayon::prelude::*;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

use image::ImageDecoder;

fn main() {
    let counts: Arc<Mutex<[[u64; 2]; 16]>> = Arc::new(Mutex::new([[0, 0]; 16]));

    env::args()
        .skip(1)
        .collect::<Vec<String>>()
        .par_iter()
        .for_each(|arg| {
            let fin = File::open(&arg).unwrap();
            let fin = BufReader::new(fin);

            let mut decoder = image::tiff::TIFFDecoder::new(fin).unwrap();

            println!("Processing {}, dimensions {:?}", arg, decoder.dimensions());

            let mut img_counts: [[u64; 2]; 16] = [[0, 0]; 16];
            match decoder.read_image().unwrap() {
                image::DecodingResult::U8(img) => count_bin_histogram(img, &mut img_counts),
                image::DecodingResult::U16(img) => count_bin_histogram(img, &mut img_counts),
            }

            let mut shared_counts = counts.lock().unwrap();
            for (sc, ic) in shared_counts.iter_mut().zip(&img_counts) {
                sc[0] += ic[0];
                sc[1] += ic[1];
            }
        });

    let shared_counts = counts.lock().unwrap();
    let (zeros, ones): (Vec<u64>, Vec<u64>) =
        shared_counts.into_iter().map(|v| (v[0], v[1])).unzip();

    println!("1s: {:?}", ones);
    println!("0s: {:?}", zeros);
}

fn count_bin_histogram<T>(img: Vec<T>, counts: &mut [[u64; 2]; 16])
where
    T: std::marker::Copy
        + std::cmp::PartialEq<T>
        + std::convert::From<u8>
        + std::ops::ShrAssign<u32>
        + std::ops::BitAnd<T, Output = T>
        + EnumerateBits<T>,
{
    for p in img {
        for (i, b) in p.enumerate_bits() {
            counts[15 - (i as usize)][b as usize] += 1;
        }
    }
}

struct BitEnumerator<T>
where
    T: std::marker::Copy
        + std::cmp::PartialEq<T>
        + std::convert::From<u8>
        + std::ops::ShrAssign<u32>
        + std::ops::BitAnd<T, Output = T>,
{
    val: T,
    curr: u32,
    n_bits: u32,
}

impl<T> Iterator for BitEnumerator<T>
where
    T: std::marker::Copy
        + std::cmp::PartialEq<T>
        + std::convert::From<u8>
        + std::ops::ShrAssign<u32>
        + std::ops::BitAnd<T, Output = T>,
{
    type Item = (u32, bool);

    fn next(&mut self) -> std::option::Option<(u32, bool)> {
        if self.curr >= self.n_bits {
            None
        } else {
            let res = (self.curr, self.val.clone() & T::from(1) == T::from(1));
            self.val >>= 1;
            self.curr += 1;

            Some(res)
        }
    }
}

trait EnumerateBits<T>
where
    T: std::marker::Copy
        + std::cmp::PartialEq<T>
        + std::convert::From<u8>
        + std::ops::ShrAssign<u32>
        + std::ops::BitAnd<T, Output = T>,
{
    fn enumerate_bits(&self) -> BitEnumerator<T>;
}

impl EnumerateBits<u8> for u8 {
    fn enumerate_bits(&self) -> BitEnumerator<u8> {
        BitEnumerator::<u8> {
            val: *self,
            curr: 0,
            n_bits: 8,
        }
    }
}

impl EnumerateBits<u16> for u16 {
    fn enumerate_bits(&self) -> BitEnumerator<u16> {
        BitEnumerator::<u16> {
            val: *self,
            curr: 0,
            n_bits: 16,
        }
    }
}

#[test]
fn test_enumerate_bits() {
    let mut counts: [u64; 16] = [0; 16];
    let v1: u8 = 0b0110_1010;

    for (i, b) in v1.enumerate_bits() {
        counts[15 - (i as usize * 2 + b as usize)] += 1;
    }

    assert_eq!(counts, [0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1]);
}
