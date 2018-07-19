use bcrypt::{DEFAULT_COST, hash};
use uuid::Uuid as GenUuid;

// Containers to keep track of all the strings we have running around
pub struct HashedPassword(pub String);
pub struct Uuid(pub String);

pub fn hash_password(password: &str) -> Result<HashedPassword, String> {
    let hashed = hash(password, DEFAULT_COST);
    match hashed {
        Ok(pw) => Ok(HashedPassword(pw)),
        Err(e) => Err(format!("{:?}", e)),
    }
}

pub fn uuid() -> Uuid {
    let id = GenUuid::new_v4();
    Uuid (id.simple().to_string())
}

#[cfg(test)]
mod test {
    use bcrypt::verify;
    use super::*;

    #[test]
    fn password_hashing() {
        // letters, numbers, special chars & extended ascii
        let pw = "åî>@%åÄSt»Æ·wj³´m~ðjC½µæGjq6?ï";
        let hashed = hash_password(pw).expect("hashing failed");

        assert!(verify(pw, &hashed.0).expect("hash failed"), "hashes match");
        assert!(!verify("moo moo", &hashed.0).expect("hash failed"), "diff strings dont match");
    }

    #[test]
    fn gen_uuids() {
        let v4 = uuid();
        assert_eq!(v4.0.find("-"), None);
        assert_eq!(v4.0.len(), 32);
    }
}
