// TODO check out Bytes instead of Vec?
// https://users.rust-lang.org/t/can-someone-explain-how-bytes-is-better-than-vec/105934
// https://doc.rust-lang.org/beta/std/io/struct.Bytes.html
// TODO Should all the JpegSegment data really be on the heap?
// TODO Figure out how to construct Box<[u8]> directly instead of going through into_boxed_slice()

use byteorder::{ ByteOrder, BigEndian };
use std::collections::HashMap;

// TODO find a way to put these in an enum and use them
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
#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq)]
enum JpegSegmentType {
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

struct UnknownSegmentError { marker: u8 }
impl std::fmt::Debug for UnknownSegmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Unrecognized jpeg marker: {:#02X}", self.marker)
    }
}
impl std::fmt::Display for UnknownSegmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Unrecognized jpeg marker: {:#02X}", self.marker)
    }
}
impl std::error::Error for UnknownSegmentError {}

// TODO maybe just impl From and return a JpegSegmentType::UNKNOWN on unrecognized marker?
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
const APP13_RECORD_APP_VERSION: u8 = 0;
const APP13_RECORD_APP_KEYWORD: u8 = 25;
const APP13_RECORD_APP_CAPTION: u8 = 120;
const APP13_RECORD_APP_OBJECT_DATA_PREVIEW: u8 = 202;

struct IptcDataSet {
    record_number: u8,
    data_set_number: u8,
    data: Box<[u8]>,
}

// https://metacpan.org/dist/Image-MetaData-JPEG/view/lib/Image/MetaData/JPEG/Structures.pod
// http://www.iptc.org/std/IIM/4.2/specification/IIMV4.2.pdf (page 14)
// NOTE: When writing APP13 segment data, it should always be padded with a null byte to an
// even length. If the data is already an even length, no null byte padding is needed.
struct JpegApp13Segment {
    // From what I've seen from CO, this is always "Photoshop 3.0\0"
    // TODO CStr vs Box<[u8]>. CStr seems more self-documenting.
    id: Box<std::ffi::CStr>,
    // "8BIM" for photoshop 4.0+
    resource_type: u32,
    resource_id: u16,
    // Padded to be even ("\0\0" if no name)
    resource_name: Vec<u8>,
    // NOTE: When writing the data sets, the data must be padded with an additional \0 to remain
    // even!
    // Technically the APP13 segment can contain multiple records but Champs only seems to use
    // a single record: 2 - the application record.
    // See notes for data set id meanings.
    // key: data set number
    // value: data set
    // TODO Maybe swap out HashMap for https://crates.io/crates/fnv since the keys are so short.
    // NOTE May need to find another data structure for this if there's ever a case where we
    // have more than just the application record. Records must come in numeric order (datasets
    // can come in any) but hashmaps are unordered and therefore extra care would have to be
    // taken when writing records to a file. Could maybe do a vec of hashmaps but that seems
    // like overkill...
    data_sets: HashMap<u8, Vec<IptcDataSet>>
}

// https://dev.exiv2.org/projects/exiv2/wiki/The_Metadata_in_JPEG_files
struct JpegSegment {
    segment_type: JpegSegmentType,
    payload: Option<Box<[u8]>>,
    additional_data: Option<Box<[u8]>>,
}

fn main() {
    let test_file = std::env::var("TEST_FILE").expect("test file environment variable not found");
    let jpeg_raw = std::fs::read(test_file).expect("failed to read");
    let mut jpeg_segments: Vec<JpegSegment> = Vec::new();
    let mut offset = 0;
    loop {
        let magic = jpeg_raw[offset];
        debug_assert!(magic == 0xFF);
        offset += 1;

        let marker = jpeg_raw[offset];
        let segment_type = JpegSegmentType::try_from(marker).unwrap_or_else(|_| {
            panic!("Unknown marker {:#02X} at offset {}", marker, offset);
        });
        offset += 1;

        let segment_payload_size = match segment_type {
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
            => { 0 }

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
            => { BigEndian::read_u16(&jpeg_raw[offset..]) },
        };

        // NOTE: The size of the payload _includes_ the 2 bytes used for reporting the payload size
        println!("Saw marker {:#06x} with size {}", marker, segment_payload_size);

        if marker == JPEG_MARKER_SOS {
            let payload = jpeg_raw[offset .. offset + segment_payload_size as usize].to_owned().into_boxed_slice();
            offset += segment_payload_size as usize;

            // let mut one_past_image_data_end = offset;
            let image_data_start = offset;
            // The marker magic number (0xFF) may be encountered within the image scan,
            // specifically for 0xFF00 and 0xFFD0 - 0xFFD7 (RST). Keep scanning to find the start
            // of the next segment, denoted by the same magic 0xFF.
            while jpeg_raw[offset] != 0xFF || matches!(jpeg_raw[offset + 1], 0x00 | JPEG_MARKER_RST0 ..= JPEG_MARKER_RST7) {
                offset += 1;
            }
            let image_data = jpeg_raw[image_data_start .. offset].to_owned().into_boxed_slice();

            jpeg_segments.push(JpegSegment {
                segment_type: JpegSegmentType::SOS,
                payload: Some(payload),
                additional_data: Some(image_data)
            });
        } else if marker == JPEG_MARKER_APP13 {
            offset += 2; // advance past payload size bytes

            // Kind of just skipping all of the following for now, but later on we'll want to save
            // all this information so we can reconstruct a valid jpeg image file when writing data
            // back to disk.
            let expected_identifier = b"Photoshop 3.0\0";
            let identifier = &jpeg_raw[offset .. offset + expected_identifier.len()];
            debug_assert!(identifier == expected_identifier);
            offset += expected_identifier.len();

            let expected_resource_type: u32 = BigEndian::read_u32(b"8BIM");
            let resource_type = BigEndian::read_u32(&jpeg_raw[offset .. offset + 4]);
            debug_assert!(resource_type == expected_resource_type);
            offset += 4;

            let expected_id = 0x0404;
            let resource_id = BigEndian::read_u16(&jpeg_raw[offset..]);
            debug_assert!(resource_id == expected_id);
            offset += 2;

            let mut name_len = 1; // will always be at least 
            while jpeg_raw[offset + name_len - 1] != 0 { name_len += 1; }
            if name_len % 2 == 1 { name_len += 1; } // name is padded to be an even size
            let resource_name = &jpeg_raw[offset .. offset + name_len];
            debug_assert!(resource_name == b"\0\0"); // only in the case of CO costume saves it seems...
            offset += name_len;

            // unused now but will be important when writing back to disk!
            let _data_size = BigEndian::read_u32(&jpeg_raw[offset..]);
            offset += 4;

            // BEGIN READING IPTC DATA BLOCKS
            #[repr(packed)]
            #[allow(dead_code)]
            struct IptcDataBlockHeader {
                tag_marker: u8, // always 0x1C, can exclude?
                record_number: u8,
                data_set_number: u8,
                data_size_bytes: u16,
            }

            let mut data_sets: HashMap<u8, Vec<IptcDataSet>> = HashMap::new();
            while jpeg_raw[offset] == 0x1C {
                // UNSAFE: What if image is malformed and cuts out in the middle of trying to read a block?
                // Also, may be unaligned? Probably will be unaligned.
                // Although maybe it won't matter since the type we're casting to is repr(packed)...
                let header = unsafe { &mut *(jpeg_raw.as_ptr().add(offset) as *mut IptcDataBlockHeader) };
                offset += std::mem::size_of::<IptcDataBlockHeader>();
                // TODO will this screw up on platforms of different endianness?
                header.data_size_bytes = header.data_size_bytes.to_be();
                let data = &jpeg_raw[offset .. offset + header.data_size_bytes as usize];
                offset += header.data_size_bytes as usize;


                data_sets.entry(header.data_set_number)
                    .and_modify(|sets| sets.push(IptcDataSet { record_number: header.record_number, data_set_number: header.data_set_number, data: data.to_owned().into_boxed_slice() }))
                    .or_insert(vec![IptcDataSet { record_number: header.record_number, data_set_number: header.data_set_number, data: data.to_owned().into_boxed_slice() }]);
            }

            let payload = Box::new(JpegApp13Segment {
                id: std::ffi::CStr::from_bytes_with_nul(identifier).unwrap().into(),
                resource_type,
                resource_id,
                resource_name: resource_name.to_owned(),
                data_sets,
            });
            let payload = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(Box::into_raw(payload) as *mut u8, std::mem::size_of::<JpegApp13Segment>())) };

            jpeg_segments.push(JpegSegment {
                segment_type,
                payload: Some(payload),
                additional_data: None,
            });

            // remember that the whole app 13 segment payload is padded to be an even size
            if jpeg_raw[offset] == 0 { offset += 1; }
        } else {
            let payload = if segment_payload_size > 0 {
                Some(jpeg_raw[offset .. offset + segment_payload_size as usize].to_owned().into_boxed_slice())
            } else {
                None
            };

            jpeg_segments.push(JpegSegment {
                segment_type,
                payload,
                additional_data: None,
            });

            offset += segment_payload_size as usize;
        }

        if offset >= jpeg_raw.len() { break; }
    }

    let app13segment = jpeg_segments.iter().find(|seg| seg.segment_type == JpegSegmentType::APP13).unwrap();
    let data = unsafe { &*(app13segment.payload.as_ref().unwrap().as_ptr() as *mut JpegApp13Segment) };
    let costume = data.data_sets.get(&APP13_RECORD_APP_OBJECT_DATA_PREVIEW).unwrap();
    use std::io::prelude::*;
    let mut out = std::io::stdout();
    _ = out.write_all(&costume[0].data);
    _ = out.flush();
}
