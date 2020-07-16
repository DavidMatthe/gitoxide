use crate::SHA1_SIZE;

/// A reference to a SHA1 identifying objects
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize))]
pub struct Id<'a>(&'a [u8; SHA1_SIZE]);

impl<'a> Id<'a> {
    pub fn encode_to_40_bytes_slice(&self, out: &mut [u8]) -> Result<(), hex::FromHexError> {
        hex::encode_to_slice(self.0, out)
    }
}

impl<'a> From<&'a [u8; SHA1_SIZE]> for Id<'a> {
    fn from(v: &'a [u8; SHA1_SIZE]) -> Self {
        Id(v)
    }
}
