use once_cell::sync::OnceCell;
use rand::*;
use rand_chacha::*;
use std::{collections::HashMap, ffi, path::PathBuf};

type Error = Box<dyn std::error::Error + Send + Sync>;
static PATH: OnceCell<PathBuf> = OnceCell::new();

unsafe fn _init(path: *const libc::c_char) -> Option<()> {
    let as_cstr = ffi::CStr::from_ptr(path);
    let as_str = as_cstr.to_str().ok()?;
    PATH.set(PathBuf::from(as_str.to_string())).ok()?;
    Some(())
}

pub unsafe extern "C" fn init(path: *const libc::c_char) -> bool {
    _init(path).is_some()
}

struct HashHasher([u8; 8]);

impl std::hash::Hasher for HashHasher {
    fn write(&mut self, value: &[u8]) {
        self.0.copy_from_slice(&value[..8]);
    }

    fn finish(&self) -> u64 {
        u64::from_le_bytes(self.0)
    }
}

#[derive(Default)]
struct HashHasherBuilder;

impl std::hash::BuildHasher for HashHasherBuilder {
    type Hasher = HashHasher;
    fn build_hasher(&self) -> HashHasher {
        HashHasher([0u8; 8])
    }
}

type Key = [u8; 32];
type KeyStore = Key;

fn write_key(key: KeyStore) -> Result<(), Error> {
    todo!()
}

fn read_key() -> Result<Key, Error> {
    todo!()
}

type CenStore = HashMap<CEN, i64, HashHasherBuilder>;

fn read_cens() -> Result<CenStore, Error> {
    todo!()
}

fn add_new_cen(cen: CEN) -> Result<(), Error> {
    todo!()
}

fn gc_cens() -> Result<(), Error> {
    todo!()
}

const CEN_BYTES: usize = 20;
type CEN = [u8; CEN_BYTES];

fn unix_time() -> Result<u64, Error> {
    use std::time;
    Ok(time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)?
        .as_secs())
}

const SECONDS_PER_INTERVAL: u64 = 60 * 15;
const SECONDS_PER_LIFETIME: u64 = 60 * 60 * 24 * 14;
const INTERVALS_PER_LIFETIME: u64 = SECONDS_PER_LIFETIME / SECONDS_PER_INTERVAL;

pub fn generate_cens_into(key: Key, start_unixtime: u64, buf: &mut [u8]) {
    let mut chacha = ChaCha8Rng::from_seed(key);

    let cen_number = start_unixtime / SECONDS_PER_INTERVAL * CEN_BYTES as u64;
    let word_pos = cen_number / 4; // word_pos is measured in 4 byte words
    chacha.set_word_pos(word_pos as u128);

    chacha.fill_bytes(buf);
}

fn _generate_cen(buf: &mut [u8]) -> Option<()> {
    let key = read_key().ok()?;
    let time = unix_time().ok()?;
    generate_cens_into(key, time, buf);
    Some(())
}

pub unsafe extern "C" fn generate_cen(out: *mut u8) -> bool {
    let slice = std::slice::from_raw_parts_mut(out, CEN_BYTES);
    _generate_cen(slice).is_some()
}

pub unsafe extern "C" fn check_cens() -> *mut u8 {
    todo!()
}
