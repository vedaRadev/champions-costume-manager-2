// TODO Once this is in a more stable state and prototyping is finished, run through and figure out
// how to reduce the amount of cloning we're doing.
//
// TODO Eventually refactor to compress UI actions (e.g. selected file save, delete, etc.). I think
// we should wait until we're further along to actually implement this. If we abstract or DRY up
// too early then when we want to go through and harden the app by adding things like logging on
// filesystem interaction failure, it might just make implementing it more of a nightmare.
//
// TODO Make the API for getting data from the CostumeSaveFile better. It's a bit messy right now
// because every JpegApp13Payload access returns the data we want and an RwLockReadGuard.
// Maybe it would be better to just lock the entire app13 payload and return that, then have some
// methods that will get the individual fields out?
//
// AUDIT Should we use env::current_dir or share the costume dir across threads?
//
// AUDIT If _many_ logs are generated per frame then maybe we shouldn't be sending them through a
// channel. Maybe we should just use an Arc<Mutex<...>>.

mod jpeg;

use jpeg::{
    Jpeg,
    JpegSegmentType,
    JpegApp13Payload,
    APP13_RECORD_APP,
    APP13_RECORD_APP_CAPTION,
    APP13_RECORD_APP_OBJECT_DATA_PREVIEW,
};

use eframe::egui;

use std::{
    fmt,
    num::NonZero,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    io::prelude::*,
    ffi::OsString,
    path::Path,
    env,
    fs,
    sync::{Arc, RwLock, Mutex, atomic, mpsc},
    thread,
    time::{Duration, SystemTime},
};

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

fn generate_costume_hash(costume_spec: &str) -> String {
    let mut upper_bits = std::num::Wrapping(0u16);
    let mut lower_bits = std::num::Wrapping(0u16);
    for byte in costume_spec.bytes() {
        lower_bits += std::num::Wrapping(COSTUME_HASH_ASCII_MAP[byte as usize]);
        upper_bits += lower_bits;
    }

    format!("7799{}\0", ((upper_bits.0 as i32) << 16) | (lower_bits.0 as i32))
}

fn get_in_game_display_name(account_name: &str, character_name: &str, timestamp: Option<i64>) -> String {
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

fn get_file_name(save_name: &str, timestamp: Option<i64>) -> String {
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

// TODO Error checking for things that get strings from raw bytes. Use from_utf8 instead of from_utf8_unchecked.
// TODO Error checking wherever there's an unwrap (unless we're able to guarantee no failure ever)
// TODO Slight refactors to DRY up code (the getters/setters have a lot in common)
const ACCOUNT_NAME_INDEX: usize = 0;
const CHARACTER_NAME_INDEX: usize = 1;
const COSTUME_HASH_INDEX: usize = 2;

#[derive(Debug)]
enum CostumeParseError {
    InvalidFileName,
}

impl std::error::Error for CostumeParseError {}

impl std::fmt::Display for CostumeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidFileName => write!(
                f,
                "Invalid file name"
            ),
        }
    }
}

enum CostumeImage {
    NotLoaded,
    Loading,
    Loaded(egui::TextureHandle),
}

// TODO list:
// * Maybe caching of file name as well (currently requires dynamic creation of string)?
struct CostumeSaveFile {
    jpeg: Jpeg,
    /// The name of the save file as it appears between the "Costume_" prefix and j2000 timestamp
    /// (if included) suffix.
    save_name: String,
    j2000_timestamp: Option<i64>,
    // TODO Anything below this note is a field I have stuck into this struct without much thought.
    // There is probably a better way to organize this data.
    image_texture: CostumeImage,
    image_visible_in_grid: bool,
    image_visible_in_edit: bool,
}

#[allow(dead_code)]
// TODO constructor that returns a result, maybe just take the file path and parse from that.
impl CostumeSaveFile {
    // TODO Don't return Box<dyn Error>, return something more specific
    // TODO save file validation
    // check the filename itself for:
    // - "Costume_" prefix
    // - ".jpg" suffix?
    // check app13 for the following (do testing and see if the game cares about any of this):
    // - segment itself exists
    // - identifier is "Photoshop 3.0\0"
    // - resource type is "8BIM" (as a u32)
    // - resource id is 0x0404
    // - resource name is "\0\0" 
    fn new(file_stem: &str, raw_bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if !file_stem.starts_with("Costume_") { return Err(Box::new(CostumeParseError::InvalidFileName)); }
        let j2000_timestamp = file_stem
            .split('_')
            .last().unwrap()
            .parse::<i64>().ok();
        let costume_jpeg = Jpeg::parse(raw_bytes)?;
        let save_name = {
            let save_name_start = file_stem.find("_").unwrap();
            let save_name_end = if j2000_timestamp.is_some() { file_stem.rfind("_").unwrap() } else { file_stem.len() };
            // Technically the file name can just be "Costume_.jpg"
            if save_name_start == save_name_end {
                String::from("")
            } else {
                file_stem[save_name_start + 1 .. save_name_end].to_owned()
            }
        };
        Ok(CostumeSaveFile {
            jpeg: costume_jpeg,
            save_name,
            j2000_timestamp,
            image_texture: CostumeImage::NotLoaded,
            image_visible_in_grid: false,
            image_visible_in_edit: false,
        })
    }

    fn get_app13_payload(&self) -> &JpegApp13Payload {
        let app13_segment = self.jpeg.get_segment(JpegSegmentType::APP13).unwrap()[0];
        app13_segment.get_payload_as::<JpegApp13Payload>()
    }

    fn get_app13_payload_mut(&mut self) -> &mut JpegApp13Payload {
        let app13_segment = self.jpeg.get_segment_mut(JpegSegmentType::APP13).unwrap().swap_remove(0);
        app13_segment.get_payload_as_mut::<JpegApp13Payload>()
    }

    fn get_account_name(&self) -> &str {
        let app13 = self.get_app13_payload();
        let datasets = app13.get_datasets(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        let result = unsafe { std::str::from_utf8_unchecked(&datasets[ACCOUNT_NAME_INDEX].data) };
        result
    }

    fn set_account_name(&mut self, value: String) {
        let app13 = self.get_app13_payload_mut();
        let datasets = app13.get_datasets_mut(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        datasets[ACCOUNT_NAME_INDEX].data = value.into_bytes().into_boxed_slice();
    }

    fn get_character_name(&self) -> &str {
        let app13 = self.get_app13_payload();
        let datasets = app13.get_datasets(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        let result = unsafe { std::str::from_utf8_unchecked(&datasets[CHARACTER_NAME_INDEX].data) };
        result
    }

    fn set_character_name(&mut self, value: String) {
        let app13 = self.get_app13_payload_mut();
        let datasets = app13.get_datasets_mut(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        datasets[CHARACTER_NAME_INDEX].data = value.into_bytes().into_boxed_slice();
    }

    fn get_costume_hash(&self) -> &str {
        let app13 = self.get_app13_payload();
        let datasets = app13.get_datasets(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        let result = unsafe { std::str::from_utf8_unchecked(&datasets[COSTUME_HASH_INDEX].data) };
        result
    }

    fn set_costume_hash(&mut self, value: String) {
        let app13 = self.get_app13_payload_mut();
        let datasets = app13.get_datasets_mut(APP13_RECORD_APP, APP13_RECORD_APP_CAPTION).unwrap();
        datasets[COSTUME_HASH_INDEX].data = value.into_bytes().into_boxed_slice();
    }

    fn get_costume_spec(&self) -> &str {
        let app13 = self.get_app13_payload();
        let datasets = app13.get_datasets(APP13_RECORD_APP, APP13_RECORD_APP_OBJECT_DATA_PREVIEW).unwrap();
        let result = unsafe { std::str::from_utf8_unchecked(&datasets[0].data) };
        result
    }

    fn set_costume_spec(&mut self, value: String) {
        let app13 = self.get_app13_payload_mut();
        let datasets = app13.get_datasets_mut(APP13_RECORD_APP, APP13_RECORD_APP_OBJECT_DATA_PREVIEW).unwrap();
        datasets[0].data = value.into_bytes().into_boxed_slice();
    }
}

const MAX_LOGS: u16 = 1024;
const MAX_DECODE_THREADS: usize = 8;

/// Critical messages that must be handled as soon as possible
enum UiPriorityMessage {
    /// We have detected that the file system has changed underneath us in some way.
    FileListChangedExternally,
}

enum LogLevel {
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

enum AppThread {
    Ui,
    Scanner,
    Decode(u8),
}

const DECODE_THREAD_NAMES: [&str; MAX_DECODE_THREADS] = [
    "DECODE 0", "DECODE 1", "DECODE 2", "DECODE 3",
    "DECODE 4", "DECODE 5", "DECODE 6", "DECODE 7",
];

impl fmt::Display for AppThread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            Self::Ui => "UI",
            Self::Scanner => "SCANNER",
            Self::Decode(n) if (n as usize) < MAX_DECODE_THREADS => DECODE_THREAD_NAMES[n as usize],
            Self::Decode(_) => "DECODE PAST MAX",
        };
        write!(f, "{}", s)
    }
}

struct Log {
    level: LogLevel,
    timestamp: chrono::NaiveDateTime,
    source: AppThread,
    message: String,
}

impl Log {
    fn new_now(message: String, level: LogLevel, source: AppThread) -> Self {
        let timestamp = chrono::Utc::now().naive_utc();
        let message = format!(
            "[{}] [{}] [{}] - {}",
            level,
            source,
            timestamp.format("%Y-%m-%d %H:%M:%S%.6f"), // microsecond granularity
            message,
        );
        Self {
            message,
            level,
            source,
            timestamp,
        }
    }
}

/// Regular messages whose handling can be delayed for one or many frames
enum UiMessage {
    JpegDecoded { file_name: OsString, texture_handle: egui::TextureHandle },
    Log(Log),
}

#[derive(Default)]
// TODO Maybe most of this could be Cows instead of explicitly owned data?
struct CostumeEdit {
    strip_timestamp: bool,
    timestamp: Option<i64>,
    save_name: String,
    account_name: String,
    character_name: String,
    costume_spec: String,
    costume_hash: String,
}

#[derive(PartialEq, Copy, Clone)]
enum DisplayType { DisplayName, FileName }

#[derive(PartialEq, Copy, Clone)]
enum SortType { Name, CreationTime, ModifiedTime }

// TODO maybe tie the selected costume and costume edit together so they can never get out of sync?
struct App {
    saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>>,
    /// List of logs with the oldest at the end and the newest at the front.
    logs: VecDeque<Log>,

    // So that we can gracefully shut down all threads on exit
    shutdown_flag: Arc<RwLock<atomic::AtomicBool>>,
    support_thread_handles: Vec<thread::JoinHandle<()>>,

    ui_priority_message_rx: mpsc::Receiver<UiPriorityMessage>,
    ui_message_rx: mpsc::Receiver<UiMessage>,
    scanner_tx: mpsc::Sender<SystemTime>,
    decode_job_tx: mpsc::Sender<OsString>,

    file_exists_warning_modal_open: bool,
    show_images_in_selection_list: bool,
    costume_spec_edit_open: bool,
    confirm_edit_spec: bool,
    sorted_saves: Vec<OsString>,
    /// Values are indices into self.sorted_saves.
    selected_costumes: HashSet<usize>,
    selection_range_pivot: usize,
    display_type: DisplayType,
    sort_type: SortType,
    costume_edit: Option<CostumeEdit>,
}

struct AppArgs {
    saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>>,
    shutdown_flag: Arc<RwLock<atomic::AtomicBool>>,
    support_thread_handles: Vec<thread::JoinHandle<()>>,
    ui_priority_message_rx: mpsc::Receiver<UiPriorityMessage>,
    ui_message_rx: mpsc::Receiver<UiMessage>,
    scanner_tx: mpsc::Sender<SystemTime>,
    decode_job_tx: mpsc::Sender<OsString>,
}

impl App {
    fn new(
        _cc: &eframe::CreationContext,
        AppArgs {
            saves,
            shutdown_flag,
            support_thread_handles,
            ui_priority_message_rx,
            ui_message_rx,
            scanner_tx,
            decode_job_tx,
        }: AppArgs,
    ) -> Self
    {
        Self {
            saves,
            logs: VecDeque::with_capacity(MAX_LOGS as usize),

            shutdown_flag,
            support_thread_handles,

            ui_priority_message_rx,
            ui_message_rx,
            scanner_tx,
            decode_job_tx,

            file_exists_warning_modal_open: false,
            show_images_in_selection_list: false,
            costume_spec_edit_open: false,
            confirm_edit_spec: false,
            sorted_saves: vec![],
            selected_costumes: HashSet::new(),
            selection_range_pivot: 0,
            display_type: DisplayType::DisplayName,
            sort_type: SortType::Name,
            costume_edit: None,
        }
    }

    // TODO Should we just clear our selected costumes in here? I think basically every time we
    // sort we do that.
    // FIXME We might need to support case-insensitive sorting on non-ascii characters, in which
    // case we'll need to use to_lowercase(). Might require more cloning than is necessary, so
    // maybe find a way to do that efficiently.
    fn sort_saves(sort_type: SortType, display_type: DisplayType, keys_to_sort: &mut [OsString], locked_saves: &std::sync::MutexGuard<HashMap<OsString, CostumeSaveFile>>) {
        match sort_type {
            SortType::Name => {
                match display_type {
                    DisplayType::DisplayName => {
                        keys_to_sort.sort_by_key(|k| {
                            let save = &locked_saves[k];
                            let account_name = save.get_account_name();
                            let character_name = save.get_character_name();
                            get_in_game_display_name(account_name, character_name, save.j2000_timestamp).to_ascii_lowercase()
                        });
                    },

                    DisplayType::FileName => {
                        keys_to_sort.sort_by_key(|k| k.to_ascii_lowercase());
                    },
                }
            },

            SortType::CreationTime => {
                keys_to_sort.sort_by_key(|k| {
                    let metadata = fs::metadata(k).unwrap();
                    std::cmp::Reverse(metadata.created().unwrap())
                });
            },

            SortType::ModifiedTime => {
                keys_to_sort.sort_by_key(|k| {
                    let metadata = fs::metadata(k).unwrap();
                    std::cmp::Reverse(metadata.modified().unwrap())
                });
            }
        };
    }
}

impl eframe::App for App {
    // Gracefully shut down all our supporting threads.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown_flag.write().unwrap().store(true, atomic::Ordering::Release);
        // NOTE(RA): I think draining the vector like this is okay for now since we shouldn't
        // update again after handling this exit event.
        self.support_thread_handles.drain(..).for_each(|thread_handle| {
            thread_handle.join().unwrap();
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut saves = self.saves.lock().unwrap();
        let current_modifiers = ctx.input(|input| input.modifiers);

        while let Ok(priority_message) = self.ui_priority_message_rx.try_recv() {
            match priority_message {
                UiPriorityMessage::FileListChangedExternally => {
                    // NOTE(RA): For now we're just going to reset selections whenever the file
                    // list changes. Might change this later. We used to only do this whenever the
                    // file(s) the user was viewing were removed from the file system.
                    self.selected_costumes.clear();
                    self.costume_edit = None;
                    self.sorted_saves = saves.keys().cloned().collect();
                    Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &saves);
                },
            }
        }

        const MAX_MESSAGES_PER_FRAME: usize = 32;
        for _ in 0..MAX_MESSAGES_PER_FRAME {
            let message = self.ui_message_rx.try_recv();
            if message.is_err() { break; }
            match message.unwrap() {
                UiMessage::JpegDecoded { file_name, texture_handle } => {
                    if saves.contains_key(&file_name) {
                        saves.get_mut(&file_name).unwrap().image_texture = CostumeImage::Loaded(texture_handle);
                    }
                },

                // NOTE: If we have many logs generated per frame then maybe this should be a
                // priority message so we can process more than MAX_MESSAGE_PER_FRAME of them.
                UiMessage::Log(log) => {
                    // TODO add to logs
                    println!("{}", log.message);
                },
            }
        }

        if self.file_exists_warning_modal_open {
            egui::Modal::new(egui::Id::new("File Exists Warning")).show(ctx, |ui| {
                ui.label("A file with the same name already exists!");
                if ui.button("Ok").clicked() {
                    self.file_exists_warning_modal_open = false;
                }
            });
        }

        if self.costume_spec_edit_open {
            assert_eq!(self.selected_costumes.len(), 1);

            // TODO figure out how to make this modal wider
            let modal = egui::Modal::new(egui::Id::new("Costume Spec Edit"));
            modal.show(ctx, |ui| {
                ui.label("EDITING THE COSTUME SPEC IS AN EXPERIMENTAL AND DANGEROUS FEATURE! BEWARE!");
                ui.label("Incorrectly modifying the costume spec can corrupt your save and make it unloadable in-game! Make a backup!");
                ui.label("Spec changes will be saved upon closing this modal and saving the costume.");
                if ui.checkbox(&mut self.confirm_edit_spec, "I have read and understand the above warnings and want to proceed (toggle off to revert all changes)").changed() && !self.confirm_edit_spec {
                    // Reset to original in case user made changes but then changed their mind.
                    // FIXME unnecessary clone if the user hasn't changed the spec/hash and is just
                    // toggling the checkbox on/off for some reason.
                    let save_idx = self.selected_costumes.iter().last().unwrap();
                    let save = &saves[&self.sorted_saves[*save_idx]];
                    let costume_spec = save.get_costume_spec();
                    let costume_hash = save.get_costume_hash();
                    self.costume_edit.as_mut().unwrap().costume_spec = costume_spec.to_owned();
                    self.costume_edit.as_mut().unwrap().costume_hash = costume_hash.to_owned();
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Hash:");
                    ui.label(&self.costume_edit.as_ref().unwrap().costume_hash);
                });

                let scroll_area = egui::ScrollArea::vertical().max_height(500.0);
                scroll_area.show(ui, |ui| {
                    let spec_editor = ui.add_enabled(
                        self.confirm_edit_spec,
                        egui::TextEdit::multiline(&mut self.costume_edit.as_mut().unwrap().costume_spec)
                            .code_editor()
                            .desired_rows(12)
                            .desired_width(f32::INFINITY)
                    );

                    if spec_editor.changed() {
                        self.costume_edit.as_mut().unwrap().costume_hash = generate_costume_hash(&self.costume_edit.as_ref().unwrap().costume_spec);
                    }
                });

                ui.centered_and_justified(|ui| {
                    let close_text = if self.confirm_edit_spec { "Save and Close" } else { "Cancel and Close" };
                    if ui.button(close_text).clicked() {
                        self.costume_spec_edit_open = false;
                    }
                });
            });
        }

        egui::SidePanel::right("details_display").show(ctx, |ui| {
            // NOTE: For now we're just assuming that the selected costume and the costume edit
            // data are properly tied together. Maybe we should tie these together better so that
            // they can't possibly get out of sync.
            // TODO delete the allowance of comparison change and follow clippy recommendation.
            #[allow(clippy::comparison_chain)]
            if self.selected_costumes.is_empty() {
                ui.label("Select a save to view details");
            } else {
                if self.selected_costumes.len() > 1 {
                    ui.label(format!("{} selected items", self.selected_costumes.len()));
                } else if self.selected_costumes.len() == 1 {
                    // FIXME probably ultimately unnecessary clone
                    let costume_file_name = &self.sorted_saves[*self.selected_costumes.iter().last().unwrap()].clone();
                    let costume = saves.get_mut(costume_file_name).unwrap();
                    let costume_edit = self.costume_edit.as_mut().unwrap();

                    // TODO there's another place in the image grid where we do something very
                    // similar to this. Maybe find a way to pull this logic out into a function?
                    if let CostumeImage::Loaded(texture) = &costume.image_texture {
                        let image = egui::Image::new(texture)
                            .maintain_aspect_ratio(true)
                            .max_height(500.0);
                        ui.add(image);
                    } else {
                        if matches!(costume.image_texture, CostumeImage::NotLoaded) {
                            // TODO log send error
                            _ = self.decode_job_tx.send(costume_file_name.clone());
                            costume.image_texture = CostumeImage::Loading;
                        }
                        ui.label("loading image...");
                    }

                    // FIXME we probably do not want to construct the file name every frame. Maybe
                    // cache it in the CostumeEdit struct itself?
                    ui.horizontal(|ui| {
                        ui.label("File Name:");
                        ui.label(get_file_name(&costume_edit.save_name, costume_edit.timestamp));
                    });
                    // FIXME again, don't want to construct this every frame
                    ui.horizontal(|ui| {
                        ui.label("In-Game Display:");
                        ui.label(get_in_game_display_name(&costume_edit.account_name, &costume_edit.character_name, costume_edit.timestamp));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Save Name:");
                        ui.text_edit_singleline(&mut costume_edit.save_name);
                    });
                    if costume.j2000_timestamp.is_some() {
                        ui.checkbox(&mut costume_edit.strip_timestamp, "Strip Timestamp");
                        if costume_edit.strip_timestamp {
                            costume_edit.timestamp = None;
                        } else {
                            costume_edit.timestamp = costume.j2000_timestamp;
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Account Name:");
                        ui.text_edit_singleline(&mut costume_edit.account_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Character Name:");
                        ui.text_edit_singleline(&mut costume_edit.character_name);
                    });
                    if ui.button("Edit Spec").clicked() {
                        self.costume_spec_edit_open = true;
                    }

                    // TODO disable button if nothing was changed?
                    // TODO if only file name was changed maybe only rename the file via OS?
                    // FIXME Logging, not crashing!
                    if ui.button("Save").clicked() {
                        let old_file_name = costume_file_name;
                        let new_file_name = get_file_name(&costume_edit.save_name, costume_edit.timestamp);
                        let temp_file_name = OsString::from(format!("{new_file_name}.CCM_TEMP"));
                        let new_file_name = OsString::from(new_file_name);
                        let file_name_changed = *new_file_name != *old_file_name;

                        // FIXME There is potentially a massive, terrible bug here on Windows where
                        // we're NOT catching here due to Windows' case insensitivity! If we've
                        // changed the file name but saves contains the same name with a different
                        // casing, we WON'T catch it!
                        if file_name_changed && saves.contains_key(&new_file_name) {
                            self.file_exists_warning_modal_open = true;
                        } else {
                            let costume = saves.get_mut(costume_file_name).unwrap();
                            let mut temp_file = fs::File::create(&temp_file_name).unwrap_or_else(|err| {
                                eprintln!("Failed to open {:?} for writing: {err}", temp_file_name);
                                std::process::exit(1);
                            });

                            costume.set_account_name(costume_edit.account_name.clone());
                            costume.set_character_name(costume_edit.character_name.clone());
                            costume.set_costume_spec(costume_edit.costume_spec.clone());
                            costume.set_costume_hash(costume_edit.costume_hash.clone());
                            let serialized = costume.jpeg.serialize();

                            if let Err(err) = temp_file.write_all(&serialized) {
                                eprintln!("failed to write to file {:?}: {err}", temp_file_name);
                                std::process::exit(1);
                            } else {
                                println!("wrote to {:?}", temp_file_name);
                            }

                            if file_name_changed {
                                costume.save_name = costume_edit.save_name.clone();
                                costume.j2000_timestamp = costume_edit.timestamp;
                            }

                            // Need to copy old creation time to new file
                            #[cfg(windows)]
                            {
                                use std::os::windows::fs::FileTimesExt;
                                let old_file = fs::File::open(old_file_name).unwrap_or_else(|err| {
                                    eprintln!("failed to open original file {:?} for reading: {err}", old_file_name);
                                    std::process::exit(1);
                                });
                                let old_metadata = old_file.metadata().unwrap_or_else(|err| {
                                    eprintln!("failed to get metadata for original file {:?}: {err}", old_file_name);
                                    std::process::exit(1);
                                });
                                let new_metadata = temp_file.metadata().unwrap_or_else(|err| {
                                    eprintln!("failed to get metadata for new file {new_file_name:?}: {err}");
                                    std::process::exit(1);
                                });
                                // SAFETY: This section is conditionally compiled for windows so
                                // setting/getting the file creation time should not error.
                                let times = fs::FileTimes::new()
                                    .set_created(old_metadata.created().unwrap())
                                    .set_accessed(new_metadata.accessed().unwrap())
                                    .set_modified(new_metadata.modified().unwrap());
                                if let Err(err) = temp_file.set_times(times) {
                                    eprintln!("failed to update filetimes for {temp_file_name:?}: {err}");
                                    std::process::exit(1);
                                }
                            }

                            if let Err(err) = fs::remove_file(old_file_name) {
                                eprintln!("failed to remove original file {old_file_name:?}: {err}");
                                std::process::exit(1);
                            }

                            if let Err(err) = fs::rename(temp_file_name, &new_file_name) {
                                eprintln!("failed to rename temp file: {err}");
                                std::process::exit(1);
                            }

                            // FIXME really lazy and inefficient. I don't think we can know where the new
                            // save name will be after sorting (maybe we actually can) but we can probably
                            // pass along the index of the old save with this event. Would eliminate an
                            // entire scan through the sorted_saves array.
                            let (old_index, _) = self.sorted_saves.iter().enumerate().find(|(_, save)| *save == old_file_name).unwrap();
                            assert!(self.selected_costumes.remove(&old_index));

                            let save = saves.remove(old_file_name).unwrap();
                            saves.insert(new_file_name.clone(), save);
                            self.sorted_saves = saves.keys().cloned().collect();
                            Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &saves);

                            let (new_index, _) = self.sorted_saves.iter().enumerate().find(|(_, save)| **save == new_file_name).unwrap();
                            self.selected_costumes.insert(new_index);

                            // TODO find a way to compress this code since we do the exact same thing when
                            // deleting files.

                            // Signal to the scanning thread that we initiated the file system change.
                            // This avoids cases where we update the file system, react to the update,
                            // then the scanner sees that something was changed and gives us ANOTHER
                            // notification that the file system was changed.
                            let current_dir = env::current_dir().unwrap();
                            let last_modified_time = fs::metadata(&current_dir).unwrap().modified().unwrap();
                            // TODO log failure
                            let _ = self.scanner_tx.send(last_modified_time);
                        }
                    }
                }

                if ui.button("Delete").clicked() {
                    // TODO show delete confirmation popup
                    for selected_idx in self.selected_costumes.iter() {
                        let costume_file_name = &self.sorted_saves[*selected_idx];
                        fs::remove_file(costume_file_name).expect("Failed to delete file");
                        saves.remove(costume_file_name);
                    }
                    self.sorted_saves = saves.keys().cloned().collect();
                    Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &saves);
                    self.selected_costumes.clear();
                    // TODO find a way to compress this code since we do the exact same thing when
                    // saving files.

                    // Signal to the scanning thread that we initiated the file system change.
                    // This avoids cases where we update the file system, react to the update,
                    // then the scanner sees that something was changed and gives us ANOTHER
                    // notification that the file system was changed.
                    let current_dir = env::current_dir().unwrap();
                    let last_modified_time = fs::metadata(&current_dir).unwrap().modified().unwrap();
                    // TODO log failure
                    let _ = self.scanner_tx.send(last_modified_time);
                }
            }

        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let prev_display_type = self.display_type;
            let prev_sort_type = self.sort_type;
            ui.horizontal(|ui| {
                ui.label("Display:");
                ui.selectable_value(&mut self.display_type, DisplayType::DisplayName, "Display Name");
                ui.selectable_value(&mut self.display_type, DisplayType::FileName, "File Name");
                ui.checkbox(&mut self.show_images_in_selection_list, "Show Images");
            });
            ui.horizontal(|ui| {
                ui.label("Sort:");
                ui.selectable_value(&mut self.sort_type, SortType::Name, "Name");
                ui.selectable_value(&mut self.sort_type, SortType::CreationTime, "Creation Time");
                ui.selectable_value(&mut self.sort_type, SortType::ModifiedTime, "Modified Time");
            });
            let sort_needed = self.sort_type != prev_sort_type || self.sort_type == SortType::Name && self.display_type != prev_display_type;
            if sort_needed {
                self.selected_costumes.clear();
                self.selection_range_pivot = 0;
                Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &saves);
            }

            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let available_width = ui.available_width();
                ui.set_width(available_width);
                const IMAGE_ASPECT_RATIO: f32 = 3.0 / 4.0;
                const IMAGE_WIDTH: f32 = 150.0;
                const IMAGE_HEIGHT: f32 = IMAGE_WIDTH / IMAGE_ASPECT_RATIO;
                const IMAGE_SIZE: [f32; 2] = [IMAGE_WIDTH, IMAGE_HEIGHT];
                const LABEL_HEIGHT: f32 = 30.0;
                const FRAME_INNER_MARGIN: f32 = 4.0;
                /// Excludes the inner and outer margins of the frame and the inner widget spacing
                const FRAME_SIZE: egui::Vec2 = egui::Vec2 {
                    x: IMAGE_WIDTH,
                    y: IMAGE_HEIGHT + LABEL_HEIGHT,
                };
                const ITEM_SPACING: egui::Vec2 = egui::Vec2 { x: 4.0, y: 4.0 };

                let num_cols = if self.show_images_in_selection_list {
                    (available_width / (FRAME_SIZE.x + (FRAME_INNER_MARGIN * 2.0) + ITEM_SPACING.x)).floor() as usize
                } else {
                    1
                };

                let grid = egui::Grid::new("selection_grid").spacing(ITEM_SPACING);
                let scroll_area_clip_rect = ui.clip_rect();
                grid.show(ui, |ui| {
                    for (idx, save_file_name) in self.sorted_saves.iter().enumerate() {
                        let save = saves.get_mut(save_file_name).unwrap();
                        let is_selected = self.selected_costumes.contains(&idx);
                        let display_name = match self.display_type {
                            DisplayType::DisplayName => {
                                let account_name = save.get_account_name();
                                let character_name = save.get_character_name();
                                get_in_game_display_name(account_name, character_name, save.j2000_timestamp)
                            },
                            DisplayType::FileName => get_file_name(&save.save_name, save.j2000_timestamp),
                        };

                        let selectable_costume_item = if self.show_images_in_selection_list {
                            // Create a selectable button that contains an image and some text beneath it.
                            let custom_button = ui.scope_builder(
                                egui::UiBuilder::new().sense(egui::Sense::click() | egui::Sense::hover()),
                                |ui| {
                                    let frame = egui::Frame::canvas(ui.style())
                                        .stroke(egui::Stroke::NONE)
                                        .fill(ui.style().visuals.window_fill)
                                        .inner_margin(FRAME_INNER_MARGIN);

                                    let mut prepped = frame.begin(ui);

                                    let is_hovered = prepped.content_ui.response().hovered();
                                    if is_hovered {
                                        prepped.frame = prepped.frame.stroke(egui::Stroke::new(2.0, prepped.content_ui.style().visuals.widgets.hovered.bg_stroke.color));
                                        prepped.frame = prepped.frame.fill(prepped.content_ui.style().visuals.widgets.hovered.bg_fill);
                                    }
                                    if is_selected {
                                        prepped.frame = prepped.frame.fill(prepped.content_ui.style().visuals.selection.bg_fill);
                                    }

                                    prepped.content_ui.set_max_width(FRAME_SIZE.x);
                                    prepped.content_ui.set_min_size(FRAME_SIZE);
                                    prepped.content_ui.vertical(|ui| {
                                        // TODO there's another place in the edit panel where we do something very
                                        // similar to this. Maybe find a way to pull this logic out into a function?
                                        if let CostumeImage::Loaded(texture) = &save.image_texture {
                                            ui.add(egui::Image::new(texture).fit_to_exact_size(IMAGE_SIZE.into()));
                                        } else {
                                            ui.label("loading image...");
                                        }

                                        ui.horizontal_wrapped(|ui| {
                                            let mut label_text = egui::RichText::new(display_name);
                                            if is_hovered {
                                                label_text = label_text.color(ui.style().visuals.widgets.hovered.text_color());
                                            }
                                            if is_selected {
                                                label_text = label_text.color(ui.style().visuals.selection.stroke.color);
                                            }
                                            ui.add(egui::Label::new(label_text).selectable(false));
                                        });
                                    });

                                    prepped.end(ui);
                                }
                            ).response;

                            if ((idx + 1) % num_cols) == 0 {
                                ui.end_row();
                            }

                            save.image_visible_in_grid = scroll_area_clip_rect.intersects(custom_button.rect);
                            if save.image_visible_in_grid && matches!(save.image_texture, CostumeImage::NotLoaded) {
                                // TODO log send error
                                _ = self.decode_job_tx.send(save_file_name.clone());
                                save.image_texture = CostumeImage::Loading;
                            }

                            custom_button
                        } else {
                            save.image_visible_in_grid = false;
                            let selectable_label = ui.selectable_label(is_selected, display_name);
                            ui.end_row();
                            selectable_label
                        };

                        if selectable_costume_item.clicked() {
                            if self.selected_costumes.is_empty() {
                                self.selected_costumes.insert(idx);
                                self.selection_range_pivot = idx;
                            } else if current_modifiers.shift {
                                // NOTE: If for some reason clearing the list and re-adding the
                                // range every time causes performance issues, change this back to
                                // the prior logic where we would check the lo/hi vals against the
                                // pivot and only add/remove items that were outside the
                                // already-selected range (may need to switch back to BTreeSet).
                                self.selected_costumes.clear();
                                match self.selection_range_pivot.cmp(&idx) {
                                    Ordering::Greater => for i in idx ..= self.selection_range_pivot {
                                        self.selected_costumes.insert(i);
                                    },
                                    Ordering::Less => for i in self.selection_range_pivot ..= idx {
                                        self.selected_costumes.insert(i);
                                    },
                                    Ordering::Equal => {
                                        self.selected_costumes.insert(idx);
                                    },
                                }
                            } else if current_modifiers.ctrl {
                                self.selection_range_pivot = idx;
                                if is_selected {
                                    self.selected_costumes.remove(&idx);
                                } else {
                                    self.selected_costumes.insert(idx);
                                }
                            } else {
                                self.selected_costumes.clear();
                                self.selected_costumes.insert(idx);
                                self.selection_range_pivot = idx;
                            }

                            if self.selected_costumes.len() == 1 && self.selected_costumes.contains(&idx) {
                                assert_eq!(*self.selected_costumes.iter().last().unwrap(), idx);
                                let save_name = save.save_name.clone();
                                let account_name = save.get_account_name();
                                let character_name = save.get_character_name();
                                let timestamp = save.j2000_timestamp;
                                let costume_spec = save.get_costume_spec();
                                let costume_hash = save.get_costume_hash();

                                let account_name = account_name.to_owned();
                                let character_name = character_name.to_owned();
                                let costume_spec = costume_spec.to_owned();
                                let costume_hash = costume_hash.to_owned();

                                if let Some(costume_edit) = self.costume_edit.as_mut() {
                                    costume_edit.save_name = save_name;
                                    costume_edit.account_name = account_name;
                                    costume_edit.character_name = character_name;
                                    costume_edit.timestamp = timestamp;
                                    costume_edit.strip_timestamp = false;
                                    costume_edit.costume_spec = costume_spec;
                                    costume_edit.costume_hash = costume_hash;
                                } else {
                                    self.costume_edit = Some(CostumeEdit {
                                        save_name,
                                        account_name,
                                        character_name,
                                        timestamp,
                                        costume_spec,
                                        costume_hash,
                                        ..Default::default()
                                    });
                                }
                            }
                        }

                        save.image_visible_in_edit = self.selected_costumes.len() == 1 && self.selected_costumes.contains(&idx);

                        // Forget the texture if our image is loaded but not actually visible anywhere.
                        // FIXME this is very aggressive forgetting. Maybe we only want to forget
                        // if it hasn't been visible for some number of seconds?
                        if let CostumeImage::Loaded(texture_handle) = &save.image_texture {
                            if !save.image_visible_in_grid && !save.image_visible_in_edit {
                                ctx.forget_image(&texture_handle.name());
                                save.image_texture = CostumeImage::NotLoaded;
                            }
                        }

                    }
                });
            });
        });
    }
}

fn main() {
    let costume_dir = env::var("COSTUMES_DIR").expect("COSTUMES_DIR env var not set");
    env::set_current_dir(&costume_dir).expect("failed to set current directory to COSTUME_DIR");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    _ = eframe::run_native(
        "Champions Costume Manager",
        options,
        Box::new(|cc| {
            let (ui_message_tx, ui_message_rx) = mpsc::channel::<UiMessage>();
            let (ui_priority_message_tx, ui_priority_message_rx) = mpsc::channel::<UiPriorityMessage>();
            // For the UI thread to communicate that it updated the filesystem so the scanner
            // thread doesn't run unnecessarily.
            let (scanner_tx, scanner_rx) = mpsc::channel::<SystemTime>();
            // For the main app to signal graceful thread shutdown on exit
            let shutdown_flag = Arc::new(RwLock::new(atomic::AtomicBool::new(false)));

            let mut support_thread_handles: Vec<thread::JoinHandle<()>> = Vec::new();

            let available_cores = thread::available_parallelism().map(NonZero::get).unwrap_or(MAX_DECODE_THREADS);
            let num_workers = MAX_DECODE_THREADS.min(available_cores);
            let (decode_job_tx, decode_job_rx) = mpsc::channel::<OsString>();
            let decode_job_rx = Arc::new(Mutex::new(decode_job_rx));

            // workers for decoding
            // let mut workers: Vec<thread::JoinHandle<()>> = Vec::with_capacity(num_workers);
            for id in 0..num_workers {
                let decode_job_rx = Arc::clone(&decode_job_rx);
                let ui_message_tx = ui_message_tx.clone();
                let shutdown_flag = Arc::clone(&shutdown_flag);
                let ctx = cc.egui_ctx.clone();
                let decode_worker_handle = thread::spawn(move || {
                    loop {
                        // TODO make sure that the RwLock is dropped after this "if" statement
                        if shutdown_flag.read().unwrap().load(atomic::Ordering::Acquire) { break; }
                        let decode_job = decode_job_rx.lock().unwrap().recv_timeout(Duration::from_millis(32));
                        if let Ok(file_name) = decode_job {
                            _ = ui_message_tx.send(UiMessage::Log(Log::new_now(format!("attempting to decode {:?}", file_name), LogLevel::Info, AppThread::Decode(id as u8))));
                            // TODO Instead of reading the file again, maybe we should just
                            // serialize the costume and use _those_ bytes? The costume data is
                            // owned by a hashmap behind a mutex though... Or maybe we need to
                            // store the CostumeSaveFiles themselves behind an RwLock.
                            let jpeg_bytes = match fs::read(&file_name) {
                                Ok(bytes) => bytes,
                                Err(_err) => {
                                    // TODO log read failure
                                    continue;
                                }
                            };

                            let mut decoder = zune_jpeg::JpegDecoder::new(jpeg_bytes);
                            // TODO when we implement logging, if this fails send to the UI as an error to display.
                            if let Ok(pixels) = decoder.decode() {
                                // TODO default if doesn't exist
                                let info = decoder.info().expect("no jpeg info");
                                let image = egui::ColorImage::from_rgb([info.width as usize, info.height as usize], &pixels);
                                let texture_handle = ctx.load_texture(file_name.to_str().unwrap(), image, egui::TextureOptions::default());
                                _ = ui_message_tx.send(UiMessage::Log(Log::new_now(format!("decoded {:?}", file_name), LogLevel::Info, AppThread::Decode(id as u8))));
                                _ = ui_message_tx.send(UiMessage::JpegDecoded { file_name, texture_handle });
                                ctx.request_repaint();
                            }
                        }
                    }
                });

                support_thread_handles.push(decode_worker_handle);
                // workers.push(join_handle);
            }


            // TODO maybe store some struct that contains the last modified date of the file and the
            // costume save metadata? Then if the file was modified underneath us we can reload it.
            // struct Something { last_modified: LastModifiedTimestamp, save: CostumeSaveFile }
            // NOTE If we do this, then we don't have to get the file metadata during sorting since
            // it'll already be here in the hashmap.
            let saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>> = Arc::new(Mutex::new(HashMap::new()));
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // SCANNING THREAD
            {
                let saves = Arc::clone(&saves);
                let shutdown_flag = Arc::clone(&shutdown_flag);
                let frame = cc.egui_ctx.clone();
                let scanner_handle = thread::spawn(move || {
                    let mut last_modified_time: Option<SystemTime> = None;
                    loop {
                        // TODO make sure that the RwLock is dropped after this "if" statement
                        if shutdown_flag.read().unwrap().load(atomic::Ordering::Acquire) { break; }

                        let current_dir = env::current_dir().unwrap();
                        let modified_time = fs::metadata(&current_dir).unwrap().modified().unwrap();
                        // If the UI initiated file system changes we need to know so that we don't
                        // misidentify an external file system change.
                        while let Ok(ui_last_modified_time) = scanner_rx.try_recv() {
                            last_modified_time = Some(ui_last_modified_time);
                        }

                        if last_modified_time.is_none_or(|lmt| modified_time != lmt) {
                            last_modified_time = Some(modified_time);
                            let mut saves = saves.lock().unwrap();
                            let mut missing_files: HashSet<OsString> = HashSet::from_iter(saves.keys().cloned());
                            for entry in fs::read_dir(&current_dir).unwrap().flatten() {
                                // TODO check that the file starts with Costume_ and is a jpeg file. If not,
                                // continue. Should that logic be a part of CostumeSaveFile?
                                let file_name = entry.file_name();
                                #[allow(clippy::map_entry)]
                                if saves.contains_key(&file_name) {
                                    missing_files.remove(&file_name);
                                    // TODO maybe log if we failed to parse the costume save?
                                } else {
                                    let file_stem = Path::new(&file_name).file_stem().unwrap().to_str().unwrap();
                                    // FIXME log error to UI if we fail to read!
                                    let jpeg_raw = fs::read(&file_name).expect("failed to read file");
                                    if let Ok(save) = CostumeSaveFile::new(file_stem, &jpeg_raw) {
                                        saves.insert(file_name, save);
                                    }
                                }
                            }
                            for missing_file in missing_files {
                                // TODO figure out if we need to explicitly forget image textures here.
                                saves.remove(&missing_file);
                            }
                            _ = ui_priority_message_tx.send(UiPriorityMessage::FileListChangedExternally);
                            frame.request_repaint();
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                });

                support_thread_handles.push(scanner_handle);
            }

            let args = AppArgs {
                saves,
                shutdown_flag,
                support_thread_handles,
                ui_priority_message_rx,
                ui_message_rx,
                scanner_tx,
                decode_job_tx,
            };

            Ok(Box::new(App::new(cc, args)))
        })
    );
}
