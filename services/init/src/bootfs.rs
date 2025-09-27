use bootproto::MemoryRange;
use core::{slice, str};

const HEADER_LEN: usize = 8;
const MAGIC: &[u8; 4] = b"RCFS";

#[derive(Clone, Copy)]
pub struct BootfsView {
    range: MemoryRange,
}

impl BootfsView {
    pub const fn empty() -> Self {
        Self {
            range: MemoryRange::empty(),
        }
    }

    pub const fn from_range(range: MemoryRange) -> Self {
        Self { range }
    }

    pub fn is_empty(&self) -> bool {
        self.range.length == 0
    }

    pub fn base(&self) -> u64 {
        self.range.base
    }

    pub fn length(&self) -> u64 {
        self.range.length
    }

    pub fn find_entry(&self, name: &str) -> Option<&'static [u8]> {
        let mut iter = self.entries()?;
        while let Some(entry) = iter.next() {
            if entry.name == name {
                return Some(entry.data);
            }
        }
        None
    }

    pub fn validate_manifest(&self) -> ManifestSummary {
        let bytes = match self.find_entry("services.manifest") {
            Some(bytes) => bytes,
            None => return ManifestSummary::error(ManifestError::MissingManifest),
        };

        let manifest = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return ManifestSummary::error(ManifestError::Utf8),
        };

        let mut services = 0u32;
        let mut has_service = false;
        for line in manifest.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let descriptor = match parse_service_line(line) {
                Ok(desc) => desc,
                Err(err) => return ManifestSummary::error(err),
            };

            if descriptor.artifact.is_empty() || self.find_entry(descriptor.artifact).is_none() {
                return ManifestSummary::error(ManifestError::MissingArtifact);
            }

            has_service = true;
            services = services.saturating_add(1);
        }

        if !has_service {
            return ManifestSummary::error(ManifestError::Empty);
        }

        ManifestSummary::ok(services)
    }

    fn entries(&self) -> Option<BootfsIter<'static>> {
        let data = unsafe { self.slice()? };
        BootfsIter::new(data)
    }

    unsafe fn slice(&self) -> Option<&'static [u8]> {
        if self.range.is_empty() {
            None
        } else {
            Some(slice::from_raw_parts(
                self.range.base as *const u8,
                self.range.length as usize,
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceDescriptor<'a> {
    pub name: &'a str,
    pub artifact: &'a str,
    pub entry: &'a str,
    pub capabilities: CapabilityList<'a>,
}

#[derive(Clone, Copy, Debug)]
pub struct CapabilityList<'a> {
    inner: &'a str,
}

impl<'a> CapabilityList<'a> {
    const fn from_str(inner: &'a str) -> Self {
        Self { inner }
    }

    pub fn iter(&self) -> CapabilityIter<'a> {
        CapabilityIter { inner: self.inner }
    }
}

pub struct CapabilityIter<'a> {
    inner: &'a str,
}

impl<'a> Iterator for CapabilityIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_empty() {
            return None;
        }
        let mut parts = self.inner.splitn(2, ',');
        let head = parts.next()?.trim();
        let tail = parts.next().unwrap_or("");
        self.inner = tail;
        if head.is_empty() {
            self.next()
        } else {
            Some(head)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ManifestSummary {
    pub services: u32,
    pub error: Option<ManifestError>,
}

impl ManifestSummary {
    pub const fn ok(services: u32) -> Self {
        Self {
            services,
            error: None,
        }
    }

    pub const fn error(error: ManifestError) -> Self {
        Self {
            services: 0,
            error: Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ManifestError {
    MissingManifest,
    Utf8,
    InvalidFormat,
    MissingArtifact,
    Empty,
}

struct BootfsEntry<'a> {
    name: &'a str,
    data: &'a [u8],
}

struct BootfsIter<'a> {
    data: &'a [u8],
    offset: usize,
    remaining: usize,
}

impl<'a> BootfsIter<'a> {
    fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < HEADER_LEN {
            return None;
        }
        if &data[..4] != MAGIC {
            return None;
        }
        let version = u16::from_le_bytes([data[4], data[5]]);
        if version != 1 {
            return None;
        }
        let count = u16::from_le_bytes([data[6], data[7]]) as usize;
        Some(Self {
            data,
            offset: HEADER_LEN,
            remaining: count,
        })
    }
}

impl<'a> Iterator for BootfsIter<'a> {
    type Item = BootfsEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        if self.offset + 2 > self.data.len() {
            return None;
        }
        let name_len =
            u16::from_le_bytes([self.data[self.offset], self.data[self.offset + 1]]) as usize;
        self.offset += 2;

        if self.offset + name_len > self.data.len() {
            return None;
        }
        let name_bytes = &self.data[self.offset..self.offset + name_len];
        self.offset += name_len;

        if self.offset + 4 > self.data.len() {
            return None;
        }
        let size = u32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]) as usize;
        self.offset += 4;

        if self.offset + size > self.data.len() {
            return None;
        }
        let payload = &self.data[self.offset..self.offset + size];
        self.offset += size;
        self.remaining -= 1;

        let name = str::from_utf8(name_bytes).ok()?;

        Some(BootfsEntry {
            name,
            data: payload,
        })
    }
}

fn parse_service_line(line: &'static str) -> Result<ServiceDescriptor<'static>, ManifestError> {
    let rest = line
        .strip_prefix("service:")
        .ok_or(ManifestError::InvalidFormat)?;
    let mut parts = rest.split(':');
    let name = parts.next().ok_or(ManifestError::InvalidFormat)?.trim();
    let artifact = parts.next().ok_or(ManifestError::InvalidFormat)?.trim();
    let entry = parts.next().ok_or(ManifestError::InvalidFormat)?.trim();
    let caps = parts.next().unwrap_or("").trim();
    if parts.next().is_some() {
        return Err(ManifestError::InvalidFormat);
    }
    if name.is_empty() || artifact.is_empty() || entry.is_empty() {
        return Err(ManifestError::InvalidFormat);
    }
    Ok(ServiceDescriptor {
        name,
        artifact,
        entry,
        capabilities: CapabilityList::from_str(caps),
    })
}
