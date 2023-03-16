use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use lazy_static::lazy_static;
use lifetime_hashmap::{get_lifetime, get_ruf_status_all, RUSTC_VER_NUM};

lazy_static! {
    static ref LIFETIME: HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> = get_lifetime();
}

#[derive(Debug)]
pub struct Feature {
    name: String,
    active: HashSet<usize>,
    accept: HashSet<usize>,
    removed: HashSet<usize>,
    unknown: HashSet<usize>,
}

impl Feature {
    pub fn new(name: String) -> Self {
        Self {
            name,
            active: HashSet::new(),
            accept: HashSet::new(),
            removed: HashSet::new(),
            unknown: HashSet::new(),
        }
    }

    pub fn sync_metas(&mut self) {
        if let Some(life) = get_ruf_status_all(&LIFETIME, &self.name) {
            for (i, status) in life.into_iter().enumerate() {
                match status {
                    "active" => self.active.insert(i),
                    "accepted" => self.accept.insert(i),
                    "removed" => self.removed.insert(i),
                    "None" => self.unknown.insert(i),
                    _ => unreachable!(),
                };
            }
        } else {
            self.unknown = (0..=RUSTC_VER_NUM).collect();
        }
    }

    pub fn usable(&self) -> HashSet<usize> {
        self.active.union(&self.accept).cloned().collect()
    }

    pub fn non_usable(&self) -> HashSet<usize> {
        self.removed.union(&self.unknown).cloned().collect()
    }

    // pub fn status(self, ver: usize) -> Option<String> {
    //     get_ruf_status(&LIFETIME, &self.name, ver).map(|s| s.to_string())
    // }
}
