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

// #[repr(packed)]
// struct JpegDataSetHeader {
//     tag: u8,
//     record_number: u8,
//     data_set_number: u8,
//     data_size_bytes: u16,
// }

fn main() {
    let test_file = std::env::var("TEST_FILE").expect("test file environment variable not found");
    let jpeg_raw = std::fs::read(test_file).expect("failed to read");
    let mut offset = 0;
    loop {
        let marker = BigEndian::read_u16(&jpeg_raw[offset .. offset + 2]);
        offset += std::mem::size_of::<u16>();

        let payload_size = match marker {
            JPEG_MARKER_SOI
            | JPEG_MARKER_EOI
            | JPEG_MARKER_RST_RANGE_BEGIN ..= JPEG_MARKER_RST_RANGE_END => 0,

            JPEG_MARKER_SOF0
            | JPEG_MARKER_SOF2
            | JPEG_MARKER_DHT
            | JPEG_MARKER_DQT
            | JPEG_MARKER_DRI
            | JPEG_MARKER_SOS
            | JPEG_MARKER_COM
            | JPEG_MARKER_APP_RANGE_BEGIN ..= JPEG_MARKER_APP_RANGE_END => {
                BigEndian::read_u16(&jpeg_raw[offset .. offset + 2])
            },

            unknown_marker => panic!("Unknown marker {:#06x} at offset {}", unknown_marker, offset),
        };

        // NOTE: The size of the payload _includes_ the 2 bytes used for reporting the payload size
        println!("Saw marker {:#06x} with size {}", marker, payload_size);
        if payload_size > 0 { offset += payload_size as usize; }

        // https://stackoverflow.com/questions/26715684/parsing-jpeg-sos-marker
        // Once SOS marker is encountered, we have to manually scan through the compressed data until we
        // find the next real marker.
        if marker == JPEG_MARKER_SOS {
            // At this point our offset should be pointing to the start of the image data.
            // Skip all of the image data for now.
            while jpeg_raw[offset] != 0xFF || matches!(BigEndian::read_u16(&jpeg_raw[offset .. offset + 2]), 0xFF00 | JPEG_MARKER_RST_RANGE_BEGIN ..= JPEG_MARKER_RST_RANGE_END) {
                offset += 1;
            }
        }

        if offset >= jpeg_raw.len() { break; }
    }
}
