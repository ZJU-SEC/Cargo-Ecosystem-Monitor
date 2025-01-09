mod ruf_info;
mod ruf_lifetime;

pub use ruf_info::*;
pub use ruf_lifetime::RUSTC_VER_NUM;

use fxhash::FxHashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref RUF_LIFETIME: FxHashMap<&'static str, [u8; RUSTC_VER_NUM]> =
        ruf_lifetime::get_lifetime_raw();
}

pub fn get_ruf_status(ruf_name: &str, rustc_ver: u32) -> RufStatus {
    if let Some(ruf_lifetime) = RUF_LIFETIME.get(ruf_name) {
        assert!((rustc_ver as usize) < RUSTC_VER_NUM);

        let ruf_status = RufStatus::from(ruf_lifetime[rustc_ver as usize] as u32);
        return ruf_status;
    }

    RufStatus::Unknown
}

#[allow(unused)]
pub fn get_all_ruf_status(ruf_name: &str) -> [RufStatus; RUSTC_VER_NUM] {
    let mut ruf_status = Vec::new();

    if let Some(ruf_lifetime) = RUF_LIFETIME.get(ruf_name) {
        for i in 0..RUSTC_VER_NUM {
            ruf_status.push(RufStatus::from(ruf_lifetime[i] as u32));
        }
    } else {
        for _ in 0..RUSTC_VER_NUM {
            ruf_status.push(RufStatus::Unknown);
        }
    }

    ruf_status
        .into_iter()
        .collect::<Vec<RufStatus>>()
        .try_into()
        .unwrap()
}
