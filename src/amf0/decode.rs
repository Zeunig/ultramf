use super::marker;
use super::Value;
use crate::DecodePart;
use crate::amf3;
use crate::error::DecodeError;
use crate::{DecodeResult, Pair};
use byteorder::{BigEndian, ReadBytesExt};
use std::error::Error;
use std::io;
use std::io::Cursor;
use std::io::Read;
use std::time;

/// AMF0 decoder.
#[derive(Debug)]
pub struct Decoder<R> {
    inner: R,
    complexes: Vec<Value>,
    reader: u32,
    innervec: Vec<u8>
}
impl<R> Decoder<R> {
    /// Unwraps this `Decoder`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Get the reference to the underlying reader.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Get the mutable reference to the underlying reader.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R> Decoder<R>
where
    R: AsRef<[u8]> + io::Read + Copy
{
    pub fn new_from_array(inner: R) -> Self {
        let b = inner;
        Decoder {
            inner: b,
            complexes: Vec::new(),
            reader: 0,
            innervec: inner.as_ref().to_vec()
        }
    }

    pub fn decode_entire(&mut self) -> DecodeResult<Vec<Value>> {
    self.decode_entire_array()
    }

    fn decode_once(&mut self) -> DecodeResult<Value> { // TODO SELF READER INCREASING
        let marker = Cursor::new(&self.innervec[self.reader as usize..]).read_u8()?;
        let mut a: Vec<Value> = Vec::new();
        self.reader += 1;
        println!("MARKER : {}",marker);
        println!("REMAINING VEC :  {:?}",&self.innervec[self.reader as usize..]);
        match marker {
            marker::NUMBER => {a.push(self.decode_number_from_vec()?); self.reader += 8},
            marker::BOOLEAN => {a.push(self.decode_boolean_from_vec()?); self.reader += 1 },
            marker::STRING => {
                let temp = self.decode_string_from_vec()?;
                self.reader += temp.1;
                a.push(temp.0);
            }
            marker::OBJECT => {a.push(self.decode_object_from_vec()?);}
            marker::MOVIECLIP => return Err(DecodeError::Unsupported { marker }),
            marker::NULL => {a.push(Value::Null);},
            marker::UNDEFINED => {a.push(Value::Undefined);},
            marker::REFERENCE => {self.decode_reference_from_vec();}
            marker::ECMA_ARRAY => {self.decode_ecma_array_from_vec();}
            marker::OBJECT_END_MARKER => return Err(DecodeError::UnexpectedObjectEnd),
            marker::STRICT_ARRAY => {self.decode_strict_array_from_vec();}
            marker::DATE => {self.decode_date_from_vec();}
            marker::LONG_STRING => {let decoded = self.decode_long_string_from_vec()?; a.push(decoded.0); self.reader += decoded.1;}
            marker::UNSUPPORTED => return Err(DecodeError::Unsupported { marker }),
            marker::RECORDSET => return Err(DecodeError::Unsupported { marker }),
            marker::XML_DOCUMENT => {self.decode_xml_document_from_vec();}
            marker::TYPED_OBJECT => {self.decode_typed_object_from_vec();}
            marker::AVMPLUS_OBJECT => {self.decode_avmplus_from_vec();}
            _ => return Err(DecodeError::Unknown { marker }),
        }
        Ok(a.get(0).unwrap().to_owned())
    }
    fn decode_entire_array(&mut self) -> DecodeResult<Vec<Value>> {
        let mut a = Vec::new();
        while self.reader != self.innervec.len() as u32 {
            let marker = Cursor::new(&self.innervec[self.reader as usize..]).read_u8()?;
            self.reader += 1;
            println!("MARKER ENTIRE : {}",marker);
            match marker {
                marker::NUMBER => {a.push(self.decode_number_from_vec()?); self.reader += 8},
                marker::BOOLEAN => {a.push(self.decode_boolean_from_vec()?); self.reader += 1 },
                marker::STRING => {
                    let temp = self.decode_string_from_vec()?;
                    self.reader += temp.1;
                    a.push(temp.0);
                }
                marker::OBJECT => {a.push(self.decode_object_from_vec()?);}
                marker::MOVIECLIP => return Err(DecodeError::Unsupported { marker }),
                marker::NULL => {a.push(Value::Null);},
                marker::UNDEFINED => {a.push(Value::Undefined);},
                marker::REFERENCE => {a.push(self.decode_reference_from_vec()?); self.reader += 2}
                marker::ECMA_ARRAY => {a.push(self.decode_ecma_array_from_vec()?);} // TODO
                marker::OBJECT_END_MARKER => return Err(DecodeError::UnexpectedObjectEnd),
                marker::STRICT_ARRAY => {a.push(self.decode_strict_array_from_vec()?);}
                marker::DATE => {a.push(self.decode_date_from_vec()?);}
                marker::LONG_STRING => {let decoded = self.decode_long_string_from_vec()?; a.push(decoded.0); self.reader += decoded.1;},
                marker::UNSUPPORTED => return Err(DecodeError::Unsupported { marker }),
                marker::RECORDSET => return Err(DecodeError::Unsupported { marker }),
                marker::XML_DOCUMENT => {let decoded = self.decode_xml_document_from_vec()?; a.push(decoded.0); self.reader += decoded.1},
                marker::TYPED_OBJECT => {a.push(self.decode_typed_object_from_vec()?);}
                marker::AVMPLUS_OBJECT => return Err(DecodeError::Unsupported { marker }), //{self.decode_avmplus_from_vec();}
                _ => return Err(DecodeError::Unknown { marker }),
            }
            println!("{:?}",a);
            println!("{:?}",a.get(0).unwrap());
        }
        Ok(a)
    }

    fn decode_number_from_vec(&mut self) -> DecodeResult<Value> {
        let mut n = Cursor::new(&self.innervec[self.reader as usize..]);
        println!("N :  {:?}",n);
        let a = n.read_f64::<BigEndian>()?;
        Ok(Value::Number(a))
    }
    fn decode_boolean_from_vec(&mut self) -> DecodeResult<Value> {
        let b = Cursor::new(&self.innervec[self.reader as usize..]).read_u8()? != 0;
        Ok(Value::Boolean(b))
    }
    fn decode_string_from_vec(&mut self) -> DecodePart<Value> {
        let len = Cursor::new(&self.innervec[self.reader as usize..]).read_u16::<BigEndian>()? as usize; self.reader += 2;
        println!("{}",len);
        println!("L : {:?}",&self.innervec[self.reader as usize..self.reader as usize + len]);
        let a = self.read_utf8_from_vec(len).map(Value::String)?;
        Ok((a, len as u32))
    }
    fn read_utf8_from_vec(&mut self, len: usize) -> DecodeResult<String> {
        let mut buf = vec![0; len];
        Cursor::new(&self.innervec[self.reader as usize..]).read_exact(&mut buf)?;
        println!("buf : {:?}",buf);
        let utf8 = String::from_utf8(buf)?;
        println!("utf : {:?}",utf8);
        Ok(utf8)
    }

    fn decode_object_from_vec(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type_from_vec(|this| {
            let entries = this.decode_pairs_from_vec()?;
            Ok(Value::Object {
                class_name: None,
                entries,
            })
        })
    }
    fn decode_complex_type_from_vec<F>(&mut self, f: F) -> DecodeResult<Value>
    where
        F: FnOnce(&mut Self) -> DecodeResult<Value>,
    {
        let index = self.complexes.len();
        self.complexes.push(Value::Null);
        let value = f(self)?;
        self.complexes[index] = value.clone();
        Ok(value)
    }
    fn decode_pairs_from_vec(&mut self) -> DecodeResult<Vec<Pair<String, Value>>> {
        let mut entries = Vec::new();
        loop {
            println!("REMAINING VEC FROM DECODE PAIRS FROM VEC FUNCTION : {:?}", &self.innervec[self.reader as usize..]);
            let len = Cursor::new(&self.innervec[self.reader as usize..]).read_u16::<BigEndian>()? as usize;
            println!("len : {}",len);
            self.reader += 2;
            let key = self.read_utf8_from_vec(len)?;
            self.reader += len as u32;
            match self.decode_once() {
                Ok(value) => {
                    entries.push(Pair { key, value });
                }
                Err(DecodeError::UnexpectedObjectEnd) if key.is_empty() => break,
                Err(e) => {println!("NOOOO : {}",e);return Err(e)},
            }
            println!("ENTRIES : {:?}",entries);
        }
        Ok(entries)
    }
    fn decode_reference_from_vec(&mut self) -> DecodeResult<Value> {
        let index = Cursor::new(&self.innervec[self.reader as usize..]).read_u16::<BigEndian>()? as usize;
        self.complexes
            .get(index)
            .ok_or(DecodeError::OutOfRangeReference { index })
            .and_then(|v| {
                if *v == Value::Null {
                    Err(DecodeError::CircularReference { index })
                } else {
                    Ok(v.clone())
                }
            })
    }
    fn decode_ecma_array_from_vec(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type_from_vec(|this: &mut Decoder<R>| {
            let _count = Cursor::new(&this.innervec[this.reader as usize..]).read_u32::<BigEndian>()? as usize;
            let entries = this.decode_pairs_from_vec()?; // TODO TODO TODO TODO TODO TODO TODO TODO 
            Ok(Value::EcmaArray { entries })
        })
    }
    fn decode_strict_array_from_vec(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type_from_vec(|this: &mut Decoder<R>| {
            let count = Cursor::new(&this.innervec[this.reader as usize..]).read_u32::<BigEndian>()? as usize; this.reader += 4;
            let entries = (0..count)
                .map(|_| this.decode_once())
                .collect::<DecodeResult<_>>()?;
            Ok(Value::Array { entries })
        })
    }
    fn decode_date_from_vec(&mut self) -> DecodeResult<Value> {
        let millis = Cursor::new(&self.innervec[self.reader as usize..]).read_f64::<BigEndian>()?; self.reader += 8;
        let time_zone = Cursor::new(&self.innervec[self.reader as usize..]).read_i16::<BigEndian>()?; self.reader += 2;
        if !(millis.is_finite() && millis.is_sign_positive()) {
            Err(DecodeError::InvalidDate { millis })
        } else {
            Ok(Value::Date {
                unix_time: time::Duration::from_millis(millis as u64),
                time_zone,
            })
        }
    }
    fn decode_long_string_from_vec(&mut self) -> DecodePart<Value> {
        let len = Cursor::new(&self.innervec[self.reader as usize..]).read_u32::<BigEndian>()? as usize; self.reader += 4;
        let string = self.read_utf8_from_vec(len).map(Value::String)?;
        Ok((string, len as u32))
    }
    fn decode_xml_document_from_vec(&mut self) -> DecodePart<Value> {
        let len = Cursor::new(&self.innervec[self.reader as usize..]).read_u32::<BigEndian>()? as usize;
        Ok((self.read_utf8_from_vec(len).map(Value::XmlDocument)?, len as u32))
    }
    fn decode_typed_object_from_vec(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type(|this| {
            let len = Cursor::new(&this.innervec[this.reader as usize..]).read_u16::<BigEndian>()? as usize;
            let class_name = this.read_utf8_from_vec(len)?;
            let entries = this.decode_pairs_from_vec()?;
            Ok(Value::Object {
                class_name: Some(class_name),
                entries,
            })
        })
    }
    fn decode_avmplus_from_vec(&mut self) -> DecodeResult<Value> {
        let value = amf3::Decoder::new(&mut self.inner).decode()?;
        Ok(Value::AvmPlus(value))
    }
    
}

impl<R> Decoder<R>
where
    R: io::Read,
{
    /// Makes a new instance.
    pub fn new(inner: R) -> Self {
        Decoder {
            inner,
            complexes: Vec::new(),
            reader: 0,
            innervec: Vec::new()
        }
    }

    /// Decodes a AMF0 value.
    pub fn decode(&mut self) -> DecodeResult<Value> {
        self.decode_value()
    }

    /// Clear the reference table of this decoder.
    ///
    /// > Note that object reference indices are local to each message body.
    /// > Serializers and deserializers must reset reference indices to 0 each time a new message is processed.
    /// >
    /// > [AMF 0 Specification: 4.1.3 AMF Message](http://download.macromedia.com/pub/labs/amf/amf0_spec_121207.pdf)
    pub fn clear_reference_table(&mut self) {
        self.complexes.clear();
    }

    fn decode_value(&mut self) -> DecodeResult<Value> {
        let marker = self.inner.read_u8()?;
        match marker {
            marker::NUMBER => self.decode_number(),
            marker::BOOLEAN => self.decode_boolean(),
            marker::STRING => self.decode_string(),
            marker::OBJECT => self.decode_object(),
            marker::MOVIECLIP => Err(DecodeError::Unsupported { marker }),
            marker::NULL => Ok(Value::Null),
            marker::UNDEFINED => Ok(Value::Undefined),
            marker::REFERENCE => self.decode_reference(),
            marker::ECMA_ARRAY => self.decode_ecma_array(),
            marker::OBJECT_END_MARKER => Err(DecodeError::UnexpectedObjectEnd),
            marker::STRICT_ARRAY => self.decode_strict_array(),
            marker::DATE => self.decode_date(),
            marker::LONG_STRING => self.decode_long_string(),
            marker::UNSUPPORTED => Err(DecodeError::Unsupported { marker }),
            marker::RECORDSET => Err(DecodeError::Unsupported { marker }),
            marker::XML_DOCUMENT => self.decode_xml_document(),
            marker::TYPED_OBJECT => self.decode_typed_object(),
            marker::AVMPLUS_OBJECT => self.decode_avmplus(),
            _ => Err(DecodeError::Unknown { marker }),
        }
    }
    fn decode_number(&mut self) -> DecodeResult<Value> {
        let n = self.inner.read_f64::<BigEndian>()?;
        Ok(Value::Number(n))
    }
    fn decode_boolean(&mut self) -> DecodeResult<Value> {
        let b = self.inner.read_u8()? != 0;
        Ok(Value::Boolean(b))
    }
    fn decode_string(&mut self) -> DecodeResult<Value> {
        let len = self.inner.read_u16::<BigEndian>()? as usize;
        self.read_utf8(len).map(Value::String)
    }
    fn decode_object(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type(|this| {
            let entries = this.decode_pairs()?;
            Ok(Value::Object {
                class_name: None,
                entries,
            })
        })
    }
    fn decode_reference(&mut self) -> DecodeResult<Value> {
        let index = self.inner.read_u16::<BigEndian>()? as usize;
        self.complexes
            .get(index)
            .ok_or(DecodeError::OutOfRangeReference { index })
            .and_then(|v| {
                if *v == Value::Null {
                    Err(DecodeError::CircularReference { index })
                } else {
                    Ok(v.clone())
                }
            })
    }
    fn decode_ecma_array(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type(|this| {
            let _count = this.inner.read_u32::<BigEndian>()? as usize;
            let entries = this.decode_pairs()?;
            Ok(Value::EcmaArray { entries })
        })
    }
    fn decode_strict_array(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type(|this| {
            let count = this.inner.read_u32::<BigEndian>()? as usize;
            let entries = (0..count)
                .map(|_| this.decode_value())
                .collect::<DecodeResult<_>>()?;
            Ok(Value::Array { entries })
        })
    }
    fn decode_date(&mut self) -> DecodeResult<Value> {
        let millis = self.inner.read_f64::<BigEndian>()?;
        let time_zone = self.inner.read_i16::<BigEndian>()?;
        if !(millis.is_finite() && millis.is_sign_positive()) {
            Err(DecodeError::InvalidDate { millis })
        } else {
            Ok(Value::Date {
                unix_time: time::Duration::from_millis(millis as u64),
                time_zone,
            })
        }
    }
    fn decode_long_string(&mut self) -> DecodeResult<Value> {
        let len = self.inner.read_u32::<BigEndian>()? as usize;
        self.read_utf8(len).map(Value::String)
    }
    fn decode_xml_document(&mut self) -> DecodeResult<Value> {
        let len = self.inner.read_u32::<BigEndian>()? as usize;
        self.read_utf8(len).map(Value::XmlDocument)
    }
    fn decode_typed_object(&mut self) -> DecodeResult<Value> {
        self.decode_complex_type(|this| {
            let len = this.inner.read_u16::<BigEndian>()? as usize;
            let class_name = this.read_utf8(len)?;
            let entries = this.decode_pairs()?;
            Ok(Value::Object {
                class_name: Some(class_name),
                entries,
            })
        })
    }
    fn decode_avmplus(&mut self) -> DecodeResult<Value> {
        let value = amf3::Decoder::new(&mut self.inner).decode()?;
        Ok(Value::AvmPlus(value))
    }

    fn read_utf8(&mut self, len: usize) -> DecodeResult<String> {
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf)?;
        println!("buf : {:?}",buf);
        let utf8 = String::from_utf8(buf)?;
        println!("utf : {:?}",utf8);
        Ok(utf8)
    }
    fn decode_pairs(&mut self) -> DecodeResult<Vec<Pair<String, Value>>> {
        let mut entries = Vec::new();
        loop {
            let len = self.inner.read_u16::<BigEndian>()? as usize;
            let key = self.read_utf8(len)?;
            match self.decode_value() {
                Ok(value) => {
                    entries.push(Pair { key, value });
                }
                Err(DecodeError::UnexpectedObjectEnd) if key.is_empty() => break,
                Err(e) => return Err(e),
            }
        }
        Ok(entries)
    }
    fn decode_complex_type<F>(&mut self, f: F) -> DecodeResult<Value>
    where
        F: FnOnce(&mut Self) -> DecodeResult<Value>,
    {
        let index = self.complexes.len();
        self.complexes.push(Value::Null);
        let value = f(self)?;
        self.complexes[index] = value.clone();
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::super::marker;
    use super::super::Value;
    use crate::amf3;
    use crate::error::DecodeError;
    use crate::Pair;
    use std::f64;
    use std::io;
    use std::iter;
    use std::time;

    macro_rules! decode {
        ($file:expr) => {{
            let input = include_bytes!(concat!("../testdata/", $file));
            Value::read_from(&mut &input[..])
        }};
    }
    macro_rules! decode_eq {
        ($file:expr, $expected: expr) => {{
            let value = decode!($file).unwrap();
            assert_eq!(value, $expected)
        }};
    }
    macro_rules! decode_unexpected_eof {
        ($file:expr) => {{
            let result = decode!($file);
            match result {
                Err(DecodeError::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
                _ => assert!(false),
            }
        }};
    }

    #[test]
    fn decodes_boolean() {
        decode_eq!("amf0-boolean-true.bin", Value::Boolean(true));
        decode_eq!("amf0-boolean-false.bin", Value::Boolean(false));
        decode_unexpected_eof!("amf0-boolean-partial.bin");
    }
    #[test]
    fn decodes_null() {
        decode_eq!("amf0-null.bin", Value::Null);
    }
    #[test]
    fn decodes_undefined() {
        decode_eq!("amf0-undefined.bin", Value::Undefined);
    }
    #[test]
    fn decodes_number() {
        decode_eq!("amf0-number.bin", Value::Number(3.5));
        decode_eq!(
            "amf0-number-positive-infinity.bin",
            Value::Number(f64::INFINITY)
        );
        decode_eq!(
            "amf0-number-negative-infinity.bin",
            Value::Number(f64::NEG_INFINITY)
        );

        let is_nan = |v| {
            if let Value::Number(n) = v {
                n.is_nan()
            } else {
                false
            }
        };
        assert!(is_nan(decode!("amf0-number-quiet-nan.bin").unwrap()));
        assert!(is_nan(decode!("amf0-number-signaling-nan.bin").unwrap()));

        decode_unexpected_eof!("amf0-number-partial.bin");
    }
    #[test]
    fn decodes_string() {
        decode_eq!(
            "amf0-string.bin",
            Value::String("this is a テスト".to_string())
        );
        decode_eq!(
            "amf0-complex-encoded-string.bin",
            obj(
                None,
                &[
                    ("utf", s("UTF テスト")),
                    ("zed", n(5.0)),
                    ("shift", s("Shift テスト"))
                ][..]
            )
        );
        decode_unexpected_eof!("amf0-string-partial.bin");
    }
    #[test]
    fn decodes_long_string() {
        decode_eq!(
            "amf0-long-string.bin",
            Value::String(iter::repeat('a').take(0x10013).collect())
        );
        decode_unexpected_eof!("amf0-long-string-partial.bin");
    }
    #[test]
    fn decodes_xml_document() {
        decode_eq!(
            "amf0-xml-doc.bin",
            Value::XmlDocument("<parent><child prop=\"test\" /></parent>".to_string())
        );
        decode_unexpected_eof!("amf0-xml-document-partial.bin");
    }
    #[test]
    fn decodes_object() {
        decode_eq!(
            "amf0-object.bin",
            obj(
                None,
                &[("", s("")), ("foo", s("baz")), ("bar", n(3.14))][..]
            )
        );
        decode_eq!(
            "amf0-untyped-object.bin",
            obj(None, &[("foo", s("bar")), ("baz", Value::Null)][..])
        );
        assert_eq!(
            decode!("amf0-bad-object-end.bin"),
            Err(DecodeError::UnexpectedObjectEnd)
        );
        decode_unexpected_eof!("amf0-object-partial.bin");
    }
    #[test]
    fn decodes_typed_object() {
        decode_eq!(
            "amf0-typed-object.bin",
            obj(
                Some("org.amf.ASClass"),
                &[("foo", s("bar")), ("baz", Value::Null)]
            )
        );
        decode_unexpected_eof!("amf0-typed-object-partial.bin");
    }
    #[test]
    fn decodes_unsupported() {
        assert_eq!(
            decode!("amf0-movieclip.bin"),
            Err(DecodeError::Unsupported {
                marker: marker::MOVIECLIP
            })
        );
        assert_eq!(
            decode!("amf0-recordset.bin"),
            Err(DecodeError::Unsupported {
                marker: marker::RECORDSET
            })
        );
        assert_eq!(
            decode!("amf0-unsupported.bin"),
            Err(DecodeError::Unsupported {
                marker: marker::UNSUPPORTED
            })
        );
    }
    #[test]
    fn decodes_ecma_array() {
        let entries = es(&[("0", s("a")), ("1", s("b")), ("2", s("c")), ("3", s("d"))][..]);
        decode_eq!(
            "amf0-ecma-ordinal-array.bin",
            Value::EcmaArray { entries: entries }
        );
        decode_unexpected_eof!("amf0-ecma-array-partial.bin");

        let entries = es(&[("c", s("d")), ("a", s("b"))][..]);
        decode_eq!("amf0-hash.bin", Value::EcmaArray { entries: entries });
    }
    #[test]
    fn decodes_strict_array() {
        decode_eq!(
            "amf0-strict-array.bin",
            Value::Array {
                entries: vec![n(1.0), s("2"), n(3.0)]
            }
        );
        decode_unexpected_eof!("amf0-strict-array-partial.bin");
    }
    #[test]
    fn decodes_reference() {
        let object = obj(None, &[("foo", s("baz")), ("bar", n(3.14))][..]);
        let expected = obj(None, &[("0", object.clone()), ("1", object)][..]);
        decode_eq!("amf0-ref-test.bin", expected);
        decode_unexpected_eof!("amf0-reference-partial.bin");

        assert_eq!(
            decode!("amf0-bad-reference.bin"),
            Err(DecodeError::OutOfRangeReference { index: 0 })
        );
        assert_eq!(
            decode!("amf0-circular-reference.bin"),
            Err(DecodeError::CircularReference { index: 0 })
        );
    }
    #[test]
    fn decodes_date() {
        decode_eq!(
            "amf0-date.bin",
            Value::Date {
                unix_time: time::Duration::from_millis(1_590_796_800_000),
                time_zone: 0
            }
        );
        decode_eq!(
            "amf0-time.bin",
            Value::Date {
                unix_time: time::Duration::from_millis(1_045_112_400_000),
                time_zone: 0
            }
        );
        decode_unexpected_eof!("amf0-date-partial.bin");
        assert_eq!(
            decode!("amf0-date-minus.bin"),
            Err(DecodeError::InvalidDate { millis: -1.0 })
        );
        assert_eq!(
            decode!("amf0-date-invalid.bin"),
            Err(DecodeError::InvalidDate {
                millis: f64::INFINITY
            })
        );
    }
    #[test]
    fn decodes_avmplus() {
        let expected = amf3::Value::Array {
            assoc_entries: vec![],
            dense_entries: (1..4).map(amf3::Value::Integer).collect(),
        };
        decode_eq!("amf0-avmplus-object.bin", Value::AvmPlus(expected));
    }
    #[test]
    fn other_errors() {
        decode_unexpected_eof!("amf0-empty.bin");
        assert_eq!(
            decode!("amf0-unknown-marker.bin"),
            Err(DecodeError::Unknown { marker: 97 })
        );
    }

    fn s(s: &str) -> Value {
        Value::String(s.to_string())
    }
    fn n(n: f64) -> Value {
        Value::Number(n)
    }
    fn obj(name: Option<&str>, entries: &[(&str, Value)]) -> Value {
        Value::Object {
            class_name: name.map(|s| s.to_string()),
            entries: es(entries),
        }
    }
    fn es(entries: &[(&str, Value)]) -> Vec<Pair<String, Value>> {
        entries
            .iter()
            .map(|e| Pair {
                key: e.0.to_string(),
                value: e.1.clone(),
            })
            .collect()
    }
}
