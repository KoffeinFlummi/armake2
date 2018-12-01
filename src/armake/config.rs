use std::str;
use std::io::{Read, Seek, Write, SeekFrom, Error, Cursor, BufReader, BufWriter};
use std::path::PathBuf;
use std::cell::{RefCell};
use std::iter::{Sum};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use colored::*;
use time::*;

use armake::preprocess::*;

mod config_grammar {
    include!(concat!(env!("OUT_DIR"), "/config_grammar.rs"));
}

pub struct Config {
    root_body: ConfigClass,
}

pub struct ConfigClass {
    parent: String,
    is_external: bool,
    is_deletion: bool,
    entries: Option<Vec<(String, ConfigEntry)>>,
}

pub enum ConfigEntry {
    StringEntry(String),
    FloatEntry(f32),
    IntEntry(i32),
    ArrayEntry(ConfigArray),
    ClassEntry(ConfigClass),
}

pub struct ConfigArray {
    is_expansion: bool,
    elements: Vec<ConfigArrayElement>,
}

pub enum ConfigArrayElement {
    StringElement(String),
    FloatElement(f32),
    IntElement(i32),
    ArrayElement(ConfigArray),
}

impl ConfigArrayElement {
    fn rapified_length(&self) -> usize {
        match self {
            ConfigArrayElement::StringElement(s) => s.len() + 2,
            ConfigArrayElement::FloatElement(f) => 5,
            ConfigArrayElement::IntElement(i) => 5,
            ConfigArrayElement::ArrayElement(a) => 1 + compressed_int_len(a.elements.len() as u32) +
                usize::sum(a.elements.iter().map(|e| e.rapified_length()))
        }
    }
}

impl ConfigArray {
    fn write<O: Write>(&self, output: &mut O) -> Result<(), &'static str> {
        output.write_all(b"{");
        for (key, value) in self.elements.iter().enumerate() {
            match value {
                ConfigArrayElement::ArrayElement(ref a) => {
                    a.write(output);
                },
                ConfigArrayElement::StringElement(s) => {
                    output.write_all(format!("\"{}\"", s.replace("\r", "\\r").replace("\n", "\\n").replace("\"", "\"\"")).as_bytes());
                },
                ConfigArrayElement::FloatElement(f) => {
                    output.write_all(format!("{:?}", f).as_bytes());
                },
                ConfigArrayElement::IntElement(i) => {
                    output.write_all(format!("{}", i).as_bytes());
                }
            }
            if key < self.elements.len() - 1 {
                output.write_all(b", ");
            }
        }
        output.write_all(b"}");
        Ok(())
    }

    fn write_rapified<O: Write>(&self, output: &mut O) -> Result<usize, Error> {
        let mut written = write_compressed_int(output, self.elements.len() as u32);

        for element in &self.elements {
            match element {
                ConfigArrayElement::StringElement(s) => {
                    output.write_all(&[0]);
                    output.write_all(s.as_bytes());
                    output.write_all(b"\0");
                    written += s.len() + 2;
                },
                ConfigArrayElement::FloatElement(f) => {
                    output.write_all(&[1]);
                    write_f32(output, *f);
                    written += 5;
                },
                ConfigArrayElement::IntElement(i) => {
                    output.write_all(&[2]);
                    output.write_i32::<LittleEndian>(*i)?;
                    written += 5;
                },
                ConfigArrayElement::ArrayElement(a) => {
                    output.write_all(&[3]);
                    written += 1 + a.write_rapified(output)?;
                },
            }
        }

        Ok(written)
    }

    fn read_rapified<I: Read + Seek>(mut input: &mut I) -> Result<ConfigArray, &'static str> {
        let num_elements: u32 = read_compressed_int(input);
        let mut elements: Vec<ConfigArrayElement> = Vec::with_capacity(num_elements as usize);

        for _i in 0..num_elements {
            let element_type: u8 = input.bytes().next().unwrap().unwrap();

            if element_type == 0 {
                elements.push(ConfigArrayElement::StringElement(read_cstring(input)));
            } else if element_type == 1 {
                elements.push(ConfigArrayElement::FloatElement(read_f32(input)));
            } else if element_type == 2 {
                elements.push(ConfigArrayElement::IntElement(read_i32(input)));
            } else if element_type == 3 {
                elements.push(ConfigArrayElement::ArrayElement(ConfigArray::read_rapified(input)?));
            } else {
                //return Err(&format!("Unrecognized array element type: {}", element_type));
                return Err("Unrecognized array element type: {}");
            }
        }

        Ok(ConfigArray {
            is_expansion: false,
            elements: elements
        })
    }
}

impl ConfigEntry {
    // without the name
    fn rapified_length(&self) -> usize {
        match self {
            ConfigEntry::StringEntry(s) => s.len() + 3,
            ConfigEntry::FloatEntry(f) => 6,
            ConfigEntry::IntEntry(i) => 6,
            ConfigEntry::ArrayEntry(a) => {
                let len = 1 + compressed_int_len(a.elements.len() as u32) +
                    usize::sum(a.elements.iter().map(|e| e.rapified_length()));
                if a.is_expansion { len + 4 } else { len }
            },
            ConfigEntry::ClassEntry(c) => {
                if c.is_external || c.is_deletion { 1 } else { 5 }
            }
        }
    }
}

impl ConfigClass {
    fn write<O: Write>(&self, mut output: &mut O, level: i32) -> Result<(), &'static str> {
        match &self.entries {
            Some(entries) => {
                if level > 0 && entries.len() > 0 {
                    output.write_all(b"\n");
                }
                for (key, value) in entries {
                    output.write_all(String::from("    ").repeat(level as usize).as_bytes());

                    //output.write_all(format!("{}{}\n", String::from("    ").repeat(level as usize), key).as_bytes());
                    match value {
                        ConfigEntry::ClassEntry(ref c) => {
                            if c.is_deletion {
                                output.write_all(format!("delete {};\n", key).as_bytes());
                            } else if c.is_external {
                                output.write_all(format!("class {};\n", key).as_bytes());
                            } else {
                                let parent = if c.parent == "" { String::from("") } else { format!(": {}", c.parent) };
                                match &c.entries {
                                    Some(entries) => {
                                        if entries.len() > 0 {
                                            output.write_all(format!("class {}{} {{", key, parent).as_bytes());
                                            c.write(output, level + 1);
                                            output.write_all(String::from("    ").repeat(level as usize).as_bytes());
                                            output.write_all(b"};\n");
                                        } else {
                                            output.write_all(format!("class {}{} {{}};\n", key, parent).as_bytes());
                                        }
                                    },
                                    None => {
                                        output.write_all(format!("class {}{} {{}};\n", key, parent).as_bytes());
                                    },
                                }
                            }
                        },
                        ConfigEntry::StringEntry(s) => {
                            output.write_all(format!("{} = \"{}\";\n", key, s.replace("\r", "\\r").replace("\n", "\\n").replace("\"", "\"\"")).as_bytes());
                        },
                        ConfigEntry::FloatEntry(f) => {
                            output.write_all(format!("{} = {:?};\n", key, f).as_bytes());
                        },
                        ConfigEntry::IntEntry(i) => {
                            output.write_all(format!("{} = {};\n", key, i).as_bytes());
                        },
                        ConfigEntry::ArrayEntry(ref a) => {
                            if a.is_expansion {
                                output.write_all(format!("{}[] += ", key).as_bytes());
                            } else {
                                output.write_all(format!("{}[] = ", key).as_bytes());
                            }
                            a.write(&mut output);
                            output.write_all(b";\n");
                        },
                    }
                }
            },
            None => {}
        }

        Ok(())
    }

    fn rapified_length(&self) -> usize {
        match &self.entries {
            Some(entries) => self.parent.len() + 1 +
                compressed_int_len(entries.len() as u32) +
                usize::sum(entries.iter().map(|(k,v)| {
                    k.len() + 1 + v.rapified_length() + match v {
                        ConfigEntry::ClassEntry(c) => c.rapified_length(),
                        _ => 0
                    }
                })),
            None => 0
        }
    }

    fn write_rapified<O: Write>(&self, output: &mut O, offset: usize) -> Result<usize, Error> {
        let mut written = 0;

        match &self.entries {
            Some(entries) => {
                output.write_all(self.parent.as_bytes());
                output.write_all(b"\0");
                written += self.parent.len() + 1;

                written += write_compressed_int(output, entries.len() as u32);

                let entries_len = usize::sum(entries.iter().map(|(k,v)| k.len() + 1 + v.rapified_length()));
                let mut class_offset = offset + written + entries_len;
                let mut class_bodies: Vec<Cursor<Box<[u8]>>> = Vec::new();
                let pre_entries = written;

                for (name, entry) in entries {
                    let pre_write = written;
                    match entry {
                        ConfigEntry::StringEntry(s) => {
                            output.write_all(&[1, 0]);
                            output.write_all(name.as_bytes());
                            output.write_all(b"\0");
                            output.write_all(s.as_bytes());
                            output.write_all(b"\0");
                            written += name.len() + s.len() + 4;
                        },
                        ConfigEntry::FloatEntry(f) => {
                            output.write_all(&[1, 1]);
                            output.write_all(name.as_bytes());
                            output.write_all(b"\0");
                            write_f32(output, *f);
                            written += name.len() + 7;
                        },
                        ConfigEntry::IntEntry(i) => {
                            output.write_all(&[1, 2]);
                            output.write_all(name.as_bytes());
                            output.write_all(b"\0");
                            output.write_i32::<LittleEndian>(*i)?;
                            written += name.len() + 7;
                        },
                        ConfigEntry::ArrayEntry(a) => {
                            output.write_all(if a.is_expansion { &[5] } else { &[2] });
                            if a.is_expansion {
                                output.write_all(&[1,0,0,0]);
                                written += 4;
                            }
                            output.write_all(name.as_bytes());
                            output.write_all(b"\0");
                            written += name.len() + 2 + a.write_rapified(output)?;
                        },
                        ConfigEntry::ClassEntry(c) => {
                            if c.is_external || c.is_deletion {
                                output.write_all(if c.is_deletion { &[4] } else { &[3] });
                                output.write_all(name.as_bytes());
                                output.write_all(b"\0");
                                written += name.len() + 2;
                            } else {
                                output.write_all(&[0]);
                                output.write_all(name.as_bytes());
                                output.write_all(b"\0");
                                output.write_u32::<LittleEndian>(class_offset as u32)?;
                                written += name.len() + 6;

                                let mut buffer: Box<[u8]> = vec![0; c.rapified_length()].into_boxed_slice();
                                let mut cursor: Cursor<Box<[u8]>> = Cursor::new(buffer);
                                class_offset += c.write_rapified(&mut cursor, class_offset).expect(&format!("Failed to rapify {}",name));

                                class_bodies.push(cursor);
                            }
                        }
                    }
                    assert_eq!(written - pre_write, entry.rapified_length() + name.len() + 1);
                }

                assert_eq!(written - pre_entries, entries_len);

                for cursor in class_bodies {
                    output.write_all(cursor.get_ref())?;
                    written += cursor.get_ref().len();
                }
            },
            None => { unreachable!() }
        }

        Ok(written)
    }

    fn read_rapified<I: Read + Seek>(input: &mut I, level: u32) -> Result<ConfigClass, &'static str> {
        let mut fp = 0;
        if level == 0 {
            input.seek(SeekFrom::Start(16));
        } else {
            let classbody_fp: u32 = read_u32(input);

            fp = input.seek(SeekFrom::Current(0)).unwrap();
            input.seek(SeekFrom::Start(classbody_fp.into()));
        }

        let parent = read_cstring(input);
        let num_entries: u32 = read_compressed_int(input);
        let mut entries: Vec<(String, ConfigEntry)> = Vec::with_capacity(num_entries as usize);

        for _i in 0..num_entries {
            let entry_type: u8 = input.bytes().next().unwrap().unwrap();

            if entry_type == 0 {
                let name = read_cstring(input);

                let class_entry = ConfigClass::read_rapified(input, level + 1)
                    .expect("Failed to read rapified class");
                entries.push((name, ConfigEntry::ClassEntry(class_entry)));
            } else if entry_type == 1 {
                let subtype: u8 = input.bytes().next().unwrap().unwrap();
                let name = read_cstring(input);

                if subtype == 0 {
                    entries.push((name, ConfigEntry::StringEntry(read_cstring(input))));
                } else if subtype == 1 {
                    entries.push((name, ConfigEntry::FloatEntry(read_f32(input))));
                } else if subtype == 2 {
                    entries.push((name, ConfigEntry::IntEntry(read_i32(input))));
                } else {
                    //return Err(&format!("Unrecognized variable entry subtype: {}", subtype) as &'static str);
                    return Err("Unrecognized variable entry subtype: {}");
                }
            } else if entry_type == 2 || entry_type == 5 {
                if entry_type == 5 {
                    input.seek(SeekFrom::Current(4)).unwrap();
                }

                let name = read_cstring(input);
                let mut array = ConfigArray::read_rapified(input).expect("Failed to read rapified array");
                array.is_expansion = entry_type == 5;

                entries.push((name.clone(), ConfigEntry::ArrayEntry(array)));
            } else if entry_type == 3 || entry_type == 4 {
                let name = read_cstring(input);
                let class_entry = ConfigClass {
                    parent: String::from(""),
                    is_external: entry_type == 3,
                    is_deletion: entry_type == 5,
                    entries: None
                };

                entries.push((name.clone(), ConfigEntry::ClassEntry(class_entry)));
            } else {
                //return Err(&format!("Unrecognized class entry type: {}", entry_type));
                return Err("Unrecognized class entry type: {}");
            }
        }

        if level > 0 {
            input.seek(SeekFrom::Start(fp));
        }

        Ok(ConfigClass {
            parent: parent,
            is_external: false,
            is_deletion: false,
            entries: Some(entries)
        })
    }
}

impl Config {
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), &'static str> {
        self.root_body.write(output, 0)
    }

    pub fn write_rapified<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        let mut writer = BufWriter::new(output);

        writer.write_all(b"\0raP")?;
        writer.write_all(b"\0\0\0\0\x08\0\0\0")?; // always_0, always_8

        let mut buffer: Box<[u8]> = vec![0; self.root_body.rapified_length()].into_boxed_slice();
        let mut cursor: Cursor<Box<[u8]>> = Cursor::new(buffer);
        self.root_body.write_rapified(&mut cursor, 16).expect("Failed to rapify root class");

        let enum_offset: u32 = 16 + cursor.get_ref().len() as u32;
        writer.write_u32::<LittleEndian>(enum_offset)?;

        writer.write_all(cursor.get_ref())?;

        writer.write_all(b"\0\0\0\0")?;

        Ok(())
    }

    pub fn to_cursor(&self) -> Result<Cursor<Box<[u8]>>, Error> {
        let len = self.root_body.rapified_length() + 20;

        let mut buffer: Box<[u8]> = vec![0; len].into_boxed_slice();
        let mut cursor: Cursor<Box<[u8]>> = Cursor::new(buffer);
        self.write_rapified(&mut cursor)?;

        Ok(cursor)
    }

    pub fn read<I: Read>(input: &mut I, path: Option<PathBuf>) -> Result<Config, String> {
        let mut t = precise_time_s();

        let mut buffer = String::new();
        input.read_to_string(&mut buffer).expect("Failed to read input file");

        //println!("reading: {}", precise_time_s() - t);
        t = precise_time_s();

        let (preprocessed, info) = preprocess(buffer, path).expect("Failed to preprocess config");

        //println!("preprocessing: {}", precise_time_s() - t);

        let result = config_grammar::config(&preprocessed);
        match result {
            Ok(config) => Ok(config),
            Err(pe) => {
                let line_origin = info.line_origins[pe.line - 1].0;
                let file_origin = match info.line_origins[pe.line - 1].1 {
                    Some(ref path) => format!("{}:", path.to_str().unwrap().to_string()),
                    None => "".to_string()
                };

                let line = preprocessed.split("\n").nth(pe.line - 1).unwrap();

                Err(format!("line {}{}:\n\n    {}\n    {}{}\n\nunexpected token \"{}\", expected: {:?}",
                    file_origin,
                    line_origin,
                    line,
                    " ".to_string().repeat(pe.column - 1),
                    "^".red().bold(),
                    line.chars().nth(pe.column - 1).unwrap(),
                    pe.expected))
            }
        }
    }

    pub fn read_rapified<I: Read + Seek>(input: &mut I) -> Result<Config, &'static str> {
        let mut reader = BufReader::new(input);

        let mut buffer = [0; 4];
        reader.read_exact(&mut buffer).unwrap();

        if &buffer != b"\0raP" {
            return Err("File doesn't seem to be a rapified config.");
        }

        Ok(Config {
            root_body: ConfigClass::read_rapified(&mut reader, 0)?
        })
    }
}

pub fn read_cstring<I: Read>(input: &mut I) -> String {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in input.bytes() {
        let b = byte.unwrap();
        if b == 0 {
            break;
        } else {
            bytes.push(b);
        }

    }
    String::from_utf8(bytes).unwrap()
}

fn read_compressed_int<I: Read>(input: &mut I) -> u32 {
    let mut i = 0;
    let mut result: u32 = 0;

    for byte in input.bytes() {
        let b: u32 = byte.unwrap().into();
        result = result | ((b & 0x7f) << (i * 7));

        if b < 0x80 {
            break;
        }

        i += 1;
    }

    result
}

fn write_compressed_int<O: Write>(output: &mut O, x: u32) -> usize {
    let mut temp = x;
    let mut len = 0;

    while temp > 0x7f {
        output.write(&[(0x80 | temp & 0x7f) as u8]);
        len += 1;
        temp &= !0x7f;
        temp >>= 7;
    }

    output.write(&[temp as u8]);
    len + 1
}

fn compressed_int_len(x: u32) -> usize {
    let mut temp = x;
    let mut len = 0;

    while temp > 0x7f {
        len += 1;
        temp &= !0x7f;
        temp >>= 7;
    }

    len + 1
}

// @todo
fn read_u32<I: Read>(input: &mut I) -> u32 {
    input.read_u32::<LittleEndian>().unwrap()
}

fn read_i32<I: Read>(input: &mut I) -> i32 {
    input.read_i32::<LittleEndian>().unwrap()
}

pub fn read_f32<I: Read>(input: &mut I) -> f32 {
    let mut buffer = [0; 4];
    input.read_exact(&mut buffer).unwrap();
    unsafe { std::mem::transmute::<[u8; 4], f32>(buffer) }
}

fn write_f32<O: Write>(output: &mut O, f: f32) {
    let buffer = unsafe { std::mem::transmute::<f32, [u8; 4]>(f) };
    output.write(&buffer);
}
