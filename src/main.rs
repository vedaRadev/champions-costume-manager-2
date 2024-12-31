// TODO check out Bytes instead of Vec?
// https://users.rust-lang.org/t/can-someone-explain-how-bytes-is-better-than-vec/105934
// https://doc.rust-lang.org/beta/std/io/struct.Bytes.html

use byteorder::{ ByteOrder, BigEndian };

const JPEG_MARKER_SOI: u16 = 0xFFD8;
const JPEG_MARKER_EOI: u16 = 0xFFD9;
const JPEG_MARKER_SOF0: u16 = 0xFFC0;
const JPEG_MARKER_SOF2: u16 = 0xFFC2;
const JPEG_MARKER_DHT: u16 = 0xFFC4;
const JPEG_MARKER_DQT: u16 = 0xFFDB;
const JPEG_MARKER_DRI: u16 = 0xFFDD;
const JPEG_MARKER_SOS: u16 = 0xFFDA;
const JPEG_MARKER_COM: u16 = 0xFFFE;
const JPEG_MARKER_RST_RANGE_BEGIN: u16 = 0xFFD0;
const JPEG_MARKER_RST_RANGE_END: u16 = 0xFFD7;
const JPEG_MARKER_APP_RANGE_BEGIN: u16 = 0xFFE0;
const JPEG_MARKER_APP_RANGE_END: u16 = 0xFFEF;

// // TODO Does this need to be repr(packed) for easy writing?
// // If it's not packed we can't just write the bytes straight to the file because of added padding.
// struct IptcDataBlock {
//     header: IptcDataBlockHeader,
//     data: *const u8,
// }

fn main() {
    let test_file = std::env::var("TEST_FILE").expect("test file environment variable not found");
    let jpeg_raw = std::fs::read(test_file).expect("failed to read");
    let mut offset = 0;
    loop {
        let marker = BigEndian::read_u16(&jpeg_raw[offset..]);
        offset += std::mem::size_of::<u16>();

        let marker_payload_size = match marker {
            JPEG_MARKER_SOI
            | JPEG_MARKER_EOI
            | JPEG_MARKER_RST_RANGE_BEGIN ..= JPEG_MARKER_RST_RANGE_END
            => { 0 }

            JPEG_MARKER_SOF0
            | JPEG_MARKER_SOF2
            | JPEG_MARKER_DHT
            | JPEG_MARKER_DQT
            | JPEG_MARKER_DRI
            | JPEG_MARKER_SOS
            | JPEG_MARKER_COM
            | JPEG_MARKER_APP_RANGE_BEGIN ..= JPEG_MARKER_APP_RANGE_END
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
            || matches!(BigEndian::read_u16(&jpeg_raw[offset..]), 0xFF00 | JPEG_MARKER_RST_RANGE_BEGIN ..= JPEG_MARKER_RST_RANGE_END)
            {
                offset += 1;
            }
        } else if matches!(marker, JPEG_MARKER_APP_RANGE_BEGIN ..= JPEG_MARKER_APP_RANGE_END) && marker & 0xF == 13 {
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
