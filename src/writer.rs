use std::io::{ Write, Result, Error, ErrorKind };
use std::string::ToString;

use byteorder::{ BigEndian, LittleEndian, WriteBytesExt, ByteOrder };

use ply::*;

pub enum NewLine {
    N,
    R,
    RN
}

pub trait ToElement<P> {
    fn to_element(&self, element_def: &ElementDef) -> Result<DefaultElement>;
}

impl ToElement<DefaultElement> for DefaultElement {
    // simple identity
    fn to_element(&self, _props_def: &ElementDef) -> Result<DefaultElement> {
        Ok(self.clone())
    }
}


use std::marker::PhantomData;
pub struct Writer<P: ToElement<P> + PropertyAccess> {
    /// Should be fairly efficient, se `as_bytes()` in https://doc.rust-lang.org/src/collections/string.rs.html#1001
    new_line: String,
    phantom: PhantomData<P>,
}
macro_rules! get_prop(
    // TODO: errror
    ($e:expr) => (match $e {None => return Ok(17), Some(x) => x})
);

impl<P: ToElement<P> + PropertyAccess> Writer<P> {
    pub fn new() -> Self {
        Writer {
            new_line: "\r\n".to_string(),
            phantom: PhantomData,
        }
    }
    pub fn set_newline(&mut self, new_line: NewLine) {
        self.new_line = match new_line {
            NewLine::R => "\r".to_string(),
            NewLine::N => "\n".to_string(),
            NewLine::RN => "\r\n".to_string(),
        };
    }
    // TODO: think about masking and valid/invalid symbols
    // TODO: make consistency check
    pub fn write_ply<T: Write>(&mut self, out: &mut T, ply: &Ply<P>) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_header(out, &ply.header));
        written += try!(self.write_payload(out, &ply.payload, &ply.header));
        out.flush().unwrap();
        Ok(written)
    }
    pub fn write_line_magic_number<T: Write>(&self, out: &mut T) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("ply".as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_format<T: Write>(&self, out: &mut T, encoding: &Encoding, version: &Version) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("format ".as_bytes()));
        written += try!(self.write_encoding(out, encoding));
        written += try!(out.write(format!(" {}.{}", version.major, version.minor).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_comment<T: Write>(&self, out: &mut T, comment: &Comment) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("comment {}", comment).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_obj_info<T: Write>(&self, out: &mut T, obj_info: &ObjInfo) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("obj_info {}", obj_info).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_element_definition<T: Write>(&self, out: &mut T, element: &ElementDef) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("element {} {}", element.name, element.count).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_property_definition<T: Write>(&self, out: &mut T, property: &PropertyDef) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("property ".as_bytes()));
        written += try!(self.write_property_type(out, &property.data_type));
        written += try!(out.write(" ".as_bytes()));
        written += try!(out.write(property.name.as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    /// Writes the element line and all the property definitions
    pub fn write_element_definition<T: Write>(&self, out: &mut T, element: &ElementDef) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_line_element_definition(out, &element));
        for (_, p) in &element.properties {
            written += try!(self.write_line_property_definition(out, &p));
        }
        Ok(written)
    }
    pub fn write_line_end_header<T: Write>(&mut self, out: &mut T) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("end_header".as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_header<T: Write>(&mut self, out: &mut T, header: &Header) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_line_magic_number(out));
        written += try!(self.write_line_format(out, &header.encoding, &header.version));
        for c in &header.comments {
            written += try!(self.write_line_comment(out, c));
        }
        for oi in &header.obj_infos {
            written += try!(self.write_line_obj_info(out, oi));
        }
        for (_, e) in &header.elements {
            written += try!(self.write_element_definition(out, &e));
        }
        written += try!(self.write_line_end_header(out));
        Ok(written)
    }

    fn write_encoding<T: Write>(&self, out: &mut T, encoding: &Encoding) -> Result<usize> {
        let s = match *encoding {
            Encoding::Ascii => "ascii",
            Encoding::BinaryBigEndian => "binary_big_endian",
            Encoding::BinaryLittleEndian => "binary_little_endian",
        };
        out.write(s.as_bytes())
    }
    fn write_property_type<T: Write>(&self, out: &mut T, data_type: &PropertyType) -> Result<usize> {
        match *data_type {
            PropertyType::Scalar(ref scalar_type) => self.write_scalar_type(out, &scalar_type),
            PropertyType::List(ref index_type, ref content_type) => {
                let mut written = try!(out.write("list ".as_bytes()));
                match *index_type {
                    ScalarType::Float => return Err(Error::new(ErrorKind::InvalidInput, "List index can not be of type float.")),
                    ScalarType::Double => return Err(Error::new(ErrorKind::InvalidInput, "List index can not be of type double.")),
                    _ => (),
                };
                written += try!(self.write_scalar_type(out, &index_type));
                written += try!(out.write(" ".as_bytes()));
                written += try!(self.write_scalar_type(out, &content_type));
                Ok(written)
            }
        }
    }
    fn write_scalar_type<T: Write>(&self, out: &mut T, scalar_type: &ScalarType) -> Result<usize> {
        match *scalar_type {
            ScalarType::Char => out.write("char".as_bytes()),
            ScalarType::UChar => out.write("uchar".as_bytes()),
            ScalarType::Short => out.write("short".as_bytes()),
            ScalarType::UShort => out.write("ushort".as_bytes()),
            ScalarType::Int => out.write("int".as_bytes()),
            ScalarType::UInt => out.write("uint".as_bytes()),
            ScalarType::Float => out.write("float".as_bytes()),
            ScalarType::Double => out.write("double".as_bytes()),
        }
    }
    ///// Payload
    pub fn write_payload<T: Write>(&mut self, out: &mut T, payload: &Payload<P>, header: &Header) -> Result<usize> {
        let mut written = 0;
        let element_defs = &header.elements;
        for (k, element_list) in payload {
            let element_def = &element_defs[k];
            written += try!(self.write_payload_of_element(out, element_list, element_def, header));
        }
        Ok(written)
    }
    pub fn write_payload_of_element<T: Write>(&mut self, out: &mut T, element_list: &Vec<P>, element_def: &ElementDef, header: &Header) -> Result<usize> {
        let mut written = 0;
        match header.encoding {
            Encoding::Ascii => for e in element_list {
                let raw_element = try!(e.to_element(element_def));
                written += try!(self.__write_ascii_element(out, &raw_element));
            },
            Encoding::BinaryBigEndian => for element in element_list {
                written += try!(self.__write_binary_element::<T, BigEndian>(out, element, &element_def));
            },
            Encoding::BinaryLittleEndian => for element in element_list {
                written += try!(self.__write_binary_element::<T, LittleEndian>(out, element, &element_def));
            }
        }
        Ok(written)
    }
    pub fn write_ascii_element<T: Write>(&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        let raw_element = try!(element.to_element(element_def));
        self.__write_ascii_element(out, &raw_element)
    }
    pub fn write_big_endian_element<T: Write> (&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        self.__write_binary_element::<T, BigEndian>(out, element, element_def)
    }
    pub fn write_little_endian_element<T: Write> (&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        self.__write_binary_element::<T, BigEndian>(out, element, element_def)
    }

    // private payload
    fn __write_binary_element<T: Write, B: ByteOrder>(&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        let mut written = 0;
        for (k, property_def) in &element_def.properties {
            match property_def.data_type {
                PropertyType::Scalar(ref scalar_type) => {
                    written += match *scalar_type {
                        ScalarType::Char => {try!(out.write_i8(get_prop!(element.get_char(k)))); 1},
                        ScalarType::UChar => {try!(out.write_u8(get_prop!(element.get_uchar(k)))); 1},
                        ScalarType::Short => {try!(out.write_i16::<B>(get_prop!(element.get_short(k)))); 2},
                        ScalarType::UShort => {try!(out.write_u16::<B>(get_prop!(element.get_ushort(k)))); 2},
                        ScalarType::Int => {try!(out.write_i32::<B>(get_prop!(element.get_int(k)))); 4},
                        ScalarType::UInt => {try!(out.write_u32::<B>(get_prop!(element.get_uint(k)))); 4},
                        ScalarType::Float => {try!(out.write_f32::<B>(get_prop!(element.get_float(k)))); 4},
                        ScalarType::Double => {try!(out.write_f64::<B>(get_prop!(element.get_double(k)))); 8},
                    };
                },
                PropertyType::List(ref index_type, ref scalar_type) => {
                    let vec_len = element_def.count;
                    written += match *index_type {
                        ScalarType::Char => {try!(out.write_i8(vec_len as i8)); 1},
                        ScalarType::UChar => {try!(out.write_u8(vec_len as u8)); 1},
                        ScalarType::Short => {try!(out.write_i16::<B>(vec_len as i16)); 2},
                        ScalarType::UShort => {try!(out.write_u16::<B>(vec_len as u16)); 2},
                        ScalarType::Int => {try!(out.write_i32::<B>(vec_len as i32)); 4},
                        ScalarType::UInt => {try!(out.write_u32::<B>(vec_len as u32)); 4},
                        ScalarType::Float => return Err(Error::new(ErrorKind::InvalidInput, "Index of list must be an integer type, float declared in PropertyType.")),
                        ScalarType::Double => return Err(Error::new(ErrorKind::InvalidInput, "Index of list must be an integer type, double declared in PropertyType.")),
                    };

                    written += match *scalar_type {
                        ScalarType::Char => try!(self.write_binary_list::<T, i8, B>(get_prop!(element.get_list_char(k)), out, &|o, x| {try!(o.write_i8(*x)); Ok(1)} )),
                        ScalarType::UChar => try!(self.write_binary_list::<T, u8, B>(get_prop!(element.get_list_uchar(k)), out, &|o, x| {try!(o.write_u8(*x)); Ok(1)} )),
                        ScalarType::Short => try!(self.write_binary_list::<T, i16, B>(get_prop!(element.get_list_short(k)), out, &|o, x| {try!(o.write_i16::<B>(*x)); Ok(2)} )),
                        ScalarType::UShort => try!(self.write_binary_list::<T, u16, B>(get_prop!(element.get_list_ushort(k)), out, &|o, x| {try!(o.write_u16::<B>(*x)); Ok(2)} )),
                        ScalarType::Int => try!(self.write_binary_list::<T, i32, B>(get_prop!(element.get_list_int(k)), out, &|o, x| {try!(o.write_i32::<B>(*x)); Ok(4)} )),
                        ScalarType::UInt => try!(self.write_binary_list::<T, u32, B>(get_prop!(element.get_list_uint(k)), out, &|o, x| {try!(o.write_u32::<B>(*x)); Ok(4)} )),
                        ScalarType::Float => try!(self.write_binary_list::<T, f32, B>(get_prop!(element.get_list_float(k)), out, &|o, x| {try!(o.write_f32::<B>(*x)); Ok(4)} )),
                        ScalarType::Double => try!(self.write_binary_list::<T, f64, B>(get_prop!(element.get_list_double(k)), out, &|o, x| {try!(o.write_f64::<B>(*x)); Ok(8)} )),
                    }
                }
            }
        };
        Ok(written)
    }
    fn write_binary_list<T: Write, D, B: ByteOrder>(&self, list: &[D], out: &mut T, out_val: &Fn(&mut T, &D) -> Result<usize>) -> Result<usize> {
        let mut written = 0;
        for v in list {
            written += try!(out_val(out, v));
        }
        Ok(written)
    }
    fn __write_ascii_element<T: Write>(&self, out: &mut T, element: &DefaultElement) -> Result<usize> {
        let mut written = 0;
        let mut p_iter = element.iter();
        let (_name, prop_val) = p_iter.next().unwrap();
        written += try!(self.write_ascii_property(out, prop_val));
        loop {
            written += try!(out.write(" ".as_bytes()));
            let n = p_iter.next();
            if n == None {
                break;
            }
            let (_name, prop_val) = n.unwrap();
            written += try!(self.write_ascii_property(out, prop_val));
        }
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    fn write_ascii_property<T: Write>(&self, out: &mut T, data_element: &Property) -> Result<usize> {
         let result = match *data_element {
            Property::Char(ref v) => self.write_simple_value(v, out),
            Property::UChar(ref v) => self.write_simple_value(v, out),
            Property::Short(ref v) => self.write_simple_value(v, out),
            Property::UShort(ref v) => self.write_simple_value(v, out),
            Property::Int(ref v) => self.write_simple_value(v, out),
            Property::UInt(ref v) => self.write_simple_value(v, out),
            Property::Float(ref v) => self.write_simple_value(v, out),
            Property::Double(ref v) => self.write_simple_value(v, out),
            Property::ListChar(ref v) => self.write_ascii_list(v, out),
            Property::ListUChar(ref v) => self.write_ascii_list(v, out),
            Property::ListShort(ref v) => self.write_ascii_list(v, out),
            Property::ListUShort(ref v) => self.write_ascii_list(v, out),
            Property::ListInt(ref v) => self.write_ascii_list(v, out),
            Property::ListUInt(ref v) => self.write_ascii_list(v, out),
            Property::ListFloat(ref v) => self.write_ascii_list(v, out),
            Property::ListDouble(ref v) => self.write_ascii_list(v, out),
        };
        result
    }

    fn write_new_line<T: Write>(&self, out: &mut T) -> Result<usize> {
        out.write(self.new_line.as_bytes())
    }
    fn write_simple_value<T: Write, V: ToString>(&self, value: &V, out: &mut T) -> Result<usize> {
        out.write(value.to_string().as_bytes())
    }
    fn write_ascii_list<T: Write, D: Clone + Display>(&self, list: &Vec<D>, out: &mut T) -> Result<usize> {
        self.write_list(list, out, &|o, number| o.write(number.to_string().as_bytes()))
    }
    fn write_list<T: Write, D: Clone>(&self, list: &[D], out: &mut T, out_val: &Fn(&mut T, &D) -> Result<usize>) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(&list.len().to_string().as_bytes()));
        let b = " ".as_bytes();
        for v in list {
            written += try!(out.write(b));
            written += try!(out_val(out, v));
        }
        Ok(written)
    }
}
use std::fmt::Display;
