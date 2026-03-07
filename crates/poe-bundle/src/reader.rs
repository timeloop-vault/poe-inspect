use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;
use std::string::FromUtf8Error;

extern crate libc;
use std::{cmp, str};
use log::*;

use super::util;

use std::io::Error;
use ggpk::GGPK;

// bundle implementation discussion
// https://github.com/poe-tool-dev/ggpk.discussion/wiki/Bundle-scheme

#[link(name = "libooz", kind = "static")]
extern "C" {
    fn Ooz_Decompress(src_buf: *const u8, src_len: u32, dst: *mut u8, dst_size: usize) -> i32;
}

fn decompress(source: *const u8, src_len: usize, destination: *mut u8, dst_size: usize) -> i32 {
    // TODO: at some point look into implementing the decompression in rust
    unsafe {
        return Ooz_Decompress(source, src_len as u32, destination, dst_size);
    }
}

//#[derive(Debug)]
pub struct BundledFile {
    pub bundle_path: String,
    pub bundle_uncompressed_size: u32,
    pub offset: u32,
    pub size: u32,
}

pub struct Bundle {
    pub name: String,
    pub uncompressed_size: u32,
}

pub struct BundleIndex {
    files: HashMap<u64, BundledFile>,
    pub paths: Vec<String>,
}

pub struct BundleReader {
    install_path: String,
    pub index: BundleIndex,
    ggpk: Option<GGPK>,
}

pub trait BundleFileRead {
    fn get(&self, filepath: &str) -> Option<&BundledFile>;
}

pub trait BundleReaderRead {
    fn size_of(&self, file: &str) -> Option<usize>;
    fn write_into(&self, file: &str, dst: &mut impl Write) -> Result<usize, Error>;
    fn bytes(&self, file: &str) -> Result<Vec<u8>, Error>;
}

impl BundleReader {
    pub fn from_install(path: &Path) -> BundleReader {
        let index_bytes = BundleIndex::get_file(path, "Bundles2/_.index.bin");

        let ggpk = if path.join("Bundles2/_.index.bin").exists() {
            None
        } else if path.is_file() {
            Some(GGPK::from_file(path).unwrap())
        } else {
            Some(GGPK::from_path(path).unwrap())
        };

        BundleReader {
            ggpk,
            install_path: path.to_string_lossy().to_string(),
            index: BundleIndex::read_index(index_bytes.as_slice()),
        }
    }
}
impl BundleReaderRead for BundleReader {
    fn size_of(&self, file: &str) -> Option<usize> {
        self.index.get(file).map(|file| file.size as usize)
    }

    fn write_into(&self, file: &str, dst: &mut impl Write) -> Result<usize, Error> {
        self.index
            .get(file)
            .map(|file| {
                let bundle_path = format!("Bundles2/{}.bundle.bin", file.bundle_path);
                let fs_path = format!("{}/{}", self.install_path, bundle_path);
                if Path::new(fs_path.as_str()).exists() {
                    fs::read(fs_path.as_str()).and_then(|bytes| {
                        let size = unpack(&bytes, &mut Vec::with_capacity(0));
                        let mut unpacked = Vec::with_capacity(size);
                        unpack(&bytes, &mut unpacked);
                        dst.write(
                            &unpacked
                                [file.offset as usize..(file.offset as usize + file.size as usize)],
                        )
                    })
                } else {
                    let bundle = self.ggpk.as_ref().unwrap().get_file(bundle_path.as_str());
                    self.ggpk.as_ref().unwrap()
                        .mmap
                        .get(bundle.record.begin..(bundle.record.begin + bundle.record.bytes as usize))
                        .map(|bytes| {
                            let size = unpack(&bytes, &mut Vec::with_capacity(0));
                            let mut unpacked = Vec::with_capacity(size);
                            unpack(&bytes, &mut unpacked);
                            dst.write(
                                &unpacked[file.offset as usize
                                    ..(file.offset as usize + file.size as usize)],
                            )
                        })
                        .unwrap_or_else(|| {
                            Err(Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Read outside GGPK",
                            ))
                        })
                }
            })
            .unwrap()
    }

    fn bytes(&self, file: &str) -> Result<Vec<u8>, Error> {
        self.index
            .get(file)
            .map(|file| {
                let mut dst = Vec::with_capacity(file.size as usize);
                let bundle_path = format!("Bundles2/{}.bundle.bin", file.bundle_path);
                let fs_path = format!("{}/{}", self.install_path, bundle_path);
                if Path::new(fs_path.as_str()).exists() {
                    fs::read(fs_path.as_str()).and_then(|bytes| {
                        let size = unpack(&bytes, &mut Vec::with_capacity(0));
                        let mut unpacked = Vec::with_capacity(size);
                        unpack(&bytes, &mut unpacked);
                        dst.write(
                            &unpacked
                                [file.offset as usize..(file.offset as usize + file.size as usize)],
                        )
                    })?;
                } else {
                    let bundle = self.ggpk.as_ref().unwrap().get_file(bundle_path.as_str());
                    self.ggpk.as_ref().unwrap()
                        .mmap
                        .get(bundle.record.begin..(bundle.record.begin + bundle.record.bytes as usize))
                        .map(|bytes| {
                            let size = unpack(&bytes, &mut Vec::with_capacity(0));
                            let mut unpacked = Vec::with_capacity(size);
                            unpack(&bytes, &mut unpacked);
                            dst.write(
                                &unpacked[file.offset as usize
                                    ..(file.offset as usize + file.size as usize)],
                            )
                        })
                        .unwrap_or_else(|| {
                            Err(Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Read outside GGPK",
                            ))
                        })?;
                }
                Ok(dst)
            })
            .unwrap()
    }
}

impl BundleIndex {
    fn get_file(install_path: &Path, file_path: &str) -> Vec<u8> {
        let extracted_file = install_path.join(file_path);

        if extracted_file.exists() {
            fs::read(extracted_file).expect("Unable to read")
        } else if install_path.is_file() {
            let ggpk = GGPK::from_file(&install_path).expect("Unable to read GGPK");
            let file = ggpk.get_file(file_path);
            let mut dst = Vec::with_capacity(file.record.bytes as usize);
            file.write_into(&mut dst).unwrap();
            dst
        } else {
            let ggpk = GGPK::from_path(install_path).expect("Unable to read GGPK");
            let file = ggpk.get_file(file_path);
            let mut dst = Vec::with_capacity(file.record.bytes as usize);
            file.write_into(&mut dst).unwrap();
            dst
        }
    }

    fn read_index(data: &[u8]) -> BundleIndex {
        let size = unpack(&data, &mut Vec::with_capacity(0));
        let mut dst = Vec::with_capacity(size);
        unpack(&data, &mut dst);
        build_index(&dst)
    }
}

impl BundleFileRead for BundleIndex {
    fn get(&self, filepath: &str) -> Option<&BundledFile> {
        let hash = util::filepath_hash(filepath.to_string());
        self.files.get(&hash)
    }
}

fn unpack(src: &[u8], dst: &mut Vec<u8>) -> usize {
    let mut c = Cursor::new(src);

    let _ = c.read_u32::<LittleEndian>().unwrap(); // total size (uncompressed)
    let _ = c.read_u32::<LittleEndian>().unwrap(); // total size (compressed)
    let _ = c.read_u32::<LittleEndian>().unwrap(); // head size

    let _ = c.read_u32::<LittleEndian>().unwrap(); // encoding of first chunk
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let uncompressed_size = c.read_u64::<LittleEndian>().unwrap();
    let _ = c.read_u64::<LittleEndian>().unwrap(); // total size (compressed)
    let chunk_count = c.read_u32::<LittleEndian>().unwrap();
    let chunk_unpacked_size = c.read_u32::<LittleEndian>().unwrap() as u64;
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown

    if dst.capacity() < uncompressed_size as usize {
        return uncompressed_size as usize;
    }

    let chunk_sizes = (0..chunk_count)
        .map(|_| c.read_u32::<LittleEndian>().unwrap())
        .map(|size| usize::try_from(size).unwrap())
        .collect::<Vec<usize>>();

    let mut chunk_offset = usize::try_from(c.position()).unwrap();
    let mut bytes_to_read = uncompressed_size;

    (0..chunk_count as usize).for_each(|index| {
        let src = &src[chunk_offset..chunk_offset + chunk_sizes[index]];
        let dst_size = cmp::min(bytes_to_read, chunk_unpacked_size) as usize;

        trace!("Decompressing chunk: {}/{}", index, chunk_count);
        let mut chunk_dst = vec![0u8; dst_size];
        let wrote = decompress(
            src.as_ptr(),
            chunk_sizes[index],
            chunk_dst.as_mut_ptr(),
            dst_size,
        );
        if wrote < 0 {
            warn!("Decompression failed with code: {}", wrote);
            warn!("Chunk header: [{},{}]", src[0], src[1]);
        }
        dst.write(&chunk_dst[0..dst_size]).unwrap();

        if bytes_to_read > chunk_unpacked_size {
            bytes_to_read -= chunk_unpacked_size;
        }
        chunk_offset = chunk_offset + chunk_sizes[index];
    });
    return 0;
}

fn build_index(data: &[u8]) -> BundleIndex {
    debug!("Building bundle index");
    let mut c = Cursor::new(data);
    let bundle_count = c.read_u32::<LittleEndian>().unwrap();

    let bundles: HashMap<_, Bundle> = (0..bundle_count)
        .map(|index| {
            let name_length = c.read_u32::<LittleEndian>().unwrap();
            let name = (0..name_length)
                .map(|_| c.read_u8().unwrap())
                .collect::<Vec<u8>>();
            let uncompressed_size = c.read_u32::<LittleEndian>().unwrap();
            (
                // TODO: clean up
                index,
                Bundle {
                    name: str::from_utf8(name.as_slice()).unwrap().to_string(),
                    uncompressed_size,
                },
            )
        })
        .collect();

    let file_count = c.read_u32::<LittleEndian>().unwrap();
    let files: HashMap<_, BundledFile> = (0..file_count)
        .map(|_| {
            let hash = c.read_u64::<LittleEndian>().unwrap();
            let bundle_index = c.read_u32::<LittleEndian>().unwrap();
            let bundle = bundles.get(&bundle_index).unwrap();
            (
                hash,
                BundledFile {
                    bundle_path: bundle.name.clone(),
                    bundle_uncompressed_size: bundle.uncompressed_size,
                    offset: c.read_u32::<LittleEndian>().unwrap(),
                    size: c.read_u32::<LittleEndian>().unwrap(),
                },
            )
        })
        .collect();

    // skip path data
    let path_rep_count = c.read_u32::<LittleEndian>().unwrap();
    c.set_position(c.position() + 20 * path_rep_count as u64);

    let remaining_bytes = &data[c.position() as usize..];
    let size = unpack(&remaining_bytes, &mut Vec::with_capacity(0));
    let mut dst = Vec::with_capacity(size);
    unpack(&remaining_bytes, &mut dst);

    BundleIndex {
        files,
        paths: build_paths(dst.as_slice()),
    }
}

fn build_paths(bytes: &[u8]) -> Vec<String> {
    debug!("Generating bundle filepaths");
    let mut c = Cursor::new(bytes);

    let mut generation_phase = false;
    let mut table = vec![];
    let mut files = vec![];

    while c.position() + 4 <= bytes.len() as u64 {
        let index = c.read_u32::<LittleEndian>().unwrap() as usize;

        if index == 0 {
            generation_phase = !generation_phase;
            if generation_phase {
                table.clear();
            }
        }

        if index > 0 {
            let mut text = read_utf8(&mut c).unwrap();
            if index <= table.len() {
                text = format!("{}{}", table[index - 1], text);
            }

            if generation_phase {
                table.push(text)
            } else {
                files.push(text);
            }
        }
    }
    debug!("Generated {} filepaths", files.len());
    files
}

fn read_utf8(c: &mut Cursor<&[u8]>) -> Result<String, FromUtf8Error> {
    let raw_bytes = (0..)
        .map(|_| c.read_u8().unwrap())
        .take_while(|&x| x != 0u8)
        .collect::<Vec<u8>>();
    return String::from_utf8(raw_bytes);
}
