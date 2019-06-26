//! Functions for calling BI's binarize.exe (on Windows)

use std::env::{var, temp_dir};
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::{Read, Write, Cursor, Error};
use std::path::{PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::*;

use crate::*;
use crate::error::*;

#[cfg(windows)]
fn find_binarize_exe() -> Result<PathBuf, Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let binarize = hkcu.open_subkey("Software\\Bohemia Interactive\\binarize")?;
    let value: String = binarize.get_value("path")?;

    Ok(PathBuf::from(value).join("binarize_x64.exe"))
}

#[cfg(unix)]
fn find_binarize_exe() -> Result<PathBuf, Error> {
    unreachable!();
}

fn create_temp_directory(name: &str) -> Result<PathBuf, Error> {
    let dir = temp_dir();
    let mut i = 0;

    let mut path;
    loop {
        path = dir.join(format!("armake_{}_{}", name, i));
        if !path.exists() { break; }

        i += 1;
    }

    create_dir_all(&path)?;

    Ok(path)
}

/// Binarizes the given path with BI's binarize.exe (Only available on Windows).
pub fn binarize(input: &PathBuf) -> Result<Cursor<Box<[u8]>>, Error> {
    if !cfg!(windows) {
        return Err(error!("binarize.exe is only available on windows. Use rapify to binarize configs."));
    }

    let binarize_exe = find_binarize_exe().prepend_error("Failed to find BI's binarize.exe:")?;
    if !binarize_exe.exists() {
        return Err(error!("BI's binarize.exe found in registry, but doesn't exist."));
    }

    let input_dir = PathBuf::from(input.parent().unwrap());
    let name = input.file_name().unwrap().to_str().unwrap().to_string();
    let tempdir = create_temp_directory(&name).prepend_error("Failed to create tempfolder:")?;

    let piped = var("BIOUTPUT").unwrap_or_else(|_| "0".to_string()) == "1";

    let binarize_output = Command::new(binarize_exe)
        .args(&["-norecurse", "-always", "-silent", "-maxProcesses=0", input_dir.to_str().unwrap(), tempdir.to_str().unwrap(), input.file_name().unwrap().to_str().unwrap()])
        .stdout(if piped { Stdio::inherit() } else { Stdio::null() })
        .stderr(if piped { Stdio::inherit() } else { Stdio::null() })
        .output().unwrap();

    if !binarize_output.status.success() {
        let msg = match binarize_output.status.code() {
            Some(code) => format!("binarize.exe terminated with exit code: {}", code),
            None => "binarize.exe terminated by signal.".to_string()
        };
        let outputhint = if !piped { "\nUse BIOUTPUT=1 to see binarize.exe's output." } else { "" };

        return Err(error!("{}{}", msg, outputhint));
    }

    let result_path = tempdir.join(input.strip_prefix(&input_dir).unwrap());
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut file = File::open(result_path).prepend_error("Failed to open binarize.exe output:")?;
        file.read_to_end(&mut buffer).prepend_error("Failed to read binarize.exe output:")?;
    }

    remove_dir_all(&tempdir).prepend_error("Failed to remove temp directory:")?;

    Ok(Cursor::new(buffer.into_boxed_slice()))
}

/// Binarizes the given path using BI's binarize.exe (on Windows) and writes it to the output.
pub fn cmd_binarize(input: PathBuf, output: PathBuf) -> Result<(), Error> {
    if !cfg!(windows) {
        return Err(error!("binarize.exe is only available on windows. Use rapify to binarize configs."));
    }

    let cursor = binarize(&input)?;
    let mut file = File::create(output).prepend_error("Failed to open output:")?;
    file.write_all(cursor.get_ref()).prepend_error("Failed to write result to file:")?;

    Ok(())
}
