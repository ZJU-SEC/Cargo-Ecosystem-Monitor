use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex}, cell::RefCell,
};

use lazy_static::lazy_static;
use lifetime_hashmap::{get_lifetime, get_ruf_status_all, RUSTC_VER_NUM};

lazy_static! {
    static ref LIFETIME: HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> = get_lifetime();
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub active: HashSet<usize>,
    pub accept: HashSet<usize>,
    pub removed: HashSet<usize>,
    pub unknown: HashSet<usize>,
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

pub struct FEATURE_STORAGE(Arc<RefCell<HashMap<String, Feature>>>);

impl FEATURE_STORAGE {
    pub fn new() -> Self {
        Self(Arc::new(RefCell::new(HashMap::new())))
    }

    pub fn set(&self, feat: Feature) {
        self.0.borrow_mut().insert(feat.name.clone(), feat);
    }

    pub fn get(&self, name: &str) -> Option<Feature> {
        self.0.borrow().get(name).cloned()
    }
}

unsafe impl Sync for FEATURE_STORAGE {}