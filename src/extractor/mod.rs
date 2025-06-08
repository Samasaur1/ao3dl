use std::{fmt::Display, io::Read};

use zip::ZipArchive;

#[derive(Debug)]
enum Error {
    TitleAttributeMissing,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TitleAttributeMissing => {
                return write!(f, "Missing 'dc:title' tag in content.opf");
            },
        }
    }
}

impl std::error::Error for Error {}

pub fn as_zip(bytes: bytes::Bytes) -> Result<ZipArchive<impl std::io::Read + std::io::Seek>, Box<dyn std::error::Error>> {
    let wrapped = std::io::Cursor::new(bytes);
    let zip = zip::ZipArchive::new(wrapped)?;

    zip.file_names().for_each(|f| {
        println!("zip has file {}", f);
    });

    Ok(zip)
}

pub fn title(zipped_epub: &mut ZipArchive<impl std::io::Read + std::io::Seek>) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = String::new();

    zipped_epub.by_name("content.opf")?
        .read_to_string(&mut buffer)?;

    let mut nsr = quick_xml::reader::NsReader::from_str(&buffer);

    loop {
        match nsr.read_event()? {
            quick_xml::events::Event::Eof => {
                return Err(Box::new(Error::TitleAttributeMissing))
            },
            quick_xml::events::Event::Start(tag) => {
                if tag.name().as_ref() == b"dc:title" {
                    let title = nsr.read_text(tag.name())?;
                    return Ok(title.to_string());
                };
            },
            _ => {
                continue;
            },
        }
    }
}

pub fn unzip_to<P: AsRef<std::path::Path>>(zipped_epub: &mut ZipArchive<impl std::io::Read + std::io::Seek>, dest: P) -> Result<(), Box<dyn std::error::Error>> {
    zipped_epub.extract(dest)?;

    Ok(())
}
