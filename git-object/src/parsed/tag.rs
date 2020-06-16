use super::{
    util::{parse_timezone_offset, split2_at_space},
    Error,
};
use crate::{parsed::Signature, Time};
use bstr::{BStr, ByteSlice};
use btoi::btoi;
use hex::FromHex;
use nom::{
    bytes::complete::tag,
    bytes::complete::{take_while1, take_while_m_n},
    character::is_alphabetic,
    sequence::{preceded, terminated},
    IResult,
};

#[derive(PartialEq, Eq, Debug, Hash)]
pub struct Tag<'data> {
    pub target: &'data BStr,
    pub name: &'data BStr,
    pub target_kind: crate::Kind,
    pub message: Option<&'data BStr>,
    pub signature: Signature<'data>,
    pub pgp_signature: Option<&'data BStr>,
}

fn parse_signature(d: &[u8]) -> Result<Signature, Error> {
    const ONE_SPACE: usize = 1;
    let email_begin = d
        .iter()
        .position(|&b| b == b'<')
        .ok_or_else(|| {
            Error::ParseError(
                "Could not find beginning of email marked by '<'",
                d.to_owned(),
            )
        })
        .and_then(|pos| {
            if pos == 0 {
                Err(Error::ParseError(
                    "Email found in place of author name",
                    d.to_owned(),
                ))
            } else {
                Ok(pos)
            }
        })?;
    let email_end = email_begin
        + d.iter()
            .skip(email_begin)
            .position(|&b| b == b'>')
            .ok_or_else(|| {
                Error::ParseError("Could not find end of email marked by '>'", d.to_owned())
            })
            .and_then(|pos| {
                if pos >= d.len() - 1 - ONE_SPACE {
                    Err(Error::ParseError(
                        "There is no time after email",
                        d.to_owned(),
                    ))
                } else {
                    Ok(pos)
                }
            })?;
    let (time_in_seconds, tzofz) = split2_at_space(&d[email_end + ONE_SPACE + 1..], |_, _| true)?;
    let (offset, sign) = parse_timezone_offset(tzofz)?;

    Ok(Signature {
        name: (&d[..email_begin - ONE_SPACE]).as_bstr(),
        email: (&d[email_begin + 1..email_end]).as_bstr(),
        time: Time {
            time: btoi::<u32>(time_in_seconds).map_err(|e| {
                Error::ParseIntegerError("Could parse to seconds", time_in_seconds.to_owned(), e)
            })?,
            offset,
            sign,
        },
    })
}

fn is_hex_digit_lc(b: u8) -> bool {
    match b {
        b'0'..=b'9' => true,
        b'a'..=b'f' => true,
        _ => false,
    }
}

pub(crate) fn parse_tag_nom(i: &[u8]) -> IResult<&[u8], Tag, Error> {
    const NL: &[u8] = b"\n";
    let (i, target) = terminated(
        preceded(
            tag(b"object "),
            take_while_m_n(40usize, 40, is_hex_digit_lc),
        ),
        tag(NL),
    )(i)
    .map_err(Error::context("object <40 lowercase hex char>"))?;
    let (i, kind) = terminated(preceded(tag(b"type "), take_while1(is_alphabetic)), tag(NL))(i)
        .map_err(Error::context("type <object kind>"))?;
    let kind = crate::Kind::from_bytes(kind)?;
    unimplemented!("parse message nom")
}

pub(crate) fn parse_message_nom(i: &[u8]) -> IResult<&[u8], (Option<&BStr>, Option<&BStr>), Error> {
    let (i, _) = tag(b"\n")(i)?;
    unimplemented!("parse message nom")
}

fn parse_message<'data>(
    d: &'data [u8],
    mut lines: impl Iterator<Item = &'data [u8]>,
) -> Result<(Option<&'data BStr>, Option<&'data BStr>), Error> {
    const PGP_SIGNATURE_BEGIN: &[u8] = b"-----BEGIN PGP SIGNATURE-----";
    const PGP_SIGNATURE_END: &[u8] = b"-----END PGP SIGNATURE-----";

    Ok(match lines.next() {
        Some(l) if l.is_empty() => {
            let msg_begin = 0; // TODO: use nom to parse this or do it without needing nightly
            if msg_begin >= d.len() {
                return Err(Error::ParseError(
                    "Message separator was not followed by message",
                    d.to_owned(),
                ));
            }
            let mut msg_end = d.len();
            let mut pgp_signature = None;
            if let Some(_pgp_begin_line) = lines.find(|l| l.starts_with(PGP_SIGNATURE_BEGIN)) {
                match lines.find(|l| l.starts_with(PGP_SIGNATURE_END)) {
                    None => {
                        return Err(Error::ParseError(
                            "Didn't find end of signature marker",
                            d.to_owned(),
                        ))
                    }
                    Some(_) => {
                        msg_end = d.len(); // TODO: use nom to parse this or do it without needing nightly
                        pgp_signature = Some((&d[msg_end..]).as_bstr())
                    }
                }
            }
            (Some((&d[msg_begin..msg_end]).as_bstr()), pgp_signature)
        }
        Some(l) => {
            return Err(Error::ParseError(
                "Expected empty newline to separate message",
                l.to_owned(),
            ))
        }
        None => (None, None),
    })
}

impl<'data> Tag<'data> {
    pub fn target(&self) -> crate::Id {
        <[u8; 20]>::from_hex(self.target).expect("prior validation")
    }
    pub fn from_bytes(d: &'data [u8]) -> Result<Tag<'data>, Error> {
        let mut lines = d.split(|&b| b == b'\n');
        let (target, target_kind, name, signature) =
            match (lines.next(), lines.next(), lines.next(), lines.next()) {
                (Some(target), Some(kind), Some(name), Some(tagger)) => {
                    let (_, target) = split2_at_space(target, |f, v| {
                        f == b"object" && v.len() == 40 && <[u8; 20]>::from_hex(v).is_ok()
                    })?;
                    let kind = split2_at_space(kind, |f, _v| f == b"type")
                        .and_then(|(_, kind)| crate::Kind::from_bytes(kind).map_err(Into::into))?;
                    let (_, name) = split2_at_space(name, |f, _v| f == b"tag")?;
                    let (_, rest) = split2_at_space(tagger, |f, _v| f == b"tagger")?;
                    (
                        target.as_bstr(),
                        kind,
                        name.as_bstr(),
                        parse_signature(rest)?,
                    )
                }
                _ => {
                    return Err(Error::ParseError(
                        "Expected four lines: target, type, tag and tagger",
                        d.to_owned(),
                    ))
                }
            };

        let (message, pgp_signature) = parse_message(d, &mut lines)?;

        Ok(Tag {
            target,
            name,
            target_kind,
            message,
            signature,
            pgp_signature,
        })
    }
}
