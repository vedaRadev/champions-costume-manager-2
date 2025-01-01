// TODO check out Bytes instead of Vec?
// https://users.rust-lang.org/t/can-someone-explain-how-bytes-is-better-than-vec/105934
// https://doc.rust-lang.org/beta/std/io/struct.Bytes.html

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
// const JPEG_MARKER_RST1: u8 = 0xD1;
// const JPEG_MARKER_RST2: u8 = 0xD2;
// const JPEG_MARKER_RST3: u8 = 0xD3;
// const JPEG_MARKER_RST4: u8 = 0xD4;
// const JPEG_MARKER_RST5: u8 = 0xD5;
// const JPEG_MARKER_RST6: u8 = 0xD6;
const JPEG_MARKER_RST7: u8 = 0xD7;
const JPEG_MARKER_APP0: u8 = 0xE0;
// const JPEG_MARKER_APP1: u8 = 0xE1;
// const JPEG_MARKER_APP2: u8 = 0xE2;
// const JPEG_MARKER_APP3: u8 = 0xE3;
// const JPEG_MARKER_APP4: u8 = 0xE4;
// const JPEG_MARKER_APP5: u8 = 0xE5;
// const JPEG_MARKER_APP6: u8 = 0xE6;
// const JPEG_MARKER_APP7: u8 = 0xE7;
// const JPEG_MARKER_APP8: u8 = 0xE8;
// const JPEG_MARKER_APP9: u8 = 0xE9;
// const JPEG_MARKER_APP10: u8 = 0xEA;
// const JPEG_MARKER_APP11: u8 = 0xEB;
// const JPEG_MARKER_APP12: u8 = 0xEC;
const JPEG_MARKER_APP13: u8 = 0xED;
// const JPEG_MARKER_APP14: u8 = 0xEE;
const JPEG_MARKER_APP15: u8 = 0xEF;

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

#[allow(clippy::upper_case_acronyms)]
// https://dev.exiv2.org/projects/exiv2/wiki/The_Metadata_in_JPEG_files
enum JpegSegment {
    Unknown { marker: u8, payload: Option<Box<[u8]>> },
    SOI,
    EOI,
    RSTn { n: u8 },
    SOF0 { payload: Box<[u8]> },
    SOF2 { payload: Box<[u8]> },
    DHT { payload: Box<[u8]> },
    DQT { payload: Box<[u8]> },
    DRI { payload: u16 },
    // NOTE: When writing SOS segment data, the image data does NOT count as part of the payload length.
    S0S { payload: Box<[u8]>, image_data: Box<[u8]> },
    APPn { n: u8, payload: Box<[u8]> },
    // https://metacpan.org/dist/Image-MetaData-JPEG/view/lib/Image/MetaData/JPEG/Structures.pod
    // http://www.iptc.org/std/IIM/4.2/specification/IIMV4.2.pdf (page 14)
    // NOTE: When writing APP13 segment data, it should always be padded with a null byte to an
    // even length. If the data is already an even length, no null byte padding is needed.
    APP13 {
        // From what I've seen from CO, this is always "Photoshop 3.0\0"
        // TODO CStr vs Box<[u8]>. CStr seems more self-documenting.
        id: Box<std::ffi::CStr>,
        // "8BIM" for photoshop 4.0+
        resource_type: u32,
        resource_id: u16,
        // Padded to be even ("\0\0" if no name)
        resource_name: Box<[u8]>,
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
}

// // TODO maybe use this instead of the tagged union? Saves some space.
// struct JpegSegment2 {
//     marker: u8,
//     payload: Option<Box<[u8]>>,
//     additional_data: Option<Box<[u8]>>,
// }

// impl JpegSegment2 {
//     // TODO return custom errors
//     fn get_payload<T>(&self) -> Result<&T, ()>  {
//         if let Some(payload) = self.payload.as_ref() {
//             if payload.len() != std::mem::size_of::<T>() { return Err(()); }
//             unsafe { Ok(&*(payload.as_ptr() as *const T)) }
//         } else {
//             Err(())
//         }
//     }
// }

fn main() {
    let test_file = std::env::var("TEST_FILE").expect("test file environment variable not found");
    let jpeg_raw = std::fs::read(test_file).expect("failed to read");
    let mut offset = 0;
    loop {
        let magic = jpeg_raw[offset];
        debug_assert!(magic == 0xFF);
        offset += 1;

        let marker = jpeg_raw[offset];
        offset += 1;

        let marker_payload_size = match marker {
            JPEG_MARKER_SOI
            | JPEG_MARKER_EOI
            | JPEG_MARKER_RST0 ..= JPEG_MARKER_RST7
            => { 0 }

            JPEG_MARKER_SOF0
            | JPEG_MARKER_SOF2
            | JPEG_MARKER_DHT
            | JPEG_MARKER_DQT
            | JPEG_MARKER_DRI
            | JPEG_MARKER_SOS
            | JPEG_MARKER_COM
            | JPEG_MARKER_APP0 ..= JPEG_MARKER_APP15
            => { BigEndian::read_u16(&jpeg_raw[offset..]) },

            unknown_marker => { panic!("Unknown marker {:#06x} at offset {}", unknown_marker, offset) },
        };

        // NOTE: The size of the payload _includes_ the 2 bytes used for reporting the payload size
        println!("Saw marker {:#06x} with size {}", marker, marker_payload_size);

        if marker == JPEG_MARKER_SOS {
            // https://stackoverflow.com/questions/26715684/parsing-jpeg-sos-marker
            // Once SOS marker is encountered, we have to manually scan through the compressed data until we
            // find the next real marker.
            // The image data comes AFTER the SOS marker payload.
            offset += marker_payload_size as usize;
            // Skip all of the image data for now.
            while jpeg_raw[offset] != 0xFF
            || matches!(jpeg_raw[offset + 1], 0x00 | JPEG_MARKER_RST0 ..= JPEG_MARKER_RST7)
            {
                offset += 1;
            }
        } else if marker == JPEG_MARKER_APP13 {
            // FORMAT OF APP13 SEGMENT
            // marker - 2 bytes
            // payload_size - 2 bytes
            // identifier (usually "Photoshop 3.0\0" but can be other things) - variable length
            // resource type - 4 bytes ("8BIM" for photoshop version 4.0+)
            // resource id - 2 bytes, "\004\004" for IPTC resource blocks but can be other things
            // name - variable, padded to be even ("\0\0" if no name)
            // data size - 4 bytes
            // data - variable, padded to be even
            offset += 2; // advance past payload size bytes

            // Kind of just skipping all of the following for now, but later on we'll want to save
            // all this information so we can reconstruct a valid jpeg image file when writing data
            // back to disk.
            let expected_identifier = b"Photoshop 3.0\0";
            let identifier = &jpeg_raw[offset .. offset + expected_identifier.len()];
            debug_assert!(identifier == expected_identifier);
            offset += expected_identifier.len();

            let expected_resource_type = b"8BIM";
            let resource_type = &jpeg_raw[offset .. offset + expected_resource_type.len()];
            debug_assert!(resource_type == expected_resource_type);
            offset += expected_resource_type.len();

            let expected_id = 0x0404;
            let id = BigEndian::read_u16(&jpeg_raw[offset..]);
            debug_assert!(id == expected_id);
            offset += 2;

            let mut name_len = 1; // will always be at least 
            while jpeg_raw[offset + name_len - 1] != 0 { name_len += 1; }
            if name_len % 2 == 1 { name_len += 1; } // name is padded to be an even size
            let name = &jpeg_raw[offset .. offset + name_len];
            debug_assert!(name == b"\0\0"); // only in the case of CO costume saves it seems...
            offset += name_len;

            // unused now but will be important when writing back to disk!
            let _data_size = BigEndian::read_u32(&jpeg_raw[offset..]);
            offset += 4;

            // BEGIN READING IPTC DATA BLOCKS
            #[repr(packed)]
            struct IptcDataBlockHeader {
                tag_marker: u8, // always 0x1C, can exclude?
                record_number: u8,
                data_set_number: u8,
                data_size_bytes: u16,
            }

            let mut data_blocks: Vec<(&IptcDataBlockHeader, &[u8])> = Vec::new();
            while jpeg_raw[offset] == 0x1C {
                // UNSAFE: What if image is malformed and cuts out in the middle of trying to read a block?
                // Also, may be unaligned? Probably will be unaligned.
                // Although maybe it won't matter since the type we're casting to is repr(packed)...
                let header = unsafe { &mut *(jpeg_raw.as_ptr().add(offset) as *mut IptcDataBlockHeader) };
                offset += std::mem::size_of::<IptcDataBlockHeader>();
                // TODO will this screw up on platforms of different endianness?
                header.data_size_bytes = header.data_size_bytes.to_be();
                let data = &jpeg_raw[offset .. offset + header.data_size_bytes as usize];
                data_blocks.push((header, data));
                offset += header.data_size_bytes as usize;
            }

            for (header, data) in data_blocks {
                println!("==========");
                println!("marker: {:#02X}", header.tag_marker);
                println!("record #: {:#02X}", header.record_number);
                println!("data set #: {:#02X}", header.data_set_number);
                // NOTE could also just copy the value to a variable then use the variable.
                // Would avoid having to cast to a pointer and read_aligned.
                println!("data size: {}", unsafe { (&raw const header.data_size_bytes).read_unaligned() });
                print!("data: ");
                use std::io::prelude::*;
                let mut out = std::io::stdout();
                _ = out.write_all(data);
                _ = out.flush();
                println!();
                println!();
            }

            // remember that the whole app 13 segment payload is padded to be an even size
            if jpeg_raw[offset] == 0 { offset += 1; }
        } else {
            offset += marker_payload_size as usize;
        }

        if offset >= jpeg_raw.len() { break; }
    }
}
