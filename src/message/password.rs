use crate::io::BufMutExt;
use crate::message::{FrontendMessage, FrontendMessageFormat};
use crate::Error;
use std::num::Saturating;

#[derive(Debug)]
pub enum Password<'a> {
    Cleartext(&'a str),
}

impl FrontendMessage for Password<'_> {
    const FORMAT: FrontendMessageFormat = FrontendMessageFormat::PasswordPolymorphic;

    #[inline(always)]
    fn body_size_hint(&self) -> Saturating<usize> {
        let mut size = Saturating(0);

        match self {
            Password::Cleartext(password) => {
                // To avoid reporting the exact password length anywhere,
                // we deliberately give a bad estimate.
                //
                // This shouldn't affect performance in the long run.
                size += password
                    .len()
                    .saturating_add(1) // NUL terminator
                    .checked_next_power_of_two()
                    .unwrap_or(usize::MAX);
            }
        }

        size
    }

    fn encode_body(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Password::Cleartext(password) => {
                buf.put_str_nul(password);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::message::FrontendMessage;

    use super::Password;

    #[test]
    fn test_encode_clear_password() {
        const EXPECTED: &[u8] = b"p\0\0\0\rpassword\0";

        let mut buf = Vec::new();
        let m = Password::Cleartext("password");

        m.encode_msg(&mut buf).unwrap();

        assert_eq!(buf, EXPECTED);
    }

    #[cfg(all(test, not(debug_assertions)))]
    #[bench]
    fn bench_encode_clear_password(b: &mut test::Bencher) {
        use test::black_box;

        let mut buf = Vec::with_capacity(128);

        b.iter(|| {
            buf.clear();

            black_box(Password::Cleartext("password")).encode_msg(&mut buf);
        });
    }
}
