//! Argon2id password hashing (spec §8.1), with the library's default cost: 19 MiB of memory,
//! two passes, one lane.

use std::sync::LazyLock;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash as Phc;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use smoke_domain::records::PasswordHash;
use smoke_domain::types::NewPassword;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not hash the password: {0}")]
pub struct HashError(String);

/// Hashes with a fresh random salt. CPU-heavy: call it from a blocking thread in async code.
pub fn hash(password: &NewPassword) -> Result<PasswordHash, HashError> {
    Argon2::default()
        .hash_password(password.expose().as_bytes())
        .map(|phc| PasswordHash::new(phc.to_string()))
        .map_err(|e| HashError(e.to_string()))
}

/// A hash of a password nobody has, checked when there is no real hash so that an unknown
/// username costs as much time as a wrong password.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    Argon2::default()
        .hash_password(b"no account has this password")
        .expect("hashing a constant cannot fail")
        .to_string()
});

/// Whether `attempt` matches `stored`. Always runs one Argon2 verification, also when there is
/// no stored hash. CPU-heavy: call it from a blocking thread in async code.
pub fn verify(stored: Option<&PasswordHash>, attempt: &str) -> bool {
    let argon2 = Argon2::default();
    if let Some(stored) = stored {
        match Phc::new(stored.as_str()) {
            Ok(phc) => return argon2.verify_password(attempt.as_bytes(), &phc).is_ok(),
            Err(error) => tracing::error!(%error, "a stored password hash is not a PHC string"),
        }
    }
    let dummy = Phc::new(&DUMMY_HASH).expect("the dummy hash is a PHC string");
    let _ = argon2.verify_password(attempt.as_bytes(), &dummy);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFIX: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

    fn password(text: &str) -> NewPassword {
        NewPassword::new(text.to_owned()).unwrap()
    }

    #[test]
    fn a_hash_verifies_only_its_own_password() {
        let stored = hash(&password("correct horse battery")).unwrap();
        assert!(verify(Some(&stored), "correct horse battery"));
        assert!(!verify(Some(&stored), "correct horse batter"));
        assert!(!verify(Some(&stored), ""));
    }

    #[test]
    fn hashes_are_salted_argon2id_with_the_default_cost() {
        let a = hash(&password("correct horse battery")).unwrap();
        let b = hash(&password("correct horse battery")).unwrap();
        assert!(a.as_str().starts_with(PREFIX), "{}", a.as_str());
        assert_ne!(a, b);
    }

    #[test]
    fn the_dummy_hash_costs_the_same_as_a_real_one() {
        assert!(DUMMY_HASH.starts_with(PREFIX), "{}", *DUMMY_HASH);
    }

    #[test]
    fn a_missing_or_broken_hash_never_verifies() {
        assert!(!verify(None, "no account has this password"));
        let broken = PasswordHash::new("not-a-phc-string".to_owned());
        assert!(!verify(Some(&broken), "anything"));
    }
}
