//! Serialize data into the bytecode protocol.
use bytes::{BufMut, Bytes};

use crate::error::{Error, Result};

// Helper macro to safely convert an usize expression into a signed
// integer.  If the conversion is not possible the macro issues a
// `CodecError`, otherwise returns the expression
// in the requested target type.
macro_rules! try_usize_to_int {
    // ~ $ttype should actually be a 'ty' ... but rust complains for
    // some reason :/
    ($value:expr, $ttype:ident) => {{
        let maxv = $ttype::MAX;
        let x: usize = $value;
        if (x as u64) <= (maxv as u64) {
            x as $ttype
        } else {
            return Err(Error::EncodingError);
        }
    }};
}

pub trait ToByte {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()>;
}

impl<'a, T: ToByte + 'a + ?Sized> ToByte for &'a T {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        (*self).encode(buffer)
    }
}

impl ToByte for bool {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_i8(*self as i8);
        Ok(())
    }
}

impl ToByte for i8 {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_i8(*self);
        Ok(())
    }
}

impl ToByte for i16 {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_i16(*self);
        Ok(())
    }
}

impl ToByte for i32 {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_i32(*self);
        Ok(())
    }
}

impl ToByte for u32 {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_u32(*self);
        Ok(())
    }
}

impl ToByte for i64 {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        buffer.put_i64(*self);
        Ok(())
    }
}

fn zigzag_encode(from: usize) -> u64 {
    ((from << 1) ^ (from >> 63)) as u64
}

pub const MSB: u8 = 0b1000_0000;
impl ToByte for usize {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        let mut n: u64 = zigzag_encode(*self);

        while n >= 0x80 {
            buffer.put_u8(MSB | (n as u8));
            n >>= 7;
        }

        buffer.put_u8(n as u8);

        Ok(())
    }
}

impl ToByte for str {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        let l = try_usize_to_int!(self.len(), i16);
        buffer.put_i16(l);
        buffer.put(self.as_bytes());
        Ok(())
    }
}

impl ToByte for String {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        let l = try_usize_to_int!(self.len(), i16);
        buffer.put_i16(l);
        buffer.put(self.as_bytes());
        Ok(())
    }
}

#[test]
fn test_string_too_long() {
    use std::str;

    let s = vec![b'a'; i16::MAX as usize + 1];
    let s = unsafe { str::from_utf8_unchecked(&s) };
    let mut buf = Vec::new();
    match s.encode(&mut buf) {
        Err(crate::error::Error::EncodingError) => {}
        _ => panic!(),
    }
    assert!(buf.is_empty());
}

impl<V: ToByte> ToByte for [V] {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        encode_as_array(buffer, self, |buffer, x| x.encode(buffer))
    }
}

impl ToByte for [u8] {
    fn encode<T: BufMut>(&self, buffer: &mut T) -> Result<()> {
        let l = try_usize_to_int!(self.len(), i32);
        buffer.put_i32(l);
        buffer.put(self);
        Ok(())
    }
}

// ~ this allows to render a slice of various types (typically &str
// and String) as strings
pub struct AsStrings<'a, T>(pub &'a [T]);

impl<'a, T: AsRef<str> + 'a> ToByte for AsStrings<'a, T> {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        encode_as_array(buffer, self.0, |buffer, x| x.as_ref().encode(buffer))
    }
}

/// ~ Renders the length of `xs` to `buffer` as the start of a
/// protocol array and then for each element of `xs` invokes `f`
/// assuming that function will render the element to the buffer.
pub fn encode_as_array<T, F, W>(buffer: &mut W, xs: &[T], mut f: F) -> Result<()>
where
    F: FnMut(&mut W, &T) -> Result<()>,
    W: BufMut,
{
    let l = try_usize_to_int!(xs.len(), i32);
    buffer.put_i32(l);
    for x in xs {
        f(buffer, x)?;
    }
    Ok(())
}

fn _encode_struct_as_array<T, F, W>(buffer: &mut W, xs: &[T], mut f: F) -> Result<()>
where
    T: ToByte,
    F: FnMut(&mut W, &T) -> Result<()>,
    W: BufMut,
{
    let l = try_usize_to_int!(xs.len(), i32);
    buffer.put_i32(l);
    for x in xs {
        f(buffer, x)?;
    }
    Ok(())
}

impl ToByte for Option<&[u8]> {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        match *self {
            Some(xs) => xs.encode(buffer),
            None => (-1i32).encode(buffer),
        }
    }
}

impl ToByte for Option<Bytes> {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        match self {
            Some(xs) => xs.encode(buffer),
            None => (-1i32).encode(buffer),
        }
    }
}

// why is this using i32 when strings need i16?
impl ToByte for Option<&str> {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        match *self {
            Some(xs) => xs.encode(buffer),
            None => (-1i32).encode(buffer),
        }
    }
}

impl ToByte for Option<String> {
    fn encode<W: BufMut>(&self, buffer: &mut W) -> Result<()> {
        match self {
            Some(xs) => xs.encode(buffer),
            None => (-1i16).encode(buffer),
        }
    }
}

#[test]
fn codec_i8() {
    let mut buf = vec![];
    let orig: i8 = 5;

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [5]);
}

#[test]
fn codec_i16() {
    let mut buf = vec![];
    let orig: i16 = 5;

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [0, 5]);
}

#[test]
fn codec_32() {
    let mut buf = vec![];
    let orig: i32 = 5;

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [0, 0, 0, 5]);
}

#[test]
fn codec_i64() {
    let mut buf = vec![];
    let orig: i64 = 5;

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [0, 0, 0, 0, 0, 0, 0, 5]);
}

#[test]
fn codec_varint_simple() {
    let mut buf = vec![];
    let orig: usize = 11;

    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [22]);
}

#[test]
fn codec_varint_twobyte() {
    let mut buf = vec![];
    let orig: usize = 260;

    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [136, 4]);
}

#[cfg(target_pointer_width = "64")]
#[test]
fn codec_varlong() {
    let mut buf = vec![];
    let orig: usize = 9223372036854775807;

    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [254, 255, 255, 255, 255, 255, 255, 255, 255, 1]);
}

#[test]
fn codec_string() {
    let mut buf = vec![];
    let orig = "test".to_owned();

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [0, 4, 116, 101, 115, 116]);
}

#[test]
fn codec_vec_u8() {
    let mut buf = vec![];
    let orig: Vec<u8> = vec![1, 2, 3];

    // Encode into buffer
    orig.encode(&mut buf).unwrap();
    assert_eq!(buf, [0, 0, 0, 3, 1, 2, 3]);
}

#[test]
fn codec_as_strings() {
    macro_rules! enc_dec_cmp {
        ($orig:expr) => {{
            let orig = $orig;

            // Encode into buffer
            let mut buf = Vec::new();
            AsStrings(&orig).encode(&mut buf).unwrap();
            assert_eq!(
                buf,
                [0, 0, 0, 2, 0, 3, b'a', b'b', b'c', 0, 4, b'd', b'e', b'f', b'g']
            );
        }};
    }

    {
        // slice of &str
        let orig: &[&str] = &["abc", "defg"];
        enc_dec_cmp!(orig);
    }

    {
        // vec of &str
        let orig: Vec<&str> = vec!["abc", "defg"];
        enc_dec_cmp!(orig);
    }

    {
        // vec of String
        let orig: Vec<String> = vec!["abc".to_owned(), "defg".to_owned()];
        enc_dec_cmp!(orig);
    }
}
