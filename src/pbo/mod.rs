use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom, Cursor};
use std::path::PathBuf;

use hashbrown::HashMap;
use linked_hash_map::LinkedHashMap;
use crypto::{digest::Digest, sha1::Sha1};
use regex::Regex;

use crate::{ArmakeError, Config, binarize};
use crate::io::{WriteExt, ReadExt};

mod fs;

mod header;
pub use header::{PBOHeader, PackingMethod};

#[derive(Clone)]
pub struct PBO {
    pub files: LinkedHashMap<String, Cursor<Box<[u8]>>>,
    pub header_extensions: HashMap<String, String>,
    pub headers: Vec<PBOHeader>,
    /// only defined when reading existing PBOs, for created PBOs this is calculated during writing
    /// and included in the output
    pub checksum: Option<Vec<u8>>,
}

impl PBO {
    /// Reads an existing PBO from input.
    pub fn read<I: Read>(input: &mut I) -> Result<PBO, ArmakeError> {
        let mut headers: Vec<PBOHeader> = Vec::new();
        let mut first = true;
        let mut header_extensions: HashMap<String, String> = HashMap::new();

        loop {
            let header = PBOHeader::read(input)?;
            // todo: garbage filter

            if header.method() == PackingMethod::ProductEntry {
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
    pub fn from_directory(directory: PathBuf, mut binarize: bool, exclude_patterns: &[&str], includefolders: &[PathBuf]) -> Result<PBO, ArmakeError> {
        let file_list = fs::list_files(&directory)?;
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

            if !fs::file_allowed(&name, &exclude_patterns) { continue; }

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
                let config = Config::read(&mut file, Some(path.clone()), includefolders)?;
                let cursor = config.to_cursor()?;

                files.insert(name, cursor);
            } else if cfg!(windows) && binarize && is_binarizable {
                let cursor = binarize::binarize(&path)?;

                files.insert(name, cursor);
            } else {
                // if is_binarizable && !cfg!(windows) {
                //     warning!("On non-Windows systems binarize.exe cannot be used; file will be copied as-is.", Some("non-windows-binarization"), (Some(&relative.to_str().unwrap()), None));
                // }

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
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), ArmakeError> {
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

        let mut h = Sha1::new();

        output.write_all(headers.get_ref())?;
        h.input(headers.get_ref());

        for (_, cursor) in &files_sorted {
            output.write_all(cursor.get_ref())?;
            h.input(cursor.get_ref());
        }

        output.write_all(&[0])?;
        let mut hash = Vec::new();
        h.result(&mut hash);
        output.write_all(&hash)?;

        Ok(())
    }

    /// Returns the PBO as a `Cursor`.
    pub fn to_cursor(&self) -> Result<Cursor<Vec<u8>>, ArmakeError> {
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        self.write(&mut cursor)?;

        cursor.seek(SeekFrom::Start(0))?;

        Ok(cursor)
    }
}
