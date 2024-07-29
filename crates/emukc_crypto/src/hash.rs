use std::{fs::File, io::Read, path::Path};

use md5::{Digest, Md5};
use sha2::Sha256;

const SALT: &str = "emukc_salt";

/// Trait for calculate simple hash
pub trait SimpleHash {
	/// Calculate simple hash
	///
	/// # Example
	///
	/// ```
	/// use emukc_crypto::SimpleHash;
	///
	/// let hash = "hello world".simple_hash();
	/// assert_eq!(hash, "DULfJyE3WQqNxy3ymuhAChyNR3yufT88pmqvAazKFMG4");
	/// ```
	fn simple_hash(&self) -> String;

	/// Calculate simple hash with salt
	///
	/// # Arguments
	///
	/// * `salt` - The salt you want to add to the hash
	///
	/// # Example
	///
	/// ```
	/// use emukc_crypto::SimpleHash;
	///
	/// let hash = "hello world".simple_hash_salted("salt");
	/// assert_eq!(hash, "CEH6Sx41BnKrSDgQwQskD1oW2tEvhH6Zx1t8f29ditYS");
	/// ```
	fn simple_hash_salted(&self, salt: &str) -> String;

	/// Calculate i64 hash
	fn hash_i64(&self) -> i64;
}

impl<T: AsRef<[u8]>> SimpleHash for T {
	fn simple_hash(&self) -> String {
		let mut hasher = Sha256::new();
		hasher.update(self.as_ref());
		let hash = hasher.finalize();
		bs58::encode(hash).into_string()
	}

	fn simple_hash_salted(&self, salt: &str) -> String {
		let salt = if salt.is_empty() {
			SALT
		} else {
			salt
		};
		let mut hasher = Sha256::new();
		hasher.update(salt.as_bytes());
		hasher.update(self.as_ref());
		let hash = hasher.finalize();
		bs58::encode(hash).into_string()
	}

	fn hash_i64(&self) -> i64 {
		if self.as_ref().is_empty() {
			0
		} else {
			self.as_ref()
				.iter()
				.fold(0i64, |acc, &x| acc.wrapping_shl(5).wrapping_sub(acc).wrapping_add(x as i64))
				.abs()
		}
	}
}

/// Calculate md5 hash of a string
///
/// # Arguments
///
/// * `input` - The string you want to hash
///
/// # Example
///
/// ```
/// use emukc_crypto::hash::md5;
///
/// let hash = md5("hello world");
/// assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
/// ```
pub fn md5(input: &str) -> String {
	let mut hasher = Md5::new();
	hasher.update(input);
	let hash = hasher.finalize();

	base16ct::lower::encode_string(&hash)
}

/// Calculate md5 hash of a file
///
/// # Arguments
///
/// * `path` - The path of the file
///
/// # Example
///
/// ```
/// use emukc_crypto::hash::md5_file;
///
/// let hash = md5_file("Cargo.toml").unwrap();
/// assert_eq!(hash, "375496e41179a266719f4770e76d83b7");
/// ```
pub fn md5_file<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
	let mut file = File::open(path)?;
	let mut hasher = Md5::new();
	let mut buffer = [0; 1024];
	loop {
		let count = file.read(&mut buffer)?;
		if count == 0 {
			break;
		}
		hasher.update(&buffer[..count]);
	}
	let hash = hasher.finalize();
	let hash = base16ct::lower::encode_string(&hash);
	Ok(hash)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_simple_hash() {
		let hash = "hello world".simple_hash();
		assert_eq!(hash, "DULfJyE3WQqNxy3ymuhAChyNR3yufT88pmqvAazKFMG4");
	}

	#[test]
	fn test_simple_hash_salted() {
		let hash = "hello world".simple_hash_salted("salt");
		assert_eq!(hash, "6g7aVvjVoDZ3GUe9oVonkLysBRqzDhv7qqt3RRD9gsWV");
	}
}
