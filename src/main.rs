use std::collections::HashMap;

struct Cache {
    data: Vec<HashMap<u64, usize>>,
    set_capacity: usize,
    current: usize,
}

impl Cache {
    fn new(set_count: usize, set_capacity: usize) -> Cache {
        let mut data = Vec::<HashMap<u64, usize>>::new();
        data.resize(set_count, HashMap::<u64, usize>::new());
        Cache { data, set_capacity, current: 0 }
    }

    fn test_and_store(&mut self, set_index: usize, tag: u64) -> (bool, bool) {
        // println!("{:?} {:?}", set_index, tag);
        let set = self.data.get_mut(set_index).unwrap();
        let hit: bool;
        let swap: bool;
        if let Some(latest) = set.get_mut(&tag) {
            hit = true;
            swap = false;
            *latest = self.current;
        } else {
            hit = false;
            assert!(set.len() <= self.set_capacity);
            if set.len() == self.set_capacity {
                swap = true;
                let mut oldest: usize = self.current;
                let mut oldest_tag: u64 = 0;
                for (tag, latest) in set.iter() {
                    if latest < &oldest {
                        oldest = *latest;
                        oldest_tag = *tag;
                    }
                }
                set.remove(&oldest_tag);
            } else {
                swap = false;
            }
            set.insert(tag, self.current);
        }
        self.current += 1;
        (hit, swap)
    }
}

struct CacheManager {
    cache: Cache,

    hit: usize,
    miss: usize,
    swap: usize,
    // dirty_bytes_count: usize,

    tag_len: usize,
    index_len: usize,
    offset_len: usize,
}

impl CacheManager {
    fn new(
        tag_len: usize, 
        index_len: usize, 
        offset_len: usize, 
        set_capacity: usize
    ) -> CacheManager {
        if tag_len + index_len + offset_len > 64 {
            panic!("address length > 64 is not supported");
        }
        let cache = Cache::new(1 << index_len, set_capacity);
        return CacheManager {
            cache,
            hit: 0, miss: 0, swap: 0, // dirty_bytes_count: 0,
            tag_len, index_len, offset_len,
        }
    }

    fn load(&mut self, address: u64, length: usize) {
        let mut address = address;
        let mut length = length;
        loop {
            let _offset = (address >> 0) & ((1 << self.offset_len) - 1);
            let index = (address >> self.offset_len) & ((1 << self.index_len) - 1);
            let tag = (address >> (self.offset_len + self.index_len)) & ((1 << self.tag_len) - 1);

            let (hit, swap) = self.cache.test_and_store(index as usize, tag);
            if hit {
                self.hit += 1;
            } else {
                self.miss += 1;
            }
            if swap {
                self.swap += 1;
            }

            let next_block = (((tag << self.index_len) | index) + 1) << self.offset_len;
            if length <= (next_block - address) as usize {
                return;
            }
            length -= (next_block - address) as usize;
            address = next_block;
        }
    }
}

use std::env;
use std::fs::read_to_string;
use regex::Regex;
use std::str::FromStr;

fn env_get<T: FromStr>(name: &str, default: T) -> T {
    env::var(name).ok()
        .and_then(|b| b.parse::<T>().ok())
        .unwrap_or(default)
}

fn main() {
    // default: Core i7 L1 Data Cache
    let mut manager = CacheManager::new(
        env_get::<usize>("TAG_LEN", 35), 
        env_get::<usize>("INDEX_LEN", 6), 
        env_get::<usize>("OFFSET_LEN", 6), 
        env_get::<usize>("SET_SIZE", 1 << 3),
    );

    let file_name = env::args().nth(1).unwrap();
    let file_content = read_to_string(file_name).unwrap();
    let line_regex = Regex::new(
        r"(?P<action>[LS]) (?P<address>[0-9a-f]+), (?P<length>\d+)").unwrap();
    for line in file_content.lines() {
        let captures = line_regex.captures(line).unwrap();
        let action = &captures["action"];
        let address = u64::from_str_radix(&captures["address"], 16).unwrap();
        let length = &captures["length"].parse::<usize>().unwrap();

        if action == "L" {
            manager.load(address, *length);
        } else {
            // TODO: write a method for store action
            manager.load(address, *length);
        }
    }

    println!("hit: {}", manager.hit);
    println!("miss: {}", manager.miss);
    println!("swap: {}", manager.swap);
}
