pub fn unix_timestamp_secs() -> i64 {
    cfg_if::cfg_if! {
        if #[cfg(feature = "native")] {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64
        } else {
            // wasm32 fallback using JS Date (ms precision)
            (js_sys::Date::now() / 1000.0) as i64
        }
    }
}
