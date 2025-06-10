use std::{fmt, io::{self, Read, Seek}, path};

use anyhow::Context;
use zip::ZipArchive;

#[derive(Debug)]
enum Error {
    TitleAttributeMissing,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TitleAttributeMissing => {
                return write!(f, "Missing 'dc:title' tag in content.opf");
            },
        }
    }
}

impl std::error::Error for Error {}

pub fn as_zip(bytes: bytes::Bytes) -> anyhow::Result<ZipArchive<impl Read + Seek>> {
    let wrapped = io::Cursor::new(bytes);
    let zip = zip::ZipArchive::new(wrapped)
        .context("Could not create ZipArchive from bytes")?;
    Ok(zip)
}

pub fn title(zipped_epub: &mut ZipArchive<impl Read + Seek>) -> anyhow::Result<String> {
    let mut buffer = String::new();

    log::trace!("Extracting content.opf from zipped EPUB");

    zipped_epub
        .by_name("content.opf")
        .context("Could not extract content.opf from zipped EPUB")?
        .read_to_string(&mut buffer)
        .context("Could not read content.opf to string")?;

    log::trace!("Extracted content.opf from zipped EPUB");

    log::trace!("Parsing content.opf as XML");

    let mut nsr = quick_xml::reader::NsReader::from_str(&buffer);

    loop {
        match nsr.read_event()? {
            quick_xml::events::Event::Eof => {
                return Err(Error::TitleAttributeMissing.into())
            },
            quick_xml::events::Event::Start(tag) => {
                // log::trace!(target: "ao3dl::extractor::verbose_xml", "Found XML tag {:?}", tag.name().as_ref());
                if tag.name().as_ref() == b"dc:title" {
                    log::trace!("Located 'dc:title' tag");
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

pub fn unzip_to<P: AsRef<path::Path>>(zipped_epub: &mut ZipArchive<impl Read + Seek>, dest: P) -> anyhow::Result<()> {
    zipped_epub
        .extract(dest)
        .context("Cannot extract zipped EPUB to directory")?;

    Ok(())
}
