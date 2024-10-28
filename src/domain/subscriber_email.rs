use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: &str) -> Result<Self, String> {
        if validate_email(s) {
            Ok(Self(s.into()))
        } else {
            Err("not a valid email".into())
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use quickcheck::Arbitrary;
    use rand::{rngs::StdRng, SeedableRng};

    #[derive(Debug, Clone)]
    pub struct ValidEmailFixture(String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let seed = Arbitrary::arbitrary(g);
            let mut rng = StdRng::seed_from_u64(seed);
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_succesfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(&valid_email.0).is_ok()
    }

    #[test]
    fn empty_email_is_rejected() {
        let s = "";
        assert_err!(SubscriberEmail::parse(s));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let s = "someone.com";
        assert_err!(SubscriberEmail::parse(s));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let s = "@domain.com";
        assert_err!(SubscriberEmail::parse(s));
    }
}
