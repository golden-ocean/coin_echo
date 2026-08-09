//! 密码哈希与校验。

use argon2::password_hash::{
    PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString, rand_core::OsRng,
};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::password::config::PasswordConfig;
use crate::password::error::PasswordError;

/// Argon2id 密码哈希器。
///
/// 无内部可变状态，`Argon2` 实例按配置构造一次，可安全地在多线程间共享
/// （`&self` 方法即可，无需 `Arc<Mutex<_>>`）。
pub struct PasswordHasher {
    argon2: Argon2<'static>,
}

impl std::fmt::Debug for PasswordHasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordHasher").finish_non_exhaustive()
    }
}

impl PasswordHasher {
    /// 由配置构造。调用前应先对 `config` 调用过 [`PasswordConfig::validate`]。
    pub fn new(config: &PasswordConfig) -> Result<Self, PasswordError> {
        let params = Params::new(
            config.memory_kib,
            config.iterations,
            config.parallelism,
            None,
        )
        .map_err(|_| PasswordError::HashingFailed)?;
        Ok(Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        })
    }

    /// 对明文密码生成哈希（PHC 字符串格式，含随机盐，可直接存库）。
    pub fn hash(&self, plain_password: &str) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        self.argon2
            .hash_password(plain_password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| PasswordError::HashingFailed)
    }

    /// 校验明文密码是否与已存储的哈希匹配。
    pub fn verify(&self, plain_password: &str, stored_hash: &str) -> Result<(), PasswordError> {
        let parsed_hash =
            PasswordHash::new(stored_hash).map_err(|_| PasswordError::InvalidHashFormat)?;
        self.argon2
            .verify_password(plain_password.as_bytes(), &parsed_hash)
            .map_err(|_| PasswordError::Mismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hasher() -> PasswordHasher {
        PasswordHasher::new(&PasswordConfig::default()).unwrap()
    }

    #[test]
    fn correct_password_verifies_successfully() {
        let h = hasher();
        let hash = h.hash("correct-horse-battery-staple").unwrap();
        assert!(h.verify("correct-horse-battery-staple", &hash).is_ok());
    }

    #[test]
    fn wrong_password_is_rejected() {
        let h = hasher();
        let hash = h.hash("correct-horse-battery-staple").unwrap();
        assert!(matches!(
            h.verify("wrong-password", &hash),
            Err(PasswordError::Mismatch)
        ));
    }

    #[test]
    fn same_password_produces_different_hashes_due_to_random_salt() {
        let h = hasher();
        let hash_a = h.hash("same-password").unwrap();
        let hash_b = h.hash("same-password").unwrap();
        assert_ne!(hash_a, hash_b);
        assert!(h.verify("same-password", &hash_a).is_ok());
        assert!(h.verify("same-password", &hash_b).is_ok());
    }

    #[test]
    fn malformed_stored_hash_is_rejected_not_panicking() {
        let h = hasher();
        let result = h.verify("anything", "not-a-valid-phc-string");
        assert!(matches!(result, Err(PasswordError::InvalidHashFormat)));
    }
}
