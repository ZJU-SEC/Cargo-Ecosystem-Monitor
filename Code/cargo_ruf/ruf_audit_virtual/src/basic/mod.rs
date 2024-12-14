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