use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use uuid::Uuid;

fn main() {
    let user_id = Uuid::new_v4();
    dbg!(&user_id);
    let _username = "admin".to_string();
    let password = "helloz2p4#5".to_string();
    let salt = SaltString::generate(&mut rand::thread_rng());
    let phc = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Couldn't hash password")
        .to_string();
    dbg!(&phc);
}
