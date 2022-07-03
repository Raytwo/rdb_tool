use std::{fs::OpenOptions, io::{Seek, SeekFrom, BufReader, BufWriter, Read}, path::{PathBuf, Path}};

use binread::{BinRead, NullString, BinResult, BinReaderExt};

use binwrite::BinWrite;

use modular_bitfield::prelude::*;

#[derive(BinRead, BinWrite, Debug)]
pub struct RdbHeader {
    pub magic: u32,
    pub version: u32,
    #[br(assert(version == 0x30303030))]
    pub header_size: u32,
    pub system_id: u32,
    pub file_count: u32,
    pub ktid: u32,
    #[br(map = NullString::into_string)]
    pub path: String,
}

#[derive(BinRead, BinWrite, Debug, Clone)]
pub struct RdbEntry {
    pub magic: u32,
    pub version: u32,
    #[br(assert(version == 0x30303030))]
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

#[derive(BinRead, BinWrite, Debug, Clone)]
pub struct IdrkEntry {
    pub magic: u32,
    pub version: u32,
    #[br(assert(version == 0x30303030))]
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

    pub fn set_external_file(&mut self, path: &std::path::Path) {
        //let mut name = self.get_name_mut().to_string();

        self.file_size = path.metadata().unwrap().len();

        // if let Some(size_marker) = name.find("@") {
        //     name.replace_range(size_marker.., &format!("@{:x}", self.file_size));
        // }

        if self.file_size == 0 {
            println!("Filesize is 0. Are you sure about that?");
        }

        // Remove the size of the original string
        self.entry_size -= self.string_size;
        // Put the edited name back into the entry
        self.name = Vec::new();
        // Fix the size of the string
        self.string_size = 0 as _;
        // Edit the size of the entry to take the new name into account
        self.entry_size += self.string_size;

        let mut ext_entry = self.clone();
        ext_entry.patch_external_file(path);
    }

    pub fn patch_external_file(&mut self, path: &std::path::Path) {
        self.name = vec![];
        //self.write(&mut bytes).unwrap();

        //let cursor : &mut std::io::Cursor<Vec<u8>> = &mut std::io::Cursor::new(Vec::new());

        let mut test = OpenOptions::new().read(true).write(true).create(true).open(path).unwrap();
        
        //let file = std::fs::read(path).unwrap();

        let mut buffer = Vec::new();        
        
        let mut out_sig = [0;4];
        test.read(&mut out_sig).unwrap();

        if &out_sig == b"IDRK" {
            println!("Already patched");
            return;
        }

        test.seek(SeekFrom::Start(0)).unwrap();

        let header_size = match self.entry_type {
            0 => 0x38,
            // 1 is KidsSingletonDb? 4 is G1E
            1 | 4 => 0x48,
            // G1A, G1T
            8 => 0x58,
            // G1M, most likely other model related formats
            12 => 0x68,
            _ => panic!("Unknown entry type found: {}", self.entry_type)
        };
        self.entry_size = header_size + test.metadata().unwrap().len() as u32;
        self.file_size = test.metadata().unwrap().len() as _;
        self.string_size = self.file_size as _;
        self.flags = RdbFlags::new();

        //test.seek(SeekFrom::Start(0)).unwrap();
        self.write(&mut buffer).unwrap();
        test.read_to_end(&mut buffer).unwrap();
        
        // let mut writer = BufWriter::new(test);
        // writer.seek(SeekFrom::Start(0)).unwrap();

        // self.write(&mut writer).unwrap();
        //writer.write_all(&mut reader.buffer());
        // Check if we're dealing with a KTID or an actual filename
        let filename = if path.file_name().unwrap().to_str().unwrap().starts_with("0x") {
            // Strip the extension (Cethleann keeps the extension even if the hash is missing)
            path.file_stem().unwrap().to_str().unwrap()
        } else {
            // Get the full filename with extension
            path.file_name().unwrap().to_str().unwrap()
        };

        let out_path = PathBuf::from(format!("./data/0x{}.file", crate::ktid(filename)));

        if !out_path.exists() {
            std::fs::create_dir_all("./data/").unwrap();
        }

        match std::fs::write(out_path, &buffer) {
            Ok(_) => {},
            Err(err) => panic!("{}", err),
        };
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

impl Rdb {
    pub fn open<P: AsRef<Path>>(path: P) -> BinResult<Self> {
        Self::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))
    }

    pub fn from_reader<R: std::io::Read + std::io::Seek + Send + 'static>(mut reader: R) -> BinResult<Self> {
        let rdb: Self = reader.read_le()?;

        Ok(rdb)
    }

    pub fn get_entry_by_ktid(&self, ktid: crate::ktid::KTID) -> Option<&RdbEntry> {
        self.entries.iter().find(|x| x.file_ktid == ktid.as_u32())
    }

    pub fn get_entry_by_ktid_mut(&mut self, ktid: crate::ktid::KTID) -> Option<&mut RdbEntry> {
        self.entries.iter_mut().find(|x| x.file_ktid == ktid.as_u32())
    }
}

#[bitfield]
#[derive(BinRead, BinWrite, Debug, Copy, Clone)]
#[br(map = Self::from_bytes)]
pub struct RdbFlags {
    pub unk: B16,
    pub external: bool,
    pub internal: bool,
    pub unk2: B2,
    // If both are set, file is encrypted
    pub zlib_compressed: bool,
    pub lz4_compressed: bool,
    pub unk3: B10,
}
