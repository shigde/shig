use rand::{thread_rng, Rng};
use regex::Regex;

#[allow(dead_code)]
pub fn parse_track_id_start(track_id: &str) -> Option<String> {
    let re = Regex::new(r"^\{?([0-9a-fA-F]+)-").unwrap();
    if let Some(caps) = re.captures(track_id) {
        let first_part = &caps[1];
        return Some(first_part.to_string());
    }
    None
}

pub fn random_id(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = thread_rng();

    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
