//! The `api_port` request key, ported from the client's `PortAPI._createKey`.
//!
//! `api_port/port` rejects a request that carries only `api_token`: the client
//! also posts `api_sort_key`, `spi_sort_order` (the upstream field really is
//! spelled `spi_`) and `api_port`, the last being a signature over the member
//! id, the current second and three random draws. A request without it comes
//! back as `api_result: 100`.

use std::time::{SystemTime, UNIX_EPOCH};

/// `PORT_API_SEED`, indexed by `member_id % 10`.
static PORT_API_SEED: [i64; 10] = [3187, 3596, 6413, 9628, 7279, 7678, 6023, 2564, 9558, 9272];

/// JS `String.prototype.substr`, which clamps instead of panicking. Every
/// string here is ASCII digits, so byte slicing is safe.
fn substr(s: &str, start: usize, len: Option<usize>) -> &str {
    let start = start.min(s.len());
    let end = match len {
        Some(len) => (start + len).min(s.len()),
        None => s.len(),
    };
    &s[start..end]
}

/// The `api_port` key.
pub struct PortApiKey;

impl PortApiKey {
    /// Build the key for this member id, drawing the three randoms and the
    /// timestamp the way the client does.
    pub fn create(member_id: i64) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        Self::create_with(
            member_id,
            now,
            [
                fastrand::i64(0..9),
                fastrand::i64(0..8999),
                fastrand::i64(0..32767),
                fastrand::i64(0..10),
                fastrand::i64(0..10),
                fastrand::i64(0..10),
            ],
        )
    }

    /// The key with every draw supplied, so the port can be checked against
    /// the client's own output. `draws` is `[r1, r2, r3, d, e, f]` in the
    /// order the client draws them: three `Math.random()` values before their
    /// offsets, then the three digits spliced into the result.
    fn create_with(member_id: i64, now_sec: i64, draws: [i64; 6]) -> String {
        let [r1, r2, r3, d, e, f] = draws;
        let seed = PORT_API_SEED[(member_id % 10) as usize];
        let a = 1000 * (r1 + 1) + member_id % 1000;
        let b = r2 + 1000;
        let c = r3 + 32768;

        // `parseInt(member_id.toString().substr(0, 4))`: the leading four
        // digits, or the whole id when it is shorter.
        let id_text = member_id.to_string();
        let head_digits: i64 = substr(&id_text, 0, Some(4)).parse().unwrap_or(0);

        let g =
            ((4132653 + c) * (head_digits + 1000) - now_sec + (1875979 + 9 * c) - member_id) * seed;

        let mut s = format!("{d}{a}{g}{b}");
        s = format!("{}{e}{}", substr(&s, 0, Some(8)), substr(&s, 8, None));
        s = format!("{}{f}{}", substr(&s, 0, Some(18)), substr(&s, 18, None));
        s.push_str(&c.to_string());
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors produced by running the client's own `_createKey` (transcribed
    /// out of `main.decoded.js` with the randoms lifted into parameters).
    #[test]
    fn matches_the_client_implementation() {
        let cases = [
            (12345678, 1790070509, [0, 0, 0, 0, 0, 0], "016787170357072736044100032768"),
            (12345678, 1790070509, [8, 8998, 32766, 9, 9, 9], "996787249381642446948999865534"),
            (1000, 1600000000, [4, 1234, 10000, 1, 2, 3], "150002152221357266317223442768"),
            (99999999, 1790070509, [3, 777, 20000, 5, 0, 7], "5499940903377151197664177752768"),
            (7, 1, [0, 0, 0, 0, 0, 0], "010071070604665641020100032768"),
        ];
        for (member_id, now, draws, expected) in cases {
            assert_eq!(
                PortApiKey::create_with(member_id, now, draws),
                expected,
                "member {member_id} at {now}"
            );
        }
    }

    #[test]
    fn a_live_key_has_the_shape_the_server_expects() {
        let key = PortApiKey::create(12345678);
        assert!(key.len() >= 28, "the key is a long digit run, got {key}");
        assert!(key.chars().all(|c| c.is_ascii_digit()), "digits only, got {key}");
    }
}
