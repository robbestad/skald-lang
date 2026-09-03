mod compile;
mod index;
mod json;
mod types;

pub use compile::{compile_dic, compile_dictionaries};
pub use json::{from_json, to_json};
pub use types::{BoundEntry, Dictionary, Entry, Table};

use std::sync::{Arc, OnceLock};

include!(concat!(env!("OUT_DIR"), "/en_us.rs"));

pub fn en_us() -> Arc<Dictionary> {
    static CELL: OnceLock<Arc<Dictionary>> = OnceLock::new();
    CELL.get_or_init(|| Arc::new(load_en_us())).clone()
}
