use crate::io::{PortalId, StatementId};

/// Postgres-flavoured writer extension for `Vec<u8>`.
///
/// Groups the little conveniences shared by frontend-message encoders
/// (nul-terminated strings, length-prefixed sections, statement/portal names).
pub(crate) trait BufMutExt {
    /// Write `s` followed by a `\0` terminator.
    fn put_str_nul(&mut self, s: &str);

    /// Reserve a 4-byte length slot, invoke `f` to write the body, then
    /// back-fill the slot with the body's byte length. Body is truncated
    /// on error so nothing partial escapes.
    fn put_length_prefixed<F>(&mut self, f: F) -> Result<(), crate::Error>
    where
        F: FnOnce(&mut Vec<u8>) -> Result<(), crate::Error>;

    /// Write a prepared-statement name (empty for the unnamed statement).
    fn put_statement_name(&mut self, id: StatementId);

    /// Write a portal name (empty for the unnamed portal).
    fn put_portal_name(&mut self, id: PortalId);
}

impl BufMutExt for Vec<u8> {
    #[inline]
    fn put_str_nul(&mut self, s: &str) {
        self.extend(s.as_bytes());
        self.push(0);
    }

    fn put_length_prefixed<F>(&mut self, write_contents: F) -> Result<(), crate::Error>
    where
        F: FnOnce(&mut Vec<u8>) -> Result<(), crate::Error>,
    {
        // reserve space to write the prefixed length
        let offset = self.len();
        self.extend(&[0; 4]);

        // write the main body of the message
        let write_result = write_contents(self);

        let size_result = write_result.and_then(|_| {
            let size = self.len() - offset;
            i32::try_from(size)
                .map_err(|_| crate::err_protocol!("message size out of range for protocol: {size}"))
        });

        match size_result {
            Ok(size) => {
                // now calculate the size of what we wrote and set the length value
                self[offset..(offset + 4)].copy_from_slice(&size.to_be_bytes());
                Ok(())
            }
            Err(e) => {
                // Put the buffer back to where it was.
                self.truncate(offset);
                Err(e)
            }
        }
    }

    #[inline]
    fn put_statement_name(&mut self, id: StatementId) {
        id.put_name_with_nul(self);
    }

    #[inline]
    fn put_portal_name(&mut self, id: PortalId) {
        id.put_name_with_nul(self);
    }
}
