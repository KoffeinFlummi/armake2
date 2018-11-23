use std::str;
use std::clone::Clone;
use std::io::{Read, Seek, Write, SeekFrom};
use std::cell::{RefCell};
use std::borrow::BorrowMut;

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

pub trait Derapify {
    fn derapify<O: Write + Clone>(&self, output: O) -> Result<(), &'static str>;

    fn derapify_indented<O: Write + Clone>(&self, output: O, level: i32) -> Result<(), &'static str>
        where Self: Sized
    {
        Ok(())
    }
}

impl Derapify for Config {
    fn derapify<O: Write + Clone>(&self, output: O) -> Result<(), &'static str> {
        self.root_body.derapify(output)
    }
}

impl Derapify for ConfigClass {
    fn derapify<O: Write + Clone>(&self, output: O) -> Result<(), &'static str> {
        self.derapify_indented(output, 0)
    }

    fn derapify_indented<O: Write + Clone>(&self, output: O, level: i32) -> Result<(), &'static str> {
        match &self.entries {
            Some(entries) => {
                if level > 0 && entries.len() > 0 {
                    output.clone().write(b"\n");
                }
                for (key, value) in entries {
                    output.clone().write(String::from("    ").repeat(level as usize).as_bytes());

                    //output.clone().write(format!("{}{}\n", String::from("    ").repeat(level as usize), key).as_bytes());
                    match value {
                        ConfigEntry::ClassEntry(ref c) => {
                            if c.is_deletion {
                                output.clone().write(format!("delete {};\n", key).as_bytes());
                            } else if c.is_external {
                                output.clone().write(format!("class {};\n", key).as_bytes());
                            } else {
                                let parent = if c.parent == "" { String::from("") } else { format!(": {}", c.parent) };
                                match &c.entries {
                                    Some(entries) => {
                                        if entries.len() > 0 {
                                            output.clone().write(format!("class {}{} {}", key, parent, "{").as_bytes());
                                            c.derapify_indented(output.clone(), level + 1);
                                            output.clone().write(String::from("    ").repeat(level as usize).as_bytes());
                                            output.clone().write(b"};\n");
                                        } else {
                                            output.clone().write(format!("class {}{} {};\n", key, parent, "{}").as_bytes());
                                        }
                                    },
                                    None => {
                                        output.clone().write(format!("class {}{} {};\n", key, parent, "{}").as_bytes());
                                    },
                                }
                            }
                        },
                        ConfigEntry::StringEntry(s) => {
                            output.clone().write(format!("{} = \"{}\";\n", key, s.replace("\r", "\\r").replace("\n", "\\n").replace("\"", "\"\"")).as_bytes());
                        },
                        ConfigEntry::FloatEntry(f) => {
                            output.clone().write(format!("{} = {};\n", key, f).as_bytes());
                        },
                        ConfigEntry::IntEntry(i) => {
                            output.clone().write(format!("{} = {};\n", key, i).as_bytes());
                        },
                        ConfigEntry::ArrayEntry(ref a) => {
                            if a.is_expansion {
                                output.clone().write(format!("{}[] += ", key).as_bytes());
                            } else {
                                output.clone().write(format!("{}[] = ", key).as_bytes());
                            }
                            a.derapify(output.clone());
                            output.clone().write(b";\n");
                        },
                    }
                }
            },
            None => {}
        }

        Ok(())
    }
}

impl Derapify for ConfigArray {
    fn derapify<O: Write + Clone>(&self, output: O) -> Result<(), &'static str> {
        output.clone().write(b"{");
        for (key, value) in self.elements.iter().enumerate() {
            match value {
                ConfigArrayElement::ArrayElement(ref a) => {
                    a.derapify(output.clone());
                },
                ConfigArrayElement::StringElement(s) => {
                    output.clone().write(format!("\"{}\"", s.replace("\r", "\\r").replace("\n", "\\n").replace("\"", "\"\"")).as_bytes());
                },
                ConfigArrayElement::FloatElement(f) => {
                    output.clone().write(format!("{}", f).as_bytes());
                },
                ConfigArrayElement::IntElement(i) => {
                    output.clone().write(format!("{}", i).as_bytes());
                }
            }
            if key < self.elements.len() - 1 {
                output.clone().write(b", ");
            }
        }
        output.clone().write(b"}");
        Ok(())
    }
}

fn read_cstring<I: Read>(input: &mut I) -> String {
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

fn read_u32<I: Read>(input: &mut I) -> u32 {
    // @todo
    let mut buffer = [0;4];
    input.read_exact(&mut buffer);
    //println!("{:?}", buffer);

    let mut result = 0;
    for i in 0..3 {
        result = result | ((buffer[i] as u32) << (i * 8))
    }
    result
}

fn read_i32<I: Read>(input: &mut I) -> i32 {
    read_u32(input) as i32
}

fn read_f32<I: Read>(input: &mut I) -> f32 {
    let mut buffer = [0; 4];
    input.read_exact(&mut buffer);
    unsafe { std::mem::transmute::<[u8; 4], f32>(buffer) }
}

fn read_rapified_array<I: Read + Seek + Clone>(mut input: &mut I) -> Result<ConfigArray, &'static str> {
    let cell = RefCell::new(input.clone());

    let num_elements: u32 = read_compressed_int(&mut input);
    let mut elements: Vec<ConfigArrayElement> = Vec::with_capacity(num_elements as usize);

    for _i in 0..num_elements {
        let element_type: u8 = (&mut *cell.borrow_mut()).bytes().next().unwrap().unwrap();

        if element_type == 0 {
            elements.push(ConfigArrayElement::StringElement(read_cstring(&mut *cell.borrow_mut())));
        } else if element_type == 1 {
            elements.push(ConfigArrayElement::FloatElement(read_f32(&mut *cell.borrow_mut())));
        } else if element_type == 2 {
            elements.push(ConfigArrayElement::IntElement(read_i32(&mut *cell.borrow_mut())));
        } else if element_type == 3 {
            elements.push(ConfigArrayElement::ArrayElement(read_rapified_array(&mut input.clone())?));
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

fn read_rapified_class<I: Read + Seek + Clone>(input: &mut I, level: u32) -> Result<ConfigClass, &'static str> {
    let cell = RefCell::new(input.clone());
    // ^ this is super ugly, @todo

    let mut fp = 0;
    if level == 0 {
        (&mut *cell.borrow_mut()).seek(SeekFrom::Start(16));
    } else {
        let classbody_fp: u32 = read_u32(&mut *cell.borrow_mut());

        //println!("{}{}", String::from("    ").repeat(level as usize), classbody_fp);

        fp = (&mut *cell.borrow_mut()).seek(SeekFrom::Current(0)).unwrap();
        (&mut *cell.borrow_mut()).seek(SeekFrom::Start(classbody_fp.into()));
    }

    let parent = read_cstring(&mut *cell.borrow_mut());
    let num_entries: u32 = read_compressed_int(&mut *cell.borrow_mut());
    //println!("{}", num_entries);
    let mut entries: Vec<(String, ConfigEntry)> = Vec::with_capacity(num_entries as usize);

    for _i in 0..num_entries {
        let entry_type: u8 = (&mut *cell.borrow_mut()).bytes().next().unwrap().unwrap();

        if entry_type == 0 {
            let name = read_cstring(&mut *cell.borrow_mut());
            //println!("{}{}", String::from("    ").repeat(level as usize), name);

            let class_entry = read_rapified_class(&mut input.clone(), level + 1)
                .expect("Failed to read rapified class");
            entries.push((name, ConfigEntry::ClassEntry(class_entry)));
        } else if entry_type == 1 {
            let subtype: u8 = (&mut *cell.borrow_mut()).bytes().next().unwrap().unwrap();
            let name = read_cstring(&mut *cell.borrow_mut());

            if subtype == 0 {
                entries.push((name, ConfigEntry::StringEntry(read_cstring(&mut *cell.borrow_mut()))));
            } else if subtype == 1 {
                entries.push((name, ConfigEntry::FloatEntry(read_f32(&mut *cell.borrow_mut()))));
            } else if subtype == 2 {
                entries.push((name, ConfigEntry::IntEntry(read_i32(&mut *cell.borrow_mut()))));
            } else {
                //return Err(&format!("Unrecognized variable entry subtype: {}", subtype) as &'static str);
                return Err("Unrecognized variable entry subtype: {}");
            }
        } else if entry_type == 2 || entry_type == 5 {
            if entry_type == 5 {
                (&mut *cell.borrow_mut()).seek(SeekFrom::Current(4)).unwrap();
            }

            let name = read_cstring(&mut *cell.borrow_mut());
            let mut array = read_rapified_array(&mut input.clone()).expect("Failed to read rapified array");
            array.is_expansion = entry_type == 5;

            entries.push((name.clone(), ConfigEntry::ArrayEntry(array)));
        } else if entry_type == 3 || entry_type == 4 {
            let name = read_cstring(&mut *cell.borrow_mut());
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
        (*cell.borrow_mut()).seek(SeekFrom::Start(fp));
    }

    Ok(ConfigClass {
        parent: parent,
        is_external: false,
        is_deletion: false,
        entries: Some(entries)
    })
}

pub fn read_rapified_config<I: Read + Seek + Clone>(mut input: I) -> Result<Config, &'static str> {
    let mut buffer = [0; 4];
    input.read_exact(&mut buffer);

    if &buffer != b"\0raP" {
        return Err("File doesn't seem to be a rapified config.");
    }

    Ok(Config {
        root_body: read_rapified_class(&mut input, 0)?
    })
}

pub fn write_rapified_config<O: Write>(_config: Config, _output: O) {
}
