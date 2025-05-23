// TODO get rid of the dependency on ByteOrder
// TODO proper image decode of SOS segment
use byteorder::{ ByteOrder, BigEndian };
use std::collections::{ HashMap, BTreeMap };

const JPEG_MARKER_SOI: u8 = 0xD8;
const JPEG_MARKER_EOI: u8 = 0xD9;
const JPEG_MARKER_SOF0: u8 = 0xC0;
const JPEG_MARKER_SOF2: u8 = 0xC2;
const JPEG_MARKER_DHT: u8 = 0xC4;
const JPEG_MARKER_DQT: u8 = 0xDB;
const JPEG_MARKER_DRI: u8 = 0xDD;
const JPEG_MARKER_SOS: u8 = 0xDA;
const JPEG_MARKER_COM: u8 = 0xFE;
const JPEG_MARKER_RST0: u8 = 0xD0;
const JPEG_MARKER_RST1: u8 = 0xD1;
const JPEG_MARKER_RST2: u8 = 0xD2;
const JPEG_MARKER_RST3: u8 = 0xD3;
const JPEG_MARKER_RST4: u8 = 0xD4;
const JPEG_MARKER_RST5: u8 = 0xD5;
const JPEG_MARKER_RST6: u8 = 0xD6;
const JPEG_MARKER_RST7: u8 = 0xD7;
const JPEG_MARKER_APP0: u8 = 0xE0;
const JPEG_MARKER_APP1: u8 = 0xE1;
const JPEG_MARKER_APP2: u8 = 0xE2;
const JPEG_MARKER_APP3: u8 = 0xE3;
const JPEG_MARKER_APP4: u8 = 0xE4;
const JPEG_MARKER_APP5: u8 = 0xE5;
const JPEG_MARKER_APP6: u8 = 0xE6;
const JPEG_MARKER_APP7: u8 = 0xE7;
const JPEG_MARKER_APP8: u8 = 0xE8;
const JPEG_MARKER_APP9: u8 = 0xE9;
const JPEG_MARKER_APP10: u8 = 0xEA;
const JPEG_MARKER_APP11: u8 = 0xEB;
const JPEG_MARKER_APP12: u8 = 0xEC;
const JPEG_MARKER_APP13: u8 = 0xED;
const JPEG_MARKER_APP14: u8 = 0xEE;
const JPEG_MARKER_APP15: u8 = 0xEF;

#[repr(u8)]
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum JpegSegmentType {
    SOI = JPEG_MARKER_SOI,
    EOI = JPEG_MARKER_EOI,
    SOF0 = JPEG_MARKER_SOF0,
    SOF2 = JPEG_MARKER_SOF2,
    DHT = JPEG_MARKER_DHT,
    DQT = JPEG_MARKER_DQT,
    DRI = JPEG_MARKER_DRI,
    SOS = JPEG_MARKER_SOS,
    COM = JPEG_MARKER_COM,
    RST0 = JPEG_MARKER_RST0,
    RST1 = JPEG_MARKER_RST1,
    RST2 = JPEG_MARKER_RST2,
    RST3 = JPEG_MARKER_RST3,
    RST4 = JPEG_MARKER_RST4,
    RST5 = JPEG_MARKER_RST5,
    RST6 = JPEG_MARKER_RST6,
    RST7 = JPEG_MARKER_RST7,
    APP0 = JPEG_MARKER_APP0,
    APP1 = JPEG_MARKER_APP1,
    APP2 = JPEG_MARKER_APP2,
    APP3 = JPEG_MARKER_APP3,
    APP4 = JPEG_MARKER_APP4,
    APP5 = JPEG_MARKER_APP5,
    APP6 = JPEG_MARKER_APP6,
    APP7 = JPEG_MARKER_APP7,
    APP8 = JPEG_MARKER_APP8,
    APP9 = JPEG_MARKER_APP9,
    APP10 = JPEG_MARKER_APP10,
    APP11 = JPEG_MARKER_APP11,
    APP12 = JPEG_MARKER_APP12,
    APP13 = JPEG_MARKER_APP13,
    APP14 = JPEG_MARKER_APP14,
    APP15 = JPEG_MARKER_APP15,
}

pub struct UnknownSegmentError { marker: u8 }

impl TryFrom<u8> for JpegSegmentType {
    type Error = UnknownSegmentError;
    fn try_from(marker: u8) -> Result<Self, Self::Error> {
        match marker {
            JPEG_MARKER_SOI => Ok(JpegSegmentType::SOI),
            JPEG_MARKER_EOI => Ok(JpegSegmentType::EOI),
            JPEG_MARKER_SOF0 => Ok(JpegSegmentType::SOF0),
            JPEG_MARKER_SOF2 => Ok(JpegSegmentType::SOF2),
            JPEG_MARKER_DHT => Ok(JpegSegmentType::DHT),
            JPEG_MARKER_DQT => Ok(JpegSegmentType::DQT),
            JPEG_MARKER_DRI => Ok(JpegSegmentType::DRI),
            JPEG_MARKER_SOS => Ok(JpegSegmentType::SOS),
            JPEG_MARKER_COM => Ok(JpegSegmentType::COM),
            JPEG_MARKER_RST0 => Ok(JpegSegmentType::RST0),
            JPEG_MARKER_RST1 => Ok(JpegSegmentType::RST1),
            JPEG_MARKER_RST2 => Ok(JpegSegmentType::RST2),
            JPEG_MARKER_RST3 => Ok(JpegSegmentType::RST3),
            JPEG_MARKER_RST4 => Ok(JpegSegmentType::RST4),
            JPEG_MARKER_RST5 => Ok(JpegSegmentType::RST5),
            JPEG_MARKER_RST6 => Ok(JpegSegmentType::RST6),
            JPEG_MARKER_RST7 => Ok(JpegSegmentType::RST7),
            JPEG_MARKER_APP0 => Ok(JpegSegmentType::APP0),
            JPEG_MARKER_APP1 => Ok(JpegSegmentType::APP1),
            JPEG_MARKER_APP2 => Ok(JpegSegmentType::APP2),
            JPEG_MARKER_APP3 => Ok(JpegSegmentType::APP3),
            JPEG_MARKER_APP4 => Ok(JpegSegmentType::APP4),
            JPEG_MARKER_APP5 => Ok(JpegSegmentType::APP5),
            JPEG_MARKER_APP6 => Ok(JpegSegmentType::APP6),
            JPEG_MARKER_APP7 => Ok(JpegSegmentType::APP7),
            JPEG_MARKER_APP8 => Ok(JpegSegmentType::APP8),
            JPEG_MARKER_APP9 => Ok(JpegSegmentType::APP9),
            JPEG_MARKER_APP10 => Ok(JpegSegmentType::APP10),
            JPEG_MARKER_APP11 => Ok(JpegSegmentType::APP11),
            JPEG_MARKER_APP12 => Ok(JpegSegmentType::APP12),
            JPEG_MARKER_APP13 => Ok(JpegSegmentType::APP13),
            JPEG_MARKER_APP14 => Ok(JpegSegmentType::APP14),
            JPEG_MARKER_APP15 => Ok(JpegSegmentType::APP15),
            _ => Err(UnknownSegmentError { marker }),
        }
    }
}

impl From<JpegSegmentType> for u8 {
    fn from(segment_type: JpegSegmentType) -> Self {
        segment_type as Self
    }
}

// TODO better naming scheme for these
pub const APP13_RECORD_APP: u8 = 2;
pub const APP13_RECORD_APP_VERSION: u8 = 0;
pub const APP13_RECORD_APP_KEYWORD: u8 = 25;
pub const APP13_RECORD_APP_CAPTION: u8 = 120;
pub const APP13_RECORD_APP_OBJECT_DATA_PREVIEW: u8 = 202;

#[repr(Rust, packed)]
#[allow(dead_code)]
struct PackedIptcDatasetHeader {
    tag_marker: u8,
    record_number: u8,
    dataset_number: u8,
    data_size_bytes: u16,
}

pub struct IptcDataset {
    pub record_number: u8,
    pub dataset_number: u8,
    pub data: Box<[u8]>,
}

pub trait SegmentPayload {}

// https://metacpan.org/dist/Image-MetaData-JPEG/view/lib/Image/MetaData/JPEG/Structures.pod
// http://www.iptc.org/std/IIM/4.2/specification/IIMV4.2.pdf (page 14)
// NOTE We do NOT account for _extended_ IPTC data sets (yet)!
impl SegmentPayload for JpegApp13Payload {}
pub struct JpegApp13Payload {
    /// Null-terminated identifier e.g. "Photoshop 3.0\0"
    pub id: Box<[u8]>,
    // "8BIM" for photoshop 4.0+
    pub resource_type: u32,
    pub resource_id: u16,
    // Padded to be even ("\0\0" if no name)
    // TODO Maybe somehow enforce above whenever someone tries to change the resource name?
    pub resource_name: Box<[u8]>,
    // Key: u16, combination of u8 record # and u8 dataset #
    // Value: Vec of datasets initially in the order that they were seen during parsing.
    datasets: BTreeMap<u16, Vec<IptcDataset>>
}

fn to_iptc_dataset_key(record_number: u8, dataset_number: u8) -> u16 {
    ((record_number as u16) << 8) | dataset_number as u16
}

impl JpegApp13Payload {
    pub fn get_datasets(&self, record_number: u8, dataset_number: u8) -> Option<&Vec<IptcDataset>> {
        let key = to_iptc_dataset_key(record_number, dataset_number);
        let result = self.datasets.get(&key);
        result
    }

    pub fn get_datasets_mut(&mut self, record_number: u8, dataset_number: u8) -> Option<&mut Vec<IptcDataset>> {
        let key = to_iptc_dataset_key(record_number, dataset_number);
        let result = self.datasets.get_mut(&key);
        result
    }
}

// https://dev.exiv2.org/projects/exiv2/wiki/The_Metadata_in_JPEG_files
pub struct JpegSegment {
    segment_type: JpegSegmentType,
    payload: Option<Box<[u8]>>,
    // NOTE(RA): Currently no expectation that any additional data needs to be modified after load.
    additional_data: Option<Box<[u8]>>,
}

// FIXME need to guard against the payload not matching the segment type!
impl JpegSegment {
    pub fn get_payload_as<T: SegmentPayload>(&self) -> &T {
        let payload = self.payload.as_ref().unwrap();
        unsafe { &*(payload.as_ptr() as *const T) }
    }

    pub fn get_payload_as_mut<T: SegmentPayload>(&mut self) -> &mut T {
        let payload = self.payload.as_mut().unwrap();
        unsafe { &mut *(payload.as_ptr() as *mut T) }
    }

    fn serialize(&self) -> Box<[u8]> {
        let mut serialized_segment = vec![0xFF, self.segment_type as u8];

        match self.segment_type {
            // TODO There's probably a way to do this without allocating so much memory.
            // Maybe we can find a way to calcalate the exact size of the segment, then just create
            // a vec with that capacity and write directly into it. Might take a bit more
            // computation but it may be better than repeated heap allocations.
            // TODO Eventually profile current method vs size calc then 1 heap alloc
            // (we have a lot of pointers to heap-alloc'd data that we need to follow to get the
            // sizes of things so traversing once to get sizes then alloc'ing then traversing again
            // to copy/write data might not be cache-friendly, so we'd need to make sure that it
            // really is better speed-wise and mem-wise).
            JpegSegmentType::APP13 => {
                let payload = self.get_payload_as::<JpegApp13Payload>();

                let mut serialized_datasets: Vec<u8> = Vec::new();
                for (_, datasets) in payload.datasets.iter() {
                    for dataset in datasets {
                        // TODO maybe use PackedIptcDatasetHeader?
                        serialized_datasets.push(0x1C);
                        serialized_datasets.push(dataset.record_number);
                        serialized_datasets.push(dataset.dataset_number);
                        // NOTE the length of the data set data does NOT include the bytes used to
                        // report the length.
                        serialized_datasets.extend((dataset.data.len() as u16).to_be_bytes());
                        serialized_datasets.extend(dataset.data.clone());
                    }
                }
                if serialized_datasets.len() % 2 == 1 { serialized_datasets.push(0); }

                let mut serialized_payload: Vec<u8> = vec![];
                serialized_payload.extend(&payload.id);
                serialized_payload.extend((payload.resource_type).to_be_bytes());
                serialized_payload.extend((payload.resource_id).to_be_bytes());
                // TODO See note on JpegApp13Payload resource_name field. If we _do_ end up
                // providing a interface that must be used to get and set the resource name, then
                // we don't need to enforce padding here.
                if payload.resource_name.is_empty() {
                    serialized_payload.push(0);
                    serialized_payload.push(0);
                } else {
                    serialized_payload.extend(payload.resource_name.clone());
                    if payload.resource_name.len() % 2 == 1 { serialized_payload.push(0); }
                }
                serialized_payload.extend((serialized_datasets.len() as u32).to_be_bytes());
                serialized_payload.extend(serialized_datasets);

                serialized_segment.extend(((serialized_payload.len() + std::mem::size_of::<u16>()) as u16).to_be_bytes());
                serialized_segment.extend(serialized_payload);
            },

            _ => {
                if let Some(payload) = &self.payload {
                    serialized_segment.extend(((payload.len() + std::mem::size_of::<u16>()) as u16).to_be_bytes());
                    serialized_segment.extend(payload.iter().copied());
                }
                if let Some(additional_data) = &self.additional_data {
                    serialized_segment.extend(additional_data.iter().copied());
                }
            }
        }

        serialized_segment.into_boxed_slice()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParseError {
    /// Segment markers are always preceded by an 0xFF byte but we encountered something else.
    InvalidSegmentMagic { magic: u8 },
    /// We don't recognize or support the segment marker we encountered.
    UnrecognizedSegmentMarker { marker: u8, offset: usize },
    /// It was determined that there were not enough bytes left to read the segment payload.
    PayloadInterrupted { marker: u8, payload_size: u16, offset: usize },
    /// Something went wrong when attempting to parse segment-specific payload data.
    MalformedSegmentPayload { marker: u8, offset: usize },
    /// An IO error occurred during parsing.
    IOError(std::io::Error)
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidSegmentMagic { magic } => write!(
                f,
                "Invalid segment magic, expected 0xFF but got {magic:#02X}"
            ),

            Self::UnrecognizedSegmentMarker { marker, offset } => write!(
                f,
                "Unrecognized marker at byte {offset}: {marker:#02X}"
            ),

            Self::PayloadInterrupted { marker, payload_size, offset } => write!(
                f,
                "Not enough bytes left to read {marker:#02X} payload of size {payload_size} at byte {offset}"
            ),

            Self::MalformedSegmentPayload { marker, offset } => write!(
                f,
                "Malformed {marker:#02X} payload at byte {offset}"
            ),

            Self::IOError(io_err) => write!(f, "{io_err}"),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error)
    }
}

pub struct Jpeg {
    // TODO swap out the hashing function for something faster (default isn't great for small keys)
    segment_indices: HashMap<JpegSegmentType, Vec<usize>>,
    segments: Vec<JpegSegment>
}

impl Jpeg {
    pub fn parse(jpeg_raw: &[u8]) -> Result<Self, ParseError>  {
        use std::io::{prelude::*, BufRead, Cursor};
        let mut jpeg_raw = Cursor::new(jpeg_raw);

        let mut parsed = Self { segment_indices: HashMap::new(), segments: Vec::new() };
        loop {
            let mut magic = [0u8];
            let bytes_read = jpeg_raw.read(&mut magic)?;
            if bytes_read == 0 { break; }
            let magic = magic[0];
            if magic != 0xFF { return Err(ParseError::InvalidSegmentMagic { magic }); }

            let mut marker = [0u8];
            let marker_position = jpeg_raw.position();
            jpeg_raw.read_exact(&mut marker)?;
            let marker = marker[0];
            let segment_type = JpegSegmentType::try_from(marker).map_err(|err| ParseError::UnrecognizedSegmentMarker { marker: err.marker, offset: marker_position as usize })?;

            // NOTE: The size of the payload _includes_ the 2 bytes used for reporting the payload size
            let segment_payload_size: u16 = match segment_type {
                JpegSegmentType::SOI
                | JpegSegmentType::EOI
                | JpegSegmentType::RST0
                | JpegSegmentType::RST1
                | JpegSegmentType::RST2
                | JpegSegmentType::RST3
                | JpegSegmentType::RST4
                | JpegSegmentType::RST5
                | JpegSegmentType::RST6
                | JpegSegmentType::RST7
                => { 0 },

                JpegSegmentType::SOF0
                | JpegSegmentType::SOF2
                | JpegSegmentType::DHT
                | JpegSegmentType::DQT
                | JpegSegmentType::DRI
                | JpegSegmentType::SOS
                | JpegSegmentType::COM
                | JpegSegmentType::APP0
                | JpegSegmentType::APP1
                | JpegSegmentType::APP2
                | JpegSegmentType::APP3
                | JpegSegmentType::APP4
                | JpegSegmentType::APP5
                | JpegSegmentType::APP6
                | JpegSegmentType::APP7
                | JpegSegmentType::APP8
                | JpegSegmentType::APP9
                | JpegSegmentType::APP10
                | JpegSegmentType::APP11
                | JpegSegmentType::APP12
                | JpegSegmentType::APP13
                | JpegSegmentType::APP14
                | JpegSegmentType::APP15
                => {
                    let mut size = [0u8; 2];
                    // NOTE: This will advance the cursor past the 2 bytes we used to report the
                    // length, which is what we want. We don't store this data explicitly since
                    // Rust will keep track of our data lengths for us in the [u8] slices.
                    jpeg_raw.read_exact(&mut size)?;
                    let mut size = BigEndian::read_u16(&size);
                    size -= 2; // we don't include the size of the payload itself in upcoming calculations
                    size
                },
            };

            {
                // NOTE I think may only be valid if the underlying cursor type is a vec which
                // contains all bytes in the file. I don't think this is valid if the underlying
                // vec is buffered (i.e. a subset of the full file)
                // FIXME if we switch to buffered reading
                let offset = jpeg_raw.position() as usize;
                if segment_payload_size > 0 && offset + segment_payload_size as usize >= jpeg_raw.get_ref().len() {
                    return Err(ParseError::PayloadInterrupted {
                        marker,
                        payload_size: segment_payload_size,
                        offset,
                    });
                }
            }

            // TODO maybe read the entire payload into a slice up here then parse from that slice
            // in each section below. Can create a new cursor over the slice, but would have to fix
            // up file byte offsets when returning errors.
            match marker {
                JPEG_MARKER_SOS => {
                    let mut payload = vec![0u8; segment_payload_size as usize];
                    // SAFETY: We've already determined that there are enough bytes left to read the entire payload.
                    jpeg_raw.read_exact(&mut payload)?;
                    let payload = payload.into_boxed_slice();

                    // The marker magic number (0xFF) may be encountered within the image scan,
                    // specifically for 0xFF00 and 0xFFD0 - 0xFFD7 (RST). Keep scanning to find the start
                    // of the next segment, denoted by the same magic 0xFF.
                    let mut image_data: Vec<u8> = Vec::new();
                    loop {
                        let bytes_read = jpeg_raw.read_until(0xFF, &mut image_data)?;
                        if bytes_read == 0 {
                            return Err(ParseError::IOError(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
                        }

                        let mut marker = [0u8];
                        jpeg_raw.read_exact(&mut marker)?;
                        let marker = marker[0];
                        if matches!(marker, 0 | JPEG_MARKER_RST0 ..= JPEG_MARKER_RST7) {
                            image_data.push(marker);
                        } else {
                            break;
                        }
                    }
                    // We've found a new marker that terminates the image scan so we need to pop
                    // 0xFF off the image data and back our cursor up a bit to prime for the next
                    // loop of the jpeg parser.
                    image_data.pop();
                    jpeg_raw.seek_relative(-2)?;

                    let segment_type = JpegSegmentType::SOS;
                    let index = parsed.segments.len();
                    parsed.segments.push(JpegSegment {
                        segment_type,
                        payload: Some(payload),
                        additional_data: Some(image_data.into_boxed_slice())
                    });
                    parsed.segment_indices.entry(segment_type).or_default().push(index);
                },

                // Because some fields in the APP13 header are variable length we need to make sure
                // we don't hit EOF while decoding (very unlikely to happen; it's more likely that
                // we'll become misaligned and start bleeding into another segment, but we should
                // hit another error such as InvalidSegmentMagic at that point).
                JPEG_MARKER_APP13 => {
                    let mut identifier: Vec<u8> = Vec::new();
                    jpeg_raw.read_until(0, &mut identifier)?;

                    let mut resource_type = [0u8; 4];
                    jpeg_raw.read_exact(&mut resource_type)?;
                    let resource_type = BigEndian::read_u32(&resource_type);

                    let mut resource_id = [0u8; 2];
                    jpeg_raw.read_exact(&mut resource_id)?;
                    let resource_id = BigEndian::read_u16(&resource_id);

                    let mut resource_name: Vec<u8> = Vec::new();
                    let bytes_read = jpeg_raw.read_until(0, &mut resource_name)?;
                    if bytes_read == 0 {
                        return Err(ParseError::IOError(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
                    }

                    // resource name should be padded with a null byte to be an even length
                    if resource_name.len() % 2 == 1 {
                        let mut padded_null_byte = [0u8];
                        jpeg_raw.read_exact(&mut padded_null_byte)?;
                        let padded_null_byte = padded_null_byte[0];
                        if padded_null_byte != 0 {
                            return Err(ParseError::MalformedSegmentPayload { marker, offset: jpeg_raw.position() as usize - 1 });
                        }
                        resource_name.push(padded_null_byte);
                    }

                    // skip past the data size since we're going to recompute it when repacking
                    jpeg_raw.seek_relative(4)?;

                    // BEGIN READING IPTC DATA BLOCKS
                    let mut datasets: BTreeMap<u16, Vec<IptcDataset>> = BTreeMap::new();
                    const IPTC_DATASET_TAG_MARKER: u8 = 0x1C;
                    // NOTE This works properly if the cursor is wrapping a full unbuffered
                    // vec. Not sure if it'll keep working if we switch to a BufReader over a file
                    // or something.
                    // FIXME if we switch to buffered reading
                    while jpeg_raw.get_ref()[jpeg_raw.position() as usize] == IPTC_DATASET_TAG_MARKER {
                        let mut header = [0u8; std::mem::size_of::<PackedIptcDatasetHeader>()];
                        jpeg_raw.read_exact(&mut header)?;
                        let mut header: PackedIptcDatasetHeader = unsafe { std::mem::transmute(header) };
                        // NOTE Remember that all data in a jpeg is stored big-endian...
                        #[cfg(target_endian = "little")]
                        {
                            header.data_size_bytes = header.data_size_bytes.swap_bytes(); 
                        }
                        let mut data = vec![0u8; header.data_size_bytes as usize];
                        jpeg_raw.read_exact(&mut data)?;
                        let key = to_iptc_dataset_key(header.record_number, header.dataset_number);
                        let dataset = IptcDataset {
                            record_number: header.record_number,
                            dataset_number: header.dataset_number,
                            data: data.into_boxed_slice()
                        };
                        if let Some(sets) = datasets.get_mut(&key) {
                            sets.push(dataset);
                        } else {
                            datasets.insert(key, vec![dataset]);
                        }
                    }

                    let payload = Box::new(JpegApp13Payload {
                        // SAFETY: We've already done checking to ensure that our identifier is
                        // null-terminated and doesn't contain interior null bytes.
                        id: identifier.into_boxed_slice(),
                        resource_type,
                        resource_id,
                        resource_name: resource_name.to_owned().into_boxed_slice(),
                        datasets,
                    });
                    let payload = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(Box::into_raw(payload) as *mut u8, std::mem::size_of::<JpegApp13Payload>())) };

                    let segment_type = JpegSegmentType::APP13;
                    let index = parsed.segments.len();
                    parsed.segments.push(JpegSegment {
                        segment_type,
                        payload: Some(payload),
                        additional_data: None,
                    });
                    parsed.segment_indices.entry(segment_type).or_default().push(index);

                    // remember that the whole app 13 segment payload is padded to be an even size
                    // NOTE I think this may only work so long as the underlying buffer contains
                    // all the bytes of the raw jpeg. If it's buffered then this may not work properly?
                    // Just something to keep an eye out for in the future.
                    if jpeg_raw.get_ref()[jpeg_raw.position() as usize] == 0 {
                        jpeg_raw.seek_relative(1)?;
                    }
                },

                _ => {
                    let payload = if segment_payload_size > 0 {
                        let mut payload = vec![0u8; segment_payload_size as usize];
                        jpeg_raw.read_exact(&mut payload)?;
                        Some(payload.into_boxed_slice())
                    } else {
                        None
                    };

                    parsed.segments.push(JpegSegment {
                        segment_type,
                        payload,
                        additional_data: None,
                    });
                },
            };
        }

        // TODO additional jpeg validation here

        Ok(parsed)
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut encoded = vec![];
        for segment in self.segments.iter() {
            let serialized_segment = segment.serialize();
            encoded.extend(serialized_segment);
        }

        encoded.into_boxed_slice()
    }

    pub fn get_segment(&self, segment_type: JpegSegmentType) -> Option<Vec<&JpegSegment>> {
        self.segment_indices
            .get(&segment_type)
            .map(|indices| indices.iter().map(|index| &self.segments[*index]).collect())
    }

    pub fn get_segment_mut(&mut self, segment_type: JpegSegmentType) -> Option<Vec<&mut JpegSegment>> {
        let indices = self.segment_indices.get(&segment_type)?;
        let mut result = Vec::new();
        for index in indices {
            result.push(unsafe { &mut *(&mut self.segments[*index] as *mut JpegSegment) });
        }

        Some(result)
    }
}
