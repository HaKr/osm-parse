#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use bzip2::read::BzDecoder;
use quick_xml::{events::Event, Reader};
use structopt::StructOpt;

#[derive(PartialEq, Eq, Hash, Debug)]
enum OsmTag {
    Node,
    Way,
    Relation,
}

enum FileFormats {
    XML,
    BZIP2,
}

#[derive(Default, Debug)]
struct TagInfo {
    starts: u64,
    ends: u64,
}

/// Parse an OSM data file
///    The data file may be either plain XML (.osm),
///    or archived (.osm.bz2)
///
///    It reports the number of Node, Way and relation tags.
///
///    Note: Parsing an archived file takes factors (~4x)
///          longer than a plan XML file.
#[derive(StructOpt, Debug)]
// #[structopt(name = "osm")]
struct Options {
    /// File to process (either .osm or .osm.bz2 extension)
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

type Info = HashMap<OsmTag, TagInfo>;
type OtherTags = HashSet<String>;

impl TryFrom<&[u8]> for OsmTag {
    type Error = bool;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.eq_ignore_ascii_case(b"node") {
            return Ok(Self::Node);
        } else if value.eq_ignore_ascii_case(b"way") {
            return Ok(Self::Way);
        } else if value.eq_ignore_ascii_case(b"relation") {
            return Ok(Self::Relation);
        }

        Err(false)
    }
}

impl TryFrom<&OsStr> for FileFormats {
    type Error = bool;

    fn try_from(os_str: &OsStr) -> Result<Self, Self::Error> {
        if let Some(s) = os_str.to_str() {
            if s.ends_with("osm.bz2") {
                return Ok(Self::BZIP2);
            } else if s.ends_with("osm") {
                return Ok(Self::XML);
            }
        }

        Err(false)
    }
}

fn main() {
    let options = Options::from_args();
    if let Ok(file_format) = FileFormats::try_from(options.file.as_os_str()) {
        let input_file = File::open(&options.file).expect("Open XML file");

        let mut reader: Reader<Box<dyn BufRead>> = match file_format {
            FileFormats::XML => {
                let buf_reader = Box::new(BufReader::new(input_file));
                Reader::from_reader(buf_reader)
            }

            FileFormats::BZIP2 => {
                let decompressor = BzDecoder::new(input_file);

                let buf_reader = Box::new(BufReader::new(decompressor));
                Reader::from_reader(buf_reader)
            }
        };

        let mut info: Info = HashMap::new();
        let mut others = HashSet::new();

        let mut register_tag = |add: bool, tag: &[u8]| {
            let osm_tag = OsmTag::try_from(tag);
            match osm_tag {
                Ok(tag) => {
                    let info_entry = info.entry(tag).or_default();
                    match add {
                        true => info_entry.starts += 1,
                        false => info_entry.ends += 1,
                    }
                }
                Err(_) => {
                    let tag_name = String::from_utf8_lossy(tag).to_string();
                    others.insert(tag_name);
                }
            }
        };

        let mut buf = Vec::new();

        loop {
            buf.clear();
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Eof => break,

                Event::Start(bytes) => {
                    register_tag(true, bytes.name().local_name().as_ref());
                }

                Event::Empty(bytes) => {
                    register_tag(true, bytes.name().local_name().as_ref());
                    register_tag(false, bytes.name().local_name().as_ref());
                }

                Event::End(bytes) => {
                    register_tag(false, bytes.name().local_name().as_ref());
                }

                _ => (),
            }
        }

        println!("... and done! \n\tinfo: {:?}\n\tOthers: {:?}", info, others);
    } else {
        println!("Only files with extension .osm or .osm.bz2 are supported.");
    }
}
