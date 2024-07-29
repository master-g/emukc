/// Trait for password hashing and verification
pub trait PasswordCrypto {
	/// Hash a password
	fn hash_password(&self) -> String;

	/// Verify a password
	fn verify_password(&self, secret: &str) -> bool;
}

impl<T: AsRef<[u8]>> PasswordCrypto for T {
	fn hash_password(&self) -> String {
		bcrypt::hash(self, bcrypt::DEFAULT_COST).unwrap()
	}

	fn verify_password(&self, hash: &str) -> bool {
		bcrypt::verify(self, hash).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hash_password() {
		let password = "password";
		let hash = password.hash_password();
		println!("{}", hash);
		assert!(password.verify_password(&hash));
	}
}
