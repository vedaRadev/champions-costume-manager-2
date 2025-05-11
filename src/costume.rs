use crate::jpeg;

const COSTUME_HASH_ASCII_MAP:  [u16; 256] = [
    0xBCD1, 0xBB65, 0x42C2, 0xDFFE, 0x9666, 0x431B, 0x8504, 0xEB46,
    0x6379, 0xD460, 0xCF14, 0x53CF, 0xDB51, 0xDB08, 0x12C8, 0xF602,
    0xE766, 0x2394, 0x250D, 0xDCBB, 0xA678, 0x02AF, 0xA5C6, 0x7EA6,
    0xB645, 0xCB4D, 0xC44B, 0xE5DC, 0x9FE6, 0x5B5C, 0x35F5, 0x701A,
    0x220F, 0x6C38, 0x1A56, 0x4CA3, 0xFFC6, 0xB152, 0x8D61, 0x7A58,
    0x9025, 0x8B3D, 0xBF0F, 0x95A3, 0xE5F4, 0xC127, 0x3BED, 0x320B,
    0xB7F3, 0x6054, 0x333C, 0xD383, 0x8154, 0x5242, 0x4E0D, 0x0A94,
    0x7028, 0x8689, 0x3A22, 0x0980, 0x1847, 0xB0F1, 0x9B5C, 0x4176,
    0xB858, 0xD542, 0x1F6C, 0x2497, 0x6A5A, 0x9FA9, 0x8C5A, 0x7743,
    0xA8A9, 0x9A02, 0x4918, 0x438C, 0xC388, 0x9E2B, 0x4CAD, 0x01B6,
    0xAB19, 0xF777, 0x365F, 0x1EB2, 0x091E, 0x7BF8, 0x7A8E, 0x5227,
    0xEAB1, 0x2074, 0x4523, 0xE781, 0x01A3, 0x163D, 0x3B2E, 0x287D,
    0x5E7F, 0xA063, 0xB134, 0x8FAE, 0x5E8E, 0xB7B7, 0x4548, 0x1F5A,
    0xFA56, 0x7A24, 0x900F, 0x42DC, 0xCC69, 0x02A0, 0x0B22, 0xDB31,
    0x71FE, 0x0C7D, 0x1732, 0x1159, 0xCB09, 0xE1D2, 0x1351, 0x52E9,
    0xF536, 0x5A4F, 0xC316, 0x6BF9, 0x8994, 0xB774, 0x5F3E, 0xF6D6,
    0x3A61, 0xF82C, 0xCC22, 0x9D06, 0x299C, 0x09E5, 0x1EEC, 0x514F,
    0x8D53, 0xA650, 0x5C6E, 0xC577, 0x7958, 0x71AC, 0x8916, 0x9B4F,
    0x2C09, 0x5211, 0xF6D8, 0xCAAA, 0xF7EF, 0x287F, 0x7A94, 0xAB49,
    0xFA2C, 0x7222, 0xE457, 0xD71A, 0x00C3, 0x1A76, 0xE98C, 0xC037,
    0x8208, 0x5C2D, 0xDFDA, 0xE5F5, 0x0B45, 0x15CE, 0x8A7E, 0xFCAD,
    0xAA2D, 0x4B5C, 0xD42E, 0xB251, 0x907E, 0x9A47, 0xC9A6, 0xD93F,
    0x085E, 0x35CE, 0xA153, 0x7E7B, 0x9F0B, 0x25AA, 0x5D9F, 0xC04D,
    0x8A0E, 0x2875, 0x4A1C, 0x295F, 0x1393, 0xF760, 0x9178, 0x0F5B,
    0xFA7D, 0x83B4, 0x2082, 0x721D, 0x6462, 0x0368, 0x67E2, 0x8624,
    0x194D, 0x22F6, 0x78FB, 0x6791, 0xB238, 0xB332, 0x7276, 0xF272,
    0x47EC, 0x4504, 0xA961, 0x9FC8, 0x3FDC, 0xB413, 0x007A, 0x0806,
    0x7458, 0x95C6, 0xCCAA, 0x18D6, 0xE2AE, 0x1B06, 0xF3F6, 0x5050,
    0xC8E8, 0xF4AC, 0xC04C, 0xF41C, 0x992F, 0xAE44, 0x5F1B, 0x1113,
    0x1738, 0xD9A8, 0x19EA, 0x2D33, 0x9698, 0x2FE9, 0x323F, 0xCDE2,
    0x6D71, 0xE37D, 0xB697, 0x2C4F, 0x4373, 0x9102, 0x075D, 0x8E25,
    0x1672, 0xEC28, 0x6ACB, 0x86CC, 0x186E, 0x9414, 0xD674, 0xD1A5,
];

pub fn generate_costume_hash(costume_spec: &str) -> String {
    let mut upper_bits = std::num::Wrapping(0u16);
    let mut lower_bits = std::num::Wrapping(0u16);
    for byte in costume_spec.bytes() {
        lower_bits += std::num::Wrapping(COSTUME_HASH_ASCII_MAP[byte as usize]);
        upper_bits += lower_bits;
    }

    format!("7799{}\0", ((upper_bits.0 as i32) << 16) | (lower_bits.0 as i32))
}

pub fn get_in_game_display_name(account_name: &str, character_name: &str, timestamp: Option<i64>) -> String {
    let maybe_datetime_string = timestamp.and_then(|j2000_timestamp| {
        const JAN_1_2000_UNIX_TIME: i64 = 946684800;
        let unix_timestamp = JAN_1_2000_UNIX_TIME + j2000_timestamp;
        chrono::DateTime::from_timestamp(unix_timestamp, 0)
            .map(|utc_datetime| utc_datetime.format("%Y-%m-%d %H:%M:%S").to_string())
    });

    if let Some(datetime_string) = maybe_datetime_string {
        format!("{}{} {}", account_name, character_name, datetime_string)
    } else {
        format!("{}{}", account_name, character_name)
    }
}

pub fn get_file_name(save_name: &str, timestamp: Option<i64>) -> String {
    if let Some(j2000_timestamp) = timestamp {
        if save_name.is_empty() {
            format!("Costume_{}.jpg", j2000_timestamp)
        } else {
            format!("Costume_{}_{}.jpg", save_name, j2000_timestamp)
        }
    } else {
        format!("Costume_{}.jpg", save_name)
    }
}

pub fn is_valid_costume_file_name(file_path: &std::path::Path) -> bool {
    let Some(extension) = file_path.extension().and_then(|s| s.to_str()) else { return false };
    let Some(file_stem) = file_path.file_stem().and_then(|s| s.to_str()) else { return false };
    file_stem.starts_with("Costume_") && extension.eq_ignore_ascii_case("jpg")
}

const ACCOUNT_NAME_INDEX: usize = 0;
const CHARACTER_NAME_INDEX: usize = 1;
const COSTUME_HASH_INDEX: usize = 2;
const COSTUME_SPEC_INDEX: usize = 0;

static EXPECTED_APP13_SEGMENT_ID: &str = "Photoshop 3.0\0";
static EXPECTED_APP13_RESOURCE_TYPE: &[u8; 4] = b"8BIM";
const EXPECTED_APP13_RESOURCE_ID: u16 = 0x0404;
static EXPECTED_APP13_RESOURCE_NAME: &str = "\0\0";

#[derive(Debug)]
pub enum CostumeParseError {
    #[allow(dead_code)]
    InvalidFileName,
    InvalidApp13SegmentCount { count: usize },
    InvalidApp13SegmentId { actual: Box<[u8]> },
    InvalidApp13ResourceType { actual: u32 },
    InvalidApp13ResourceId { actual: u16 },
    InvalidApp13ResourceName { actual: Box<[u8]> },
    JpegParseError(jpeg::ParseError),
}

impl std::error::Error for CostumeParseError {}

impl std::fmt::Display for CostumeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidFileName => write!(f, "Invalid file name"),
            Self::InvalidApp13SegmentCount { count } => write!(f, "Invalid App13 segment count: expected 1 but found {count}"),
            Self::InvalidApp13SegmentId { actual } => write!(
                f,
                "Invalid App13 segment id: expected {EXPECTED_APP13_SEGMENT_ID:?} but found {:?}",
                unsafe { std::str::from_utf8_unchecked(actual) }
            ),
            Self::InvalidApp13ResourceType { actual } => write!(
                f,
                "Invalid App13 resource type: expected {:?} but found {:?}",
                unsafe { std::str::from_utf8_unchecked(EXPECTED_APP13_RESOURCE_TYPE) },
                unsafe { std::str::from_utf8_unchecked(&actual.to_be_bytes()) },
            ),
            Self::InvalidApp13ResourceId { actual } => write!(f, "Invalid App13 resource id: expected {EXPECTED_APP13_RESOURCE_ID:#06X} but found {actual:#06X}"),
            Self::InvalidApp13ResourceName { actual } => write!(
                f,
                "Invalid App13 resource name: expected {EXPECTED_APP13_RESOURCE_NAME:?} but found {:?}",
                unsafe { std::str::from_utf8_unchecked(actual) },
            ),
            Self::JpegParseError(parse_error) => write!(f, "Failed to parse jpeg: {parse_error}"),
        }
    }
}

pub struct CostumeSave(pub jpeg::Jpeg);

// TODO can we coalesce UpdateCostumeMetadata and CostumeMetadata?
// e.g. account_name: Option<Cow<'a, str>>
// That might feel terrible to work with.
pub struct CostumeMetadata<'a> {
    pub account_name: &'a str,
    pub character_name: &'a str,
    pub hash: &'a str,
    pub spec: &'a str,
}

#[derive(Default)]
pub struct UpdateCostumeMetadata {
    pub account_name: Option<String>,
    pub character_name: Option<String>,
    pub hash: Option<String>,
    pub spec: Option<String>,
}

impl CostumeSave {
    pub fn parse(bytes: &[u8]) -> Result<Self, CostumeParseError> {
        let jpeg = jpeg::Jpeg::parse(bytes).map_err(CostumeParseError::JpegParseError)?;
        let app13_segments = jpeg.get_segment(jpeg::JpegSegmentType::APP13).ok_or(CostumeParseError::InvalidApp13SegmentCount { count: 0 })?;
        if app13_segments.len() != 1 { return Err(CostumeParseError::InvalidApp13SegmentCount { count: app13_segments.len() }); }
        let app13_segment = app13_segments[0].get_payload_as::<jpeg::JpegApp13Payload>();

        // NOTE The following checks might be too restrictive. Additional testing should be done to
        // see if Champions Online will load costume saves whose App13 segment payloads have
        // different values. If it will, then some of these validations should be removed.
        if &*app13_segment.id != EXPECTED_APP13_SEGMENT_ID.as_bytes() {
            return Err(CostumeParseError::InvalidApp13SegmentId { actual: app13_segment.id.clone() });
        }
        if app13_segment.resource_type != u32::from_be_bytes(*EXPECTED_APP13_RESOURCE_TYPE) {
            return Err(CostumeParseError::InvalidApp13ResourceType { actual: app13_segment.resource_type });
        }
        if app13_segment.resource_id != EXPECTED_APP13_RESOURCE_ID {
            return Err(CostumeParseError::InvalidApp13ResourceId { actual: app13_segment.resource_id });
        }
        if &*app13_segment.resource_name != EXPECTED_APP13_RESOURCE_NAME.as_bytes() {
            return Err(CostumeParseError::InvalidApp13ResourceName { actual: app13_segment.resource_name.clone() });
        }

        // TODO validate that the costume hash matches a hash of the spec?

        Ok(Self(jpeg))
    }

    pub fn get_metadata(&self) -> CostumeMetadata {
        let app13_segment = self.0.get_segment(jpeg::JpegSegmentType::APP13).unwrap()[0];
        let app13_payload = app13_segment.get_payload_as::<jpeg::JpegApp13Payload>();
        let caption_datasets = app13_payload.get_datasets(jpeg::APP13_RECORD_APP, jpeg::APP13_RECORD_APP_CAPTION).unwrap();
        let app_object_data_preview_datasets = app13_payload.get_datasets(jpeg::APP13_RECORD_APP, jpeg::APP13_RECORD_APP_OBJECT_DATA_PREVIEW).unwrap();
        // TODO nuke unsafe code in favor of safe variants, return Result to account for failures
        CostumeMetadata {
            account_name: unsafe { std::str::from_utf8_unchecked(&caption_datasets[ACCOUNT_NAME_INDEX].data) },
            character_name: unsafe { std::str::from_utf8_unchecked(&caption_datasets[CHARACTER_NAME_INDEX].data) },
            hash: unsafe { std::str::from_utf8_unchecked(&caption_datasets[COSTUME_HASH_INDEX].data) },
            spec: unsafe { std::str::from_utf8_unchecked(&app_object_data_preview_datasets[COSTUME_SPEC_INDEX].data) },
        }
    }

    pub fn update_metadata(&mut self, updates: UpdateCostumeMetadata) {
        let app13_segment = self.0.get_segment_mut(jpeg::JpegSegmentType::APP13).unwrap().swap_remove(0);
        let app13_payload = app13_segment.get_payload_as_mut::<jpeg::JpegApp13Payload>();

        fn into_boxed_bytes(s: String) -> Box<[u8]> { s.into_bytes().into_boxed_slice() }

        {
            let caption_datasets = app13_payload.get_datasets_mut(jpeg::APP13_RECORD_APP, jpeg::APP13_RECORD_APP_CAPTION).unwrap();
            if let Some(account_name) = updates.account_name.map(into_boxed_bytes) {
                caption_datasets[ACCOUNT_NAME_INDEX].data = account_name;
            }
            if let Some(character_name) = updates.character_name.map(into_boxed_bytes) {
                caption_datasets[CHARACTER_NAME_INDEX].data = character_name;
            }
            if let Some(hash) = updates.hash.map(into_boxed_bytes) {
                caption_datasets[COSTUME_HASH_INDEX].data = hash;
            }
        }

        {
            let app_object_data_preview_datasets = app13_payload.get_datasets_mut(jpeg::APP13_RECORD_APP, jpeg::APP13_RECORD_APP_OBJECT_DATA_PREVIEW).unwrap();
            if let Some(spec) = updates.spec.map(into_boxed_bytes) {
                app_object_data_preview_datasets[0].data = spec;
            }
        }
    }
}
