use std::collections::{HashMap};
use std::ffi::{OsStr};
use std::fs::{File, create_dir_all, read_dir};
use std::io::{Read, Write, Seek, SeekFrom, Error, Cursor};
use std::path::{PathBuf};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use linked_hash_map::{LinkedHashMap};
use openssl::hash::{Hasher, MessageDigest};
use regex::{Regex};

use crate::error::*;
use crate::io::*;
use crate::config::*;
use crate::preprocess::*;
use crate::binarize;

struct PBOHeader {
    filename: String,
    packing_method: u32,
    original_size: u32,
    reserved: u32,
    timestamp: u32,
    data_size: u32,
}

/// PBO file
///
/// # Examples
///
/// ```
/// # use std::path::PathBuf;
/// # use armake2::pbo::PBO;
/// let pbo = PBO::from_directory(PathBuf::from("src"), false, &Vec::new(), &Vec::new()).expect("Failed to create PBO");
///
/// assert!(pbo.files.iter().any(|(name, _data)| name == "main.rs"));
///
/// let mut cursor = pbo.to_cursor().unwrap();
/// let reread = PBO::read(&mut cursor).unwrap();
///
/// assert!(reread.checksum.is_some());
/// ```
pub struct PBO {
    pub files: LinkedHashMap<String, Cursor<Box<[u8]>>>,
    pub header_extensions: HashMap<String, String>,
    headers: Vec<PBOHeader>,
    /// only defined when reading existing PBOs, for created PBOs this is calculated during writing
    /// and included in the output
    pub checksum: Option<Vec<u8>>,
}

impl PBOHeader {
    fn read<I: Read>(input: &mut I) -> Result<PBOHeader, Error> {
        Ok(PBOHeader {
            filename: input.read_cstring()?,
            packing_method: input.read_u32::<LittleEndian>()?,
            original_size: input.read_u32::<LittleEndian>()?,
            reserved: input.read_u32::<LittleEndian>()?,
            timestamp: input.read_u32::<LittleEndian>()?,
            data_size: input.read_u32::<LittleEndian>()?,
        })
    }

    fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_cstring(&self.filename)?;
        output.write_u32::<LittleEndian>(self.packing_method)?;
        output.write_u32::<LittleEndian>(self.original_size)?;
        output.write_u32::<LittleEndian>(self.reserved)?;
        output.write_u32::<LittleEndian>(self.timestamp)?;
        output.write_u32::<LittleEndian>(self.data_size)?;
        Ok(())
    }
}

fn matches_glob(s: &str, pattern: &str) -> bool {
    if let Some(index) = pattern.find('*') {
        if s[..index] != pattern[..index] { return false; }

        for i in (index+1)..(s.len()-1) {
            if matches_glob(&s[i..].to_string(), &pattern[(index+1)..].to_string()) { return true; }
        }

        false
    } else {
        s == pattern
    }
}

fn file_allowed(name: &str, exclude_patterns: &[String]) -> bool {
    for pattern in exclude_patterns {
        if matches_glob(&name, &pattern) { return false; }
    }

    true
}

impl PBO {
    /// Reads an existing PBO from input.
    pub fn read<I: Read>(input: &mut I) -> Result<PBO, Error> {
        let mut headers: Vec<PBOHeader> = Vec::new();
        let mut first = true;
        let mut header_extensions: HashMap<String, String> = HashMap::new();

        loop {
            let header = PBOHeader::read(input)?;
            // todo: garbage filter

            if header.packing_method == 0x5665_7273 {
                if !first { unreachable!(); }

                loop {
                    let s = input.read_cstring()?;
                    if s.is_empty() { break; }

                    header_extensions.insert(s, input.read_cstring()?);
                }
            } else if header.filename == "" {
                break;
            } else {
                headers.push(header);
            }

            first = false;
        }

        let mut files: LinkedHashMap<String, Cursor<Box<[u8]>>> = LinkedHashMap::new();
        for header in &headers {
            let mut buffer: Box<[u8]> = vec![0; header.data_size as usize].into_boxed_slice();
            input.read_exact(&mut buffer)?;
            files.insert(header.filename.clone(), Cursor::new(buffer));
        }

        input.bytes().next();
        let mut checksum = vec![0; 20];
        input.read_exact(&mut checksum)?;

        Ok(PBO {
            files,
            header_extensions,
            headers,
            checksum: Some(checksum),
        })
    }

    /// Constructs a PBO from a directory with optional binarization.
    ///
    /// `exclude_patterns` contains glob patterns to exclude from the PBO, `includefolders` contain
    /// paths to search for absolute includes and should generally include the current working
    /// directory.
    pub fn from_directory(directory: PathBuf, mut binarize: bool, exclude_patterns: &[String], includefolders: &[PathBuf]) -> Result<PBO, Error> {
        let file_list = list_files(&directory)?;
        let mut files: LinkedHashMap<String, Cursor<Box<[u8]>>> = LinkedHashMap::new();
        let mut header_extensions: HashMap<String,String> = HashMap::new();

        if directory.join("$NOBIN$").exists() || directory.join("$NOBIN-NOTEST$").exists() {
            binarize = false;
        }

        for path in file_list {
            let mut relative = path.strip_prefix(&directory).unwrap().to_path_buf();
            if binarize && relative.file_name() == Some(OsStr::new("config.cpp")) {
                relative = relative.with_file_name("config.bin");
            }

            let mut name: String = relative.to_str().unwrap().replace("/", "\\");
            let is_binarizable = Regex::new(".(rtm|p3d)$").unwrap().is_match(&name);

            if !file_allowed(&name, &exclude_patterns) { continue; }

            let mut file = File::open(&path)?;

            if name == "$PBOPREFIX$" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                for l in content.lines() {
                    if l.is_empty() { break; }

                    let eq: Vec<String> = l.split('=').map(|s| s.to_string()).collect();
                    if eq.len() == 1 {
                        header_extensions.insert("prefix".to_string(), l.to_string());
                    } else {
                        header_extensions.insert(eq[0].clone(), eq[1].clone());
                    }
                }
            } else if binarize && vec!["cpp", "rvmat"].contains(&path.extension().unwrap_or_else(|| OsStr::new("")).to_str().unwrap()) {
                let config = Config::read(&mut file, Some(path.clone()), includefolders).prepend_error("Failed to parse config:")?;
                let cursor = config.to_cursor()?;

                files.insert(name, cursor);
            } else if cfg!(windows) && binarize && is_binarizable {
                let cursor = binarize::binarize(&path).prepend_error(format!("Failed to binarize {:?}:", relative).to_string())?;

                files.insert(name, cursor);
            } else {
                if is_binarizable && !cfg!(windows) {
                    warning("On non-Windows systems binarize.exe cannot be used; file will be copied as-is.", Some("non-windows-binarization"), (Some(&relative.to_str().unwrap()), None));
                }

                let mut buffer: Vec<u8> = Vec::new();
                file.read_to_end(&mut buffer)?;

                name = Regex::new(".p3do$").unwrap().replace_all(&name, ".p3d").to_string();

                files.insert(name, Cursor::new(buffer.into_boxed_slice()));
            }
        }

        if header_extensions.get("prefix").is_none() {
            let prefix: String = directory.file_name().unwrap().to_str().unwrap().to_string();
            header_extensions.insert("prefix".to_string(), prefix);
        }

        Ok(PBO {
            files,
            header_extensions,
            headers: Vec::new(),
            checksum: None,
        })
    }

    /// Writes PBO to output.
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        let mut headers: Cursor<Vec<u8>> = Cursor::new(Vec::new());

        let ext_header = PBOHeader {
            filename: "".to_string(),
            packing_method: 0x5665_7273,
            original_size: 0,
            reserved: 0,
            timestamp: 0,
            data_size: 0,
        };
        ext_header.write(&mut headers)?;

        if let Some(prefix) = self.header_extensions.get("prefix") {
            headers.write_all(b"prefix\0")?;
            headers.write_cstring(prefix)?;
        }

        for (key, value) in self.header_extensions.iter() {
            if key == "prefix" { continue; }

            headers.write_cstring(key)?;
            headers.write_cstring(value)?;
        }
        headers.write_cstring("".to_string())?;

        let mut files_sorted: Vec<(String,&Cursor<Box<[u8]>>)> = self.files.iter().map(|(a,b)| (a.clone(),b)).collect();
        files_sorted.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        for (name, cursor) in &files_sorted {
            let header = PBOHeader {
                filename: name.clone(),
                packing_method: 0,
                original_size: cursor.get_ref().len() as u32,
                reserved: 0,
                timestamp: 0,
                data_size: cursor.get_ref().len() as u32,
            };

            header.write(&mut headers)?;
        }

        let header = PBOHeader {
            packing_method: 0,
            ..ext_header
        };
        header.write(&mut headers)?;

        let mut h = Hasher::new(MessageDigest::sha1()).unwrap();

        output.write_all(headers.get_ref())?;
        h.update(headers.get_ref()).unwrap();

        for (_, cursor) in &files_sorted {
            output.write_all(cursor.get_ref())?;
            h.update(cursor.get_ref()).unwrap();
        }

        output.write_all(&[0])?;
        output.write_all(&*h.finish().unwrap())?;

        Ok(())
    }

    /// Returns the PBO as a `Cursor`.
    pub fn to_cursor(&self) -> Result<Cursor<Vec<u8>>, Error> {
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        self.write(&mut cursor)?;

        cursor.seek(SeekFrom::Start(0))?;

        Ok(cursor)
    }
}

fn list_files(directory: &PathBuf) -> Result<Vec<PathBuf>, Error> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            for f in list_files(&path)? {
                files.push(f);
            }
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

pub fn cmd_inspect<I: Read>(input: &mut I) -> Result<(), Error> {
    let pbo = PBO::read(input).prepend_error("Failed to read PBO:")?;

    if !pbo.header_extensions.is_empty() {
        println!("Header extensions:");
        for (key, value) in pbo.header_extensions.iter() {
            println!("- {}={}", key, value);
        }
        println!();
    }

    println!("# Files: {}\n", pbo.files.len());

    println!("Path                                                  Method  Original    Packed");
    println!("                                                                  Size      Size");
    println!("================================================================================");
    for header in pbo.headers {
        println!("{:50} {:9} {:9} {:9}", header.filename, header.packing_method, header.original_size, header.data_size);
    }

    Ok(())
}

pub fn cmd_cat<I: Read, O: Write>(input: &mut I, output: &mut O, name: &str) -> Result<(), Error> {
    let pbo = PBO::read(input).prepend_error("Failed to read PBO:")?;

    match pbo.files.get(name) {
        Some(cursor) => {
            output.write_all(cursor.get_ref()).prepend_error("Failed to write output:")?;
        },
        None => {
            eprintln!("not found"); // @todo
        }
    }

    Ok(())
}

pub fn cmd_unpack<I: Read>(input: &mut I, output: PathBuf) -> Result<(), Error> {
    let pbo = PBO::read(input).prepend_error("Failed to read PBO:")?;

    create_dir_all(&output).prepend_error("Failed to create output folder:")?;

    if !pbo.header_extensions.is_empty() {
        let prefix_path = output.join(PathBuf::from("$PBOPREFIX$"));
        let mut prefix_file = File::create(prefix_path).prepend_error("Failed to create prefix file:")?;

        for (key, value) in pbo.header_extensions.iter() {
            prefix_file.write_all(format!("{}={}\n", key, value).as_bytes()).prepend_error("Failed to write prefix file:")?;
        }
    }

    for (file_name, cursor) in pbo.files.iter() {
        // @todo: windows
        let path = output.join(PathBuf::from(file_name.replace("\\", pathsep())));
        create_dir_all(path.parent().unwrap()).prepend_error("Failed to create output folder:")?;
        let mut file = File::create(path).prepend_error("Failed to open output file:")?;
        file.write_all(cursor.get_ref()).prepend_error("Failed to write output file:")?;
    }

    Ok(())
}

pub fn cmd_pack<O: Write>(input: PathBuf, output: &mut O, headerext: &[String], excludes: &[String]) -> Result<(), Error> {
    let mut pbo = PBO::from_directory(input, false, excludes, &Vec::new())?;

    for h in headerext {
        let (key, value) = (h.split('=').nth(0).unwrap(), h.split('=').nth(1).unwrap());
        pbo.header_extensions.insert(key.to_string(), value.to_string());
    }

    pbo.write(output).prepend_error("Failed to write PBO:")?;

    Ok(())
}

pub fn cmd_build<O: Write>(input: PathBuf, output: &mut O, headerext: &[String], excludes: &[String], includefolders: &[PathBuf]) -> Result<(), Error> {
    let mut pbo = PBO::from_directory(input, true, excludes, includefolders)?;

    for h in headerext {
        let (key, value) = (h.split('=').nth(0).unwrap(), h.split('=').nth(1).unwrap());
        pbo.header_extensions.insert(key.to_string(), value.to_string());
    }

    pbo.write(output).prepend_error("Failed to write PBO:")?;

    Ok(())
}
