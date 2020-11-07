use std::{fs::File, fs::ReadDir, path::Path, path::PathBuf};

use binread::{io::Cursor, BinRead};

use binwrite::BinWrite;

mod rdb;
use rdb::Rdb;
use rdb::RdbEntry;
use rdb::RdbFlags;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "RdbTool",
    about = "Simple command-line tool to manipulate RDB files. You are expected to patch your external file's header yourself for now."
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "Path to the RDB file relative to RdbTool's executable")]
    pub path: PathBuf,
    #[structopt(parse(from_os_str), help = "Output path to the RDB file relative to RdbTool's executable")]
    pub out_path: PathBuf,
}

fn patch_rdb(path: &Path, out_path: &Path) -> Result<(), String> {
    let mut rdb: Rdb = Rdb::read(&mut Cursor::new(&std::fs::read(path).unwrap())).unwrap();

    let external_path = PathBuf::from(format!("./{}", rdb.header.path));

    if !external_path.exists() {
        return Err(format!("Couldn't find a directory matching the internal path ({}). Consider making it.", rdb.header.path));
    }

    let files = match std::fs::read_dir(external_path) {
        Ok(files) => files,
        Err(_) => return Err("How did you even managed to delete the internal path directory this fast? Stop that.".to_string()),
    };

    for entry in files {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        // We don't care about subdirectories
        if metadata.is_dir() {
            continue;
        }

        let filename = &entry.file_name().to_str().unwrap().to_owned();

        if !filename.ends_with(".file") {
            println!("File {} does not have the '.file' extension. Skipping.", filename);
            continue;
        }

        if !filename.starts_with("0x") {
            println!("File {} is not named after an offset (Ex.: 0x69696969.file). Skipping", filename);
            continue;
        }

        match rdb.entries.iter_mut().find(|x| &x.get_external_path().to_str().unwrap().to_owned() == filename)  {
            Some(entry_found) => {
                println!("Patching {}", filename);
                entry_found.make_external();
                entry_found.make_uncompressed();
                entry_found.set_external_file(&metadata);
            },
            None => println!("File {} not found in the RDB. Skipping.", filename),
        }
    }

    let mut bytes = vec![];
    rdb.write(&mut bytes).unwrap();

    match std::fs::write(out_path, bytes) {
        Ok(_) => {},
        Err(err) => panic!(err),
    };

    Ok(())
}

fn main() {
    let opt = Opt::from_args();

    if let Err(error_msg) = patch_rdb(&opt.path, &opt.out_path) {
        println!("{}", error_msg);
    }
}

mod tests {
    use super::*;

    const TEST_CONTENTS: &[u8] = include_bytes!("../MaterialEditor.rdb");

    #[test]
    fn test() {
        
    }
}
