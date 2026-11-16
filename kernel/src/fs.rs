//! RAM file system.
//!
//! A file system organises data into named files. BootScope keeps its files in
//! the heap (RAM), so they work while the system runs but are lost on reboot.
//! A disk file system would also need a disk driver and an on-disk format
//! (like FAT32), which is outside this project's scope.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

const MAX_FILES: usize = 32;
const MAX_NAME_LEN: usize = 24;
const MAX_FILE_SIZE: usize = 4096;

pub struct File {
    pub name: String,
    pub data: String,
}

pub enum FsError {
    NotFound,
    AlreadyExists,
    InvalidName,
    TooManyFiles,
    FileTooLarge,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            FsError::NotFound => "file not found",
            FsError::AlreadyExists => "file already exists",
            FsError::InvalidName => "invalid name (use letters, digits, '.', '_', '-'; max 24)",
            FsError::TooManyFiles => "file system full (32 files)",
            FsError::FileTooLarge => "file too large (max 4096 bytes)",
        };
        f.write_str(message)
    }
}

pub struct RamFs {
    files: Vec<File>,
}

impl RamFs {
    /// Create the file system with one welcome file.
    pub fn new() -> Self {
        let mut fs = RamFs { files: Vec::new() };
        let _ = fs.write(
            "readme.txt",
            "Welcome to BootScope! Files here live in RAM and vanish on reboot.",
            false,
        );
        fs
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    pub fn total_bytes(&self) -> usize {
        self.files.iter().map(|f| f.data.len()).sum()
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.files.iter().position(|f| f.name == name)
    }

    fn check_name(name: &str) -> Result<(), FsError> {
        let valid = !name.is_empty()
            && name.len() <= MAX_NAME_LEN
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
        if valid { Ok(()) } else { Err(FsError::InvalidName) }
    }

    /// Create an empty file.
    pub fn create(&mut self, name: &str) -> Result<(), FsError> {
        Self::check_name(name)?;
        if self.find(name).is_some() {
            return Err(FsError::AlreadyExists);
        }
        if self.files.len() >= MAX_FILES {
            return Err(FsError::TooManyFiles);
        }
        self.files.push(File { name: String::from(name), data: String::new() });
        Ok(())
    }

    /// Write text to a file (creating it if needed). `append` adds to the end.
    pub fn write(&mut self, name: &str, text: &str, append: bool) -> Result<(), FsError> {
        if self.find(name).is_none() {
            self.create(name)?;
        }
        let index = self.find(name).ok_or(FsError::NotFound)?;
        let file = &mut self.files[index];
        let new_len = if append { file.data.len() + text.len() + 1 } else { text.len() };
        if new_len > MAX_FILE_SIZE {
            return Err(FsError::FileTooLarge);
        }
        if append {
            if !file.data.is_empty() {
                file.data.push('\n');
            }
            file.data.push_str(text);
        } else {
            file.data = String::from(text);
        }
        Ok(())
    }

    pub fn read(&self, name: &str) -> Result<&str, FsError> {
        let index = self.find(name).ok_or(FsError::NotFound)?;
        Ok(&self.files[index].data)
    }

    pub fn remove(&mut self, name: &str) -> Result<(), FsError> {
        let index = self.find(name).ok_or(FsError::NotFound)?;
        self.files.remove(index); // its memory goes back to the heap
        Ok(())
    }
}
