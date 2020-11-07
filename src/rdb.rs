// Sorry but this needed three mores extra lines
// To be 69 lines

use std::path::PathBuf;

use binread::{io::SeekFrom, BinRead, NullString};

use binwrite::BinWrite;

use modular_bitfield::prelude::*;

#[derive(BinRead, BinWrite, Debug)]
pub struct RdbHeader {
    pub magic: u32,
    pub version: u32,
    #[br(assert("version != 0x30303030"))]
    pub header_size: u32,
    pub system_id: u32,
    pub file_count: u32,
    pub ktid: u32,
    #[br(map = NullString::into_string)]
    pub path: String,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct RdbEntry {
    pub magic: u32,
    pub version: u32,
    #[br(assert("version != 0x30303030"))]
    pub entry_size: u32,
    pub unk: u32,
    pub string_size: u32,
    pub unk2: u32,
    pub file_size: u64,
    pub entry_type: u32,
    pub file_ktid: u32,
    pub type_info_ktid: u32,
    pub flags: RdbFlags,
    #[br(count = (entry_size - string_size) - 0x30)]
    pub unk_content: Vec<u8>,
    #[br(count = string_size, align_after = 4)]
    #[binwrite(align_after(4))]
    pub name: Vec<u8>,
}

impl RdbEntry {
    pub fn get_external_path(&self) -> PathBuf {
        PathBuf::from(&format!("0x{:08x}.file", self.file_ktid))
    }

    pub fn make_external(&mut self) {
        self.flags.set_external(true);
        self.flags.set_internal(false);
    }

    pub fn make_uncompressed(&mut self) {
        self.flags.set_zlib_compressed(false);
        self.flags.set_lz4_compressed(false);
    }

    pub fn get_name(&mut self) -> &str {
        std::str::from_utf8(self.name.as_slice()).unwrap()
    }

    pub fn get_name_mut(&mut self) -> &mut str {
        std::str::from_utf8_mut(self.name.as_mut_slice()).unwrap()
    }

    pub fn set_external_file(&mut self, metadata: &std::fs::Metadata) {
        let mut name = self.get_name_mut().to_string();
        if let Some(size_marker) = name.find("@") {
            name.replace_range(size_marker.., &format!("@{:x}", metadata.len()));
        }

        self.file_size = metadata.len();

        if self.file_size == 0 {
            println!("Filesize is 0. Are you sure about that?");
        }

        // Remove the size of the original string
        self.entry_size -= self.string_size;
        // Put the edited name back into the entry
        self.name = name.as_bytes().to_vec();
        // Fix the size of the string
        self.string_size = name.len() as _;
        // Edit the size of the entry to take the new name into account
        self.entry_size += self.string_size;
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[binwrite(little)]
pub struct Rdb {
    pub header: RdbHeader,
    #[br(seek_before = SeekFrom::Start(header.header_size as _), count = header.file_count)]
    #[binwrite(align(4))]
    pub entries: Vec<RdbEntry>,
}

#[bitfield]
#[derive(BinRead, BinWrite, Debug)]
#[br(map = Self::from_bytes)]
pub struct RdbFlags {
    pub unk: B16,
    pub external: bool,
    pub internal: bool,
    pub unk2: B2,
    pub zlib_compressed: bool,
    pub lz4_compressed: bool,
    pub unk3: B10,
}
