// TODO Once this is in a more stable state and prototyping is finished, run through and figure out
// how to reduce the amount of cloning we're doing.

// TODO Eventually refactor to compress UI actions (e.g. selected file save, delete, etc.). I think
// we should wait until we're further along to actually implement this. If we abstract or DRY up
// too early then when we want to go through and harden the app by adding things like logging on
// filesystem interaction failure, it might just make implementing it more of a nightmare.

// TODO refactor so that the UI always loads and can display errors, even ones related to startup.

// TODO Better app config / startup. Here's probably what I want to do:
// Attempt to load app config from file.
// If file doesn't exist or data is invalid, display a window asking for user config.
// When config changes, save to file.

// TODO add an actual event system? e.g. saving/deleting files could be triggered by events
// dispatched from the UI, and any errors would also be fed into the event system instead of the
// logging system.
// This could also solve an issue where displaying the UI takes a while because the scanner thread
// locks the saves hashmap and parses _every_ costume file on startup. I probably could change this
// now using a channel and just lock the hashmap to retrieve the current keys.

// TODO refactor and simplify the app update loop. Really just for the modal/dialog stuff at the
// moment since we only ever should show one of those at a time.

// TODO should we verify that a costume directory is selected before performing a costume file
// operation (e.g. save, delete)

// FIXME
// 1) select a costume
// 2) edit the spec, save and close
// 3) edit the spec again, uncheck the checkbox to revert the change
// 4) can't save!
// to fix: maybe have a "revert changes" button?

mod jpeg;
mod costume;

use eframe::egui;

use std::{
    str,
    env,
    fmt,
    error,
    io,
    num::NonZero,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    io::prelude::*,
    path::{PathBuf, Path},
    fs,
    sync::{Arc, Mutex, RwLock, atomic, mpsc, LazyLock},
    thread,
    time::{Duration, SystemTime},
};


enum CostumeImage {
    NotLoaded,
    Loading,
    Loaded(egui::TextureHandle),
}

// TODO audit what fields we actually need.
//
// TODO should we move file_name, in_game_display_name, and j2000_timestamp into CostumeSave?
// We have a weird situation where functions in the costume module need information that's only
// available on the CostumeEntry, which is more of a UI thing.
struct CostumeEntry {
    /// Parsed costume jpeg save.
    save: costume::CostumeSave,
    /// The full name of the file including the extension.
    file_name: String,
    /// Represents how the name of the file appears in-game.
    in_game_display_name: String,
    j2000_timestamp: Option<i64>,
    image_texture: CostumeImage,
    image_visible_in_grid: bool,
    image_visible_in_edit: bool,
}

impl CostumeEntry {
    fn new(file_path: &Path, save: costume::CostumeSave) -> Self {
        let file_name = file_path.file_name().unwrap().to_str().unwrap().to_owned();
        let j2000_timestamp = file_path.file_stem().unwrap().to_str().unwrap()
            .split('_')
            .last().unwrap()
            .parse::<i64>().ok();
        let metadata = save.get_metadata();
        let in_game_display_name = costume::get_in_game_display_name(metadata.account_name, metadata.character_name, j2000_timestamp);

        Self {
            save,
            j2000_timestamp,
            image_texture: CostumeImage::NotLoaded,
            image_visible_in_grid: false,
            image_visible_in_edit: false,
            file_name,
            in_game_display_name,
        }
    }

    /// Get the name of the save file as it appears between the "Costume_" prefix and either the
    /// j2000 timestamp suffix (if included) or the file extension.
    fn get_save_name(&self) -> &str {
        let save_name_start = self.file_name.find("_").unwrap();
        let save_name_end = if self.j2000_timestamp.is_some() {
            self.file_name.rfind("_").unwrap()
        } else {
            self.file_name.rfind(".").unwrap()
        };

        // Technically the file name can just be "Costume_.jpg"
        if save_name_start == save_name_end {
            ""
        } else {
            &self.file_name[save_name_start + 1 .. save_name_end]
        }
    }

}

// TODO If there are many error types maybe break the types into their own enum and do this:
// struct AppError { kind: AppErrorKind, message: Option<String> }
// enum AppErrorKind { CostumeSaveFailed { source: Option<io::Error>, which: OsString } }
/// Application errors that currently all require acknowledgement by the UI.
#[derive(Debug)]
enum AppError {
    CostumeSaveFailed { source: Option<io::Error>, which: PathBuf, message: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CostumeSaveFailed { source, which, message } => {
                let header = format!("Failed to save costume {which:?}: {message}");
                if let Some(source) = source {
                    write!(f, "{header} - {source}")
                } else {
                    write!(f, "{header}")
                }
            }
        }
    }
}

impl error::Error for AppError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::CostumeSaveFailed { source, .. } => source.as_ref().map(|err| err as &dyn error::Error)
        }
    }
}

const MAX_LOGS: u16 = 1024;

enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

struct Log {
    level: LogLevel,
    timestamp: chrono::NaiveDateTime,
    source: &'static str,
    message: String,
}

impl Log {
    fn new_now(level: LogLevel, message: &str, source: &'static str) -> Self {
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

struct Logger {
    /// List of logs with the oldest at the end and the newest at the front.
    logs: VecDeque<Log>,
    max_logs: u16,
    // NOTE(RA): This may not be thread-safe. If two threads produce an error that needs user
    // action/acknowledgement, only the most recent one will be read. Ideally the system as a whole
    // should pause operations that could produce errors if the user needs to acknowledge an error,
    // but there might always be that little bit of time where two threads have seen that no error
    // is being awaited and then proceed to both produce an error before checking again...
    /// Will contain an error if the UI needs to acknowledge it.
    last_error: Option<AppError>,
}

impl Logger {
    fn new(max_logs: u16) -> Self {
        Self {
            logs: VecDeque::with_capacity(max_logs as usize),
            max_logs,
            last_error: None,
        }
    }

    fn add_log(&mut self, log: Log) {
        // TODO remove, debugging only
        // Or maybe find a way to also print logs only if the console is running
        println!("{}", log.message);
        if self.logs.len() == self.max_logs.into() {
            self.logs.pop_back();
        }
        self.logs.push_front(log);
    }
}

type SharedLogger = Arc<RwLock<Logger>>;

struct GlobalLogger(SharedLogger);

struct LoggerHandle {
    thread_id: &'static str,
    shared_logger: SharedLogger,
}

impl GlobalLogger {
    fn new_handle(&self, thread_id: &'static str) -> LoggerHandle {
        LoggerHandle {
            shared_logger: Arc::clone(&self.0),
            thread_id,
        }
    }
}

// TODO maybe we should separate the error system from the logging system?
impl LoggerHandle {
    fn log(&self, level: LogLevel, message: &str) {
        let log = Log::new_now(level, message, self.thread_id);
        if let Ok(mut logger) = self.shared_logger.write() {
            logger.add_log(log);
        }
    }

    /// Log an error that needs to be presented to and acknowledged by the user.
    fn log_err_ack_required(&self, error: AppError) {
        let log = Log::new_now(LogLevel::Error, &error.to_string(), self.thread_id);
        if let Ok(mut logger) = self.shared_logger.write() {
            logger.add_log(log);
            logger.last_error = Some(error);
        }
    }

    fn ui_ack_required(&self) -> bool {
        self.shared_logger.read().unwrap().last_error.is_some()
    }

    fn ack_errors(&mut self) {
        if let Ok(mut logger) = self.shared_logger.write() {
            logger.last_error = None;
        }
    }
}

static LOGGER: LazyLock<GlobalLogger> = LazyLock::new(|| GlobalLogger(Arc::new(RwLock::new(Logger::new(MAX_LOGS)))));

/// Critical messages that must be handled as soon as possible
enum UiPriorityMessage {
    /// We have detected that the file system has changed underneath us in some way.
    FileListChangedExternally,
}

/// Regular messages whose handling can be delayed for one or many frames
enum UiMessage {
    JpegDecoded { file_path: PathBuf, texture_handle: egui::TextureHandle },
}

#[derive(Default)]
struct CostumeEdit {
    strip_timestamp: bool,

    timestamp: Option<i64>,
    save_name: String,
    account_name: String,
    character_name: String,

    // These fields do not affect the indirect fields.
    costume_spec: String,
    costume_hash: String,

    // Indirect fields: The follow fields aren't directly edited; they are just cached for efficiency.
    file_name: String,
    in_game_display_name: String,
}

impl CostumeEdit {
    fn new_from_entry(entry: &CostumeEntry) -> Self {
        let file_name = entry.file_name.to_owned();
        let in_game_display_name = entry.in_game_display_name.to_owned();
        let save_name = entry.get_save_name().to_owned();
        let timestamp = entry.j2000_timestamp;
        let metadata = entry.save.get_metadata();

        Self {
            strip_timestamp: timestamp.is_none(),
            save_name,
            timestamp,
            account_name: metadata.account_name.to_owned(),
            character_name: metadata.character_name.to_owned(),
            costume_spec: metadata.spec.to_owned(),
            costume_hash: metadata.hash.to_owned(),
            file_name, 
            in_game_display_name,
        }
    }

    /// Call this to regenerate indirect fields whenever one of the following is changed:
    /// - timestamp
    /// - save_name
    /// - account_name
    /// - character_name
    fn regenerate_indirect_fields(&mut self) {
        self.file_name = costume::get_file_name(&self.save_name, self.timestamp);
        self.in_game_display_name = costume::get_in_game_display_name(&self.account_name, &self.character_name, self.timestamp);
    }
}

#[derive(PartialEq, Copy, Clone)]
enum DisplayType { DisplayName, FileName }

#[derive(PartialEq, Copy, Clone)]
enum SortType { Name, CreationTime, ModifiedTime }

// TODO maybe tie the selected costume and costume edit together so they can never get out of sync?
struct App {
    costume_dir: Arc<RwLock<Option<PathBuf>>>,

    costume_entries: Arc<Mutex<HashMap<PathBuf, CostumeEntry>>>,

    logger: LoggerHandle,

    // So that we can gracefully shut down all threads on exit
    shutdown_flag: Arc<atomic::AtomicBool>,
    support_thread_handles: Vec<thread::JoinHandle<()>>,

    ui_priority_message_rx: mpsc::Receiver<UiPriorityMessage>,
    ui_message_rx: mpsc::Receiver<UiMessage>,
    scanner_tx: mpsc::Sender<SystemTime>,
    // TODO can we make this send &Path instead of PathBuf?
    decode_job_tx: mpsc::Sender<PathBuf>,

    file_exists_warning_modal_open: bool,
    show_images_in_selection_list: bool,
    costume_spec_edit_open: bool,
    confirm_edit_spec: bool,
    // TODO try to make this a Vec<&Path> if possible
    sorted_saves: Vec<PathBuf>,
    /// Values are indices into self.sorted_saves.
    selected_costumes: HashSet<usize>,
    selection_range_pivot: usize,
    display_type: DisplayType,
    sort_type: SortType,
    costume_edit: Option<CostumeEdit>,
}

struct AppArgs {
    costume_dir: Arc<RwLock<Option<PathBuf>>>,
    costume_entries: Arc<Mutex<HashMap<PathBuf, CostumeEntry>>>,
    shutdown_flag: Arc<atomic::AtomicBool>,
    support_thread_handles: Vec<thread::JoinHandle<()>>,
    ui_priority_message_rx: mpsc::Receiver<UiPriorityMessage>,
    ui_message_rx: mpsc::Receiver<UiMessage>,
    scanner_tx: mpsc::Sender<SystemTime>,
    // TODO can we make this a Sender<&Path>?
    decode_job_tx: mpsc::Sender<PathBuf>,
    logger: LoggerHandle,
}

impl App {
    fn new(
        _cc: &eframe::CreationContext,
        AppArgs {
            costume_dir,
            costume_entries,
            shutdown_flag,
            support_thread_handles,
            ui_priority_message_rx,
            ui_message_rx,
            scanner_tx,
            decode_job_tx,
            logger,
        }: AppArgs,
    ) -> Self
    {
        Self {
            costume_dir,

            costume_entries,

            logger,

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
    fn sort_saves(sort_type: SortType, display_type: DisplayType, keys_to_sort: &mut [PathBuf], locked_costume_entries: &std::sync::MutexGuard<HashMap<PathBuf, CostumeEntry>>) {
        match sort_type {
            SortType::Name => {
                match display_type {
                    DisplayType::DisplayName => {
                        keys_to_sort.sort_by_key(|k| {
                            let save = &locked_costume_entries[k];
                            save.in_game_display_name.to_ascii_lowercase()
                        });
                    },

                    DisplayType::FileName => {
                        keys_to_sort.sort_by_key(|k| k.file_stem().unwrap().to_ascii_lowercase());
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
        self.logger.log(LogLevel::Info, "signalling shutdown");
        self.shutdown_flag.store(true, atomic::Ordering::Release);
        // NOTE(RA): I think draining the vector like this is okay for now since we shouldn't
        // update again after handling this exit event.
        self.support_thread_handles.drain(..).for_each(|thread_handle| {
            thread_handle.join().unwrap();
        });
        self.logger.log(LogLevel::Info, "shutdown complete");
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let window_rect = ctx.available_rect();

        if self.logger.ui_ack_required() {
            egui::Modal::new(egui::Id::new("error modal")).show(ctx, |ui| {
                ui.set_max_size(window_rect.size() * 0.9);
                ui.set_min_width(0.0);

                {
                    let logger = self.logger.shared_logger.read().unwrap();
                    let error = logger.last_error.as_ref().unwrap();
                    let header = match error {
                        AppError::CostumeSaveFailed { .. } => "Costume Save Failed",
                    };
                    let description = error.to_string();
                    ui.label(header);
                    ui.label(description);

                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let locked_logger = self.logger.shared_logger.read().unwrap();
                        for log in locked_logger.logs.iter() {
                            ui.label(&log.message);
                        }
                    });
                    ui.separator();
                }

                ui.horizontal(|ui| {
                    if ui.button("Okay").clicked() {
                        self.logger.ack_errors();
                    }

                    if ui.button("Copy Logs to Clipboard").clicked() {
                        let logs = &self.logger.shared_logger.read().unwrap().logs;
                        let num_logs = logs.len();
                        // 100 bytes per log + bytes for newlines be a good starting assumption
                        let mut serialized = String::with_capacity(num_logs * 100 + num_logs - 1);
                        for Log { message, .. } in logs.iter() {
                            serialized.push_str(message);
                            serialized.push('\n');
                        }

                        ui.output_mut(|platform| platform.copied_text = serialized);
                    }
                });
            });

            return;
        }

        let mut costume_entries = self.costume_entries.lock().unwrap();
        let current_modifiers = ctx.input(|input| input.modifiers);

        while let Ok(priority_message) = self.ui_priority_message_rx.try_recv() {
            match priority_message {
                UiPriorityMessage::FileListChangedExternally => {
                    // NOTE(RA): For now we're just going to reset selections whenever the file
                    // list changes. Might change this later. We used to only do this whenever the
                    // file(s) the user was viewing were removed from the file system.
                    self.selected_costumes.clear();
                    self.costume_edit = None;
                    self.sorted_saves = costume_entries.keys().cloned().collect();
                    Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &costume_entries);
                },
            }
        }

        const MAX_MESSAGES_PER_FRAME: usize = 32;
        for _ in 0..MAX_MESSAGES_PER_FRAME {
            let message = self.ui_message_rx.try_recv();
            if message.is_err() { break; }
            match message.unwrap() {
                UiMessage::JpegDecoded { file_path, texture_handle } => {
                    if costume_entries.contains_key(&file_path) {
                        costume_entries.get_mut(&file_path).unwrap().image_texture = CostumeImage::Loaded(texture_handle);
                    }
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
            assert!(self.costume_edit.is_some());

            let modal = egui::Modal::new(egui::Id::new("Costume Spec Edit"));
            modal.show(ctx, |ui| {
                ui.set_max_height(window_rect.height() * 0.9);
                ui.set_max_width(window_rect.width() * 0.5);
                ui.set_min_size([0.0, 0.0].into());

                ui.label("EDITING THE COSTUME SPEC IS AN EXPERIMENTAL AND DANGEROUS FEATURE! BEWARE!");
                ui.label("Incorrectly modifying the costume spec can corrupt your save and make it unloadable in-game! Make a backup!");
                ui.label("Spec changes will be saved upon closing this modal and saving the costume.");
                if ui.checkbox(&mut self.confirm_edit_spec, "I have read and understand the above warnings and want to proceed (toggle off to revert all changes)").changed() && !self.confirm_edit_spec {
                    // Reset to original in case user made changes but then changed their mind.
                    // FIXME unnecessary clone if the user hasn't changed the spec/hash and is just
                    // toggling the checkbox on/off for some reason.
                    let save_idx = self.selected_costumes.iter().last().unwrap();
                    let entry = &costume_entries[&self.sorted_saves[*save_idx]];
                    let costume::CostumeMetadata { spec, hash, .. } = entry.save.get_metadata();
                    self.costume_edit.as_mut().unwrap().costume_spec = spec.to_owned();
                    self.costume_edit.as_mut().unwrap().costume_hash = hash.to_owned();
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Hash:");
                    ui.label(&self.costume_edit.as_ref().unwrap().costume_hash);
                });

                let scroll_area = egui::ScrollArea::vertical();
                scroll_area.show(ui, |ui| {
                    let spec_editor = ui.add_enabled(
                        self.confirm_edit_spec,
                        egui::TextEdit::multiline(&mut self.costume_edit.as_mut().unwrap().costume_spec)
                            .code_editor()
                            .desired_rows(12)
                            .desired_width(f32::INFINITY)
                    );

                    if spec_editor.changed() {
                        self.costume_edit.as_mut().unwrap().costume_hash = costume::generate_costume_hash(&self.costume_edit.as_ref().unwrap().costume_spec);
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
                    let costume_path = &self.sorted_saves[*self.selected_costumes.iter().last().unwrap()].clone();
                    let costume = costume_entries.get_mut(costume_path).unwrap();
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
                            _ = self.decode_job_tx.send(costume_path.clone());
                            costume.image_texture = CostumeImage::Loading;
                        }
                        ui.label("loading image...");
                    }

                    ui.horizontal(|ui| {
                        ui.label("File Name:");
                        ui.label(&costume_edit.file_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("In-Game Display:");
                        ui.label(&costume_edit.in_game_display_name);
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Save Name:");
                        if ui.text_edit_singleline(&mut costume_edit.save_name).changed() {
                            costume_edit.regenerate_indirect_fields();
                        }
                    });
                    if costume.j2000_timestamp.is_some() && ui.checkbox(&mut costume_edit.strip_timestamp, "Strip Timestamp").changed() {
                        if costume_edit.strip_timestamp {
                            costume_edit.timestamp = None;
                        } else {
                            costume_edit.timestamp = costume.j2000_timestamp;
                        }

                        costume_edit.regenerate_indirect_fields();
                    }

                    ui.horizontal(|ui| {
                        ui.label("Account Name:");
                        if ui.text_edit_singleline(&mut costume_edit.account_name).changed() {
                            costume_edit.regenerate_indirect_fields();
                        };
                    });
                    ui.horizontal(|ui| {
                        ui.label("Character Name:");
                        if ui.text_edit_singleline(&mut costume_edit.character_name).changed() {
                            costume_edit.regenerate_indirect_fields();
                        }
                    });
                    if ui.button("Edit Spec").clicked() {
                        self.costume_spec_edit_open = true;
                    }

                    // TODO disable button if nothing was changed?
                    // TODO if only file name was changed maybe only rename the file via OS?
                    if ui.button("Save").clicked() {
                        let costume_dir = self.costume_dir.read().unwrap();
                        debug_assert!(costume_dir.is_some());
                        let costume_dir = costume_dir.as_ref().unwrap();
                        debug_assert!(costume_path.as_path().parent() == Some(costume_dir));

                        let old_file_path = costume_path;
                        let new_file_path = costume_dir.join(&costume_edit.file_name);
                        let file_name_changed = new_file_path.file_name().unwrap() != old_file_path.file_name().unwrap();

                        // FIXME There is potentially a massive, terrible bug here on Windows where
                        // we're NOT catching here due to Windows' case insensitivity! If we've
                        // changed the file name but saves contains the same name with a different
                        // casing, we WON'T catch it!
                        if file_name_changed && costume_entries.contains_key(&new_file_path) {
                            self.file_exists_warning_modal_open = true;
                        } else {
                            self.logger.log(LogLevel::Info, format!("attempting to save {old_file_path:?} as {new_file_path:?}").as_str());
                            // TODO Create a SaveError type, map all errors in this block to that
                            // and include additional information about the failed operation, and
                            // just use the `?` operator. Return a result and log if there's an
                            // error afterward.
                            // TODO Clean up the error handling and maybe implement a new type
                            // that can auto-remove or roll back temp files if the save operation
                            // fails. There are also cases where if the save operation fails we
                            // just leave our temp files laying around...
                            (|| {
                                let costume = costume_entries.get_mut(costume_path).unwrap();
                                // NOTE we use a temp file so that we're not immediately
                                // overwriting the existing file in the case the file name hasn't
                                // changed. If the save operation fails we don't want to lose or
                                // corrupt the original file.
                                let mut temp_file_path = new_file_path.clone();
                                // FIXME should probably grab the current extension of new_file_path and
                                // use that to create the temp extension
                                temp_file_path.set_extension("jpg.CCM_TEMP");
                                let mut temp_file = match fs::File::create(&temp_file_path) {
                                    Ok(file) => file,
                                    Err(err) => {
                                        let costume_save_error = AppError::CostumeSaveFailed {
                                            which: costume_path.clone(),
                                            source: Some(err),
                                            message: format!("failed to open temp file {temp_file_path:?}"),
                                        };
                                        self.logger.log_err_ack_required(costume_save_error);
                                        return;
                                    }
                                };

                                let updates = costume::UpdateCostumeMetadata {
                                    account_name: Some(costume_edit.account_name.clone()),
                                    character_name: Some(costume_edit.character_name.clone()),
                                    spec: Some(costume_edit.costume_spec.clone()),
                                    hash: Some(costume_edit.costume_hash.clone()),
                                };
                                costume.save.update_metadata(updates);
                                costume.in_game_display_name = costume_edit.in_game_display_name.clone();
                                if file_name_changed {
                                    costume.file_name = costume_edit.file_name.clone();
                                    costume.j2000_timestamp = costume_edit.timestamp;
                                }
                                let serialized = costume.save.0.serialize();

                                if let Err(err) = temp_file.write_all(&serialized) {
                                    let costume_save_error = AppError::CostumeSaveFailed {
                                        which: costume_path.clone(),
                                        source: Some(err),
                                        message: format!("failed to write temp file {temp_file_path:?}"),
                                    };
                                    self.logger.log_err_ack_required(costume_save_error);
                                    return;
                                }

                                // Need to copy old creation time to new file
                                #[cfg(windows)]
                                {
                                    // TODO should we just continue with our save operation if we
                                    // fail to update file times? I don't think it's really _that_
                                    // important. Maybe we can just warn.
                                    use std::os::windows::fs::FileTimesExt;
                                    let old_file = match fs::File::open(old_file_path) {
                                        Ok(file) => file,
                                        Err(err) => {
                                            let costume_save_error = AppError::CostumeSaveFailed {
                                                which: costume_path.clone(),
                                                source: Some(err),
                                                message: format!("failed to open original file {old_file_path:?} for reading"),
                                            };
                                            self.logger.log_err_ack_required(costume_save_error);
                                            return;
                                        }
                                    };
                                    let old_metadata = match old_file.metadata() {
                                        Ok(metadata) => metadata,
                                        Err(err) => {
                                            let costume_save_error = AppError::CostumeSaveFailed {
                                                which: costume_path.clone(),
                                                source: Some(err),
                                                message: format!("failed to get metadata for original file {old_file_path:?}"),
                                            };
                                            self.logger.log_err_ack_required(costume_save_error);
                                            return;
                                        },
                                    };
                                    let new_metadata = match temp_file.metadata() {
                                        Ok(metadata) => metadata,
                                        Err(err) => {
                                            let costume_save_error = AppError::CostumeSaveFailed {
                                                which: costume_path.clone(),
                                                source: Some(err),
                                                message: format!("failed to get metadata for temp file {temp_file_path:?}"),
                                            };
                                            self.logger.log_err_ack_required(costume_save_error);
                                            return;
                                        },
                                    };
                                    // SAFETY: This section is conditionally compiled for windows so
                                    // setting/getting the file creation time should not error.
                                    let times = fs::FileTimes::new()
                                        .set_created(old_metadata.created().unwrap())
                                        .set_accessed(new_metadata.accessed().unwrap())
                                        .set_modified(new_metadata.modified().unwrap());
                                    if let Err(err) = temp_file.set_times(times) {
                                        let costume_save_error = AppError::CostumeSaveFailed {
                                            which: costume_path.clone(),
                                            source: Some(err),
                                            message: format!("failed to update filetimes for {temp_file_path:?}"),
                                        };
                                        self.logger.log_err_ack_required(costume_save_error);
                                        return;
                                    }
                                }

                                // Rename the old file so that we have something to revert to if
                                // the temp file rename fails.
                                let mut old_file_renamed_path = old_file_path.clone();
                                old_file_renamed_path.set_extension("jpg.CCM_TEMP");
                                if let Err(err) = fs::rename(old_file_path, &old_file_renamed_path) {
                                    let costume_save_error = AppError::CostumeSaveFailed {
                                        which: costume_path.clone(),
                                        source: Some(err),
                                        message: "failed to rename original file".to_string(),
                                    };
                                    self.logger.log_err_ack_required(costume_save_error);
                                    // TODO remove the temp file?
                                    return;
                                }

                                if let Err(err) = fs::rename(temp_file_path, &new_file_path) {
                                    let costume_save_error = AppError::CostumeSaveFailed {
                                        which: costume_path.clone(),
                                        source: Some(err),
                                        message: "failed to rename temp file".to_string(),
                                    };
                                    self.logger.log_err_ack_required(costume_save_error);
                                    // Revert the name of the old file
                                    if let Err(err) = fs::rename(&old_file_renamed_path, old_file_path) {
                                        // TODO should this be a separate app error?
                                        let costume_save_error = AppError::CostumeSaveFailed {
                                            which: costume_path.clone(),
                                            source: Some(err),
                                            message: format!(
                                                "failed to revert old file rename ({:?} --> {:?})",
                                                old_file_renamed_path,
                                                old_file_path
                                            )
                                        };
                                        self.logger.log_err_ack_required(costume_save_error);
                                    }

                                    return;
                                }

                                if let Err(err) = fs::remove_file(&old_file_renamed_path) {
                                    self.logger.log(LogLevel::Warn, format!("failed to remove renamed old file path: {err}").as_str());
                                }

                                self.logger.log(LogLevel::Info, format!("successfully saved {new_file_path:?}").as_str());

                                // FIXME really lazy and inefficient. I don't think we can know where the new
                                // save name will be after sorting (maybe we actually can) but we can probably
                                // pass along the index of the old save with this event. Would eliminate an
                                // entire scan through the sorted_saves array.
                                let (old_index, _) = self.sorted_saves.iter().enumerate().find(|(_, save)| *save == old_file_path).unwrap();
                                assert!(self.selected_costumes.remove(&old_index));

                                let entry = costume_entries.remove(old_file_path).unwrap();
                                costume_entries.insert(new_file_path.clone(), entry);
                                self.sorted_saves = costume_entries.keys().cloned().collect();
                                Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &costume_entries);

                                let (new_index, _) = self.sorted_saves.iter().enumerate().find(|(_, save)| **save == new_file_path).unwrap();
                                self.selected_costumes.insert(new_index);

                                // TODO find a way to compress this code since we do the exact same thing when
                                // deleting files.

                                // Signal to the scanning thread that we initiated the file system change.
                                // This avoids cases where we update the file system, react to the update,
                                // then the scanner sees that something was changed and gives us ANOTHER
                                // notification that the file system was changed.
                                let costume_dir = self.costume_dir.read().unwrap();
                                if costume_dir.is_none() {
                                    self.logger.log(LogLevel::Warn, "somehow we performed a costume file operation with no costume directory selected");
                                    return;
                                }

                                let last_modified_time = fs::metadata(costume_dir.as_ref().unwrap()).unwrap().modified().unwrap();
                                let _ = self.scanner_tx.send(last_modified_time);
                            })();
                        }
                    }
                }

                if ui.button("Delete").clicked() {
                    // TODO show delete confirmation popup
                    for selected_idx in self.selected_costumes.iter() {
                        let costume_path = &self.sorted_saves[*selected_idx];
                        if let Err(err) = fs::remove_file(costume_path) {
                            self.logger.log(LogLevel::Error, format!("failed to delete {costume_path:?}: {err}").as_str());
                            continue;
                        }
                        costume_entries.remove(costume_path);
                        self.logger.log(LogLevel::Info, format!("deleted {costume_path:?}").as_str());
                    }
                    self.sorted_saves = costume_entries.keys().cloned().collect();
                    Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &costume_entries);
                    self.selected_costumes.clear();
                    // TODO find a way to compress this code since we do the exact same thing when
                    // saving files.

                    // Signal to the scanning thread that we initiated the file system change.
                    // This avoids cases where we update the file system, react to the update,
                    // then the scanner sees that something was changed and gives us ANOTHER
                    // notification that the file system was changed.
                    let costume_dir = self.costume_dir.read().unwrap();
                    if costume_dir.is_none() {
                        self.logger.log(LogLevel::Warn, "somehow we performed a costume file operation with no costume directory selected");
                        return;
                    }

                    let last_modified_time = fs::metadata(costume_dir.as_ref().unwrap()).unwrap().modified().unwrap();
                    let _ = self.scanner_tx.send(last_modified_time);
                }
            }

        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Change").clicked() {
                    // TODO I feel like this could be a bit cleaner
                    let new_costume_dir = {
                        let costume_dir = self.costume_dir.read().unwrap();
                        let rfd_dir = costume_dir.as_ref().map(|pb| pb.to_str().unwrap()).unwrap_or("/");
                        self.logger.log(LogLevel::Info, format!("rfd dir: {rfd_dir:?}").as_str());
                        rfd::FileDialog::new()
                            .set_title("Select your Champions Online screenshots directory")
                            .set_directory(rfd_dir)
                            .pick_folder()
                    };
                    if let Some(dir) = new_costume_dir {
                        self.logger.log(LogLevel::Info, format!("changing costume directory to {dir:?}").as_str());
                        if let Err(e) = fs::write(APP_CONFIG_FILE_NAME, dir.to_str().unwrap()) {
                            // TODO should we display this error to the user?
                            self.logger.log(LogLevel::Error, &e.to_string());
                        }
                        self.costume_dir.write().unwrap().replace(dir);
                    }
                }

                ui.label("Costume Directory:");
                if let Some(dir) = self.costume_dir.read().unwrap().as_ref() {
                    ui.label(dir.to_str().unwrap());
                } else {
                    ui.label("No costume directory selected");
                }
            });

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
                Self::sort_saves(self.sort_type, self.display_type, &mut self.sorted_saves, &costume_entries);
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
                        let entry = costume_entries.get_mut(save_file_name).unwrap();
                        let is_selected = self.selected_costumes.contains(&idx);
                        let display_name = match self.display_type {
                            DisplayType::DisplayName => entry.in_game_display_name.as_str(),
                            DisplayType::FileName => entry.file_name.as_str(),
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
                                        if let CostumeImage::Loaded(texture) = &entry.image_texture {
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

                            entry.image_visible_in_grid = scroll_area_clip_rect.intersects(custom_button.rect);
                            if entry.image_visible_in_grid && matches!(entry.image_texture, CostumeImage::NotLoaded) {
                                _ = self.decode_job_tx.send(save_file_name.clone());
                                entry.image_texture = CostumeImage::Loading;
                            }

                            custom_button
                        } else {
                            entry.image_visible_in_grid = false;
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
                                self.costume_edit = Some(CostumeEdit::new_from_entry(entry));
                            }
                        }

                        entry.image_visible_in_edit = self.selected_costumes.len() == 1 && self.selected_costumes.contains(&idx);

                        // Forget the texture if our image is loaded but not actually visible anywhere.
                        // FIXME this is very aggressive forgetting. Maybe we only want to forget
                        // if it hasn't been visible for some number of seconds?
                        if let CostumeImage::Loaded(texture_handle) = &entry.image_texture {
                            if !entry.image_visible_in_grid && !entry.image_visible_in_edit {
                                ctx.forget_image(&texture_handle.name());
                                entry.image_texture = CostumeImage::NotLoaded;
                            }
                        }
                    }
                });
            });
        });
    }
}

// FIXME this directory is windows-specific
const DEFAULT_COSTUME_DIR: &str = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Champions Online\\Champions Online\\Live\\screenshots";
const APP_CONFIG_FILE_NAME: &str = "ccm_config.cfg";

fn main() {
    let exe_path = env::current_exe().expect("failed to get dir of executable");
    env::set_current_dir(exe_path.parent().unwrap()).expect("failed to set cwd");

    let mut costume_dir: Option<PathBuf> = None;
    if fs::exists(APP_CONFIG_FILE_NAME).expect("failed to check if app config file exists") {
        let app_config_bytes = fs::read(APP_CONFIG_FILE_NAME).unwrap();
        costume_dir.replace(String::from_utf8(app_config_bytes).unwrap().into());
    } else if fs::exists(DEFAULT_COSTUME_DIR).expect("failed to check if default costume dir exists") {
        costume_dir.replace(DEFAULT_COSTUME_DIR.into());
    }
    let costume_dir = Arc::new(RwLock::new(costume_dir));

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
            let shutdown_flag = Arc::new(atomic::AtomicBool::new(false));

            let mut support_thread_handles: Vec<thread::JoinHandle<()>> = Vec::new();

            const MAX_DECODE_THREADS: usize = 8;
            const DECODE_THREAD_NAMES: [&str; MAX_DECODE_THREADS] = [
                "DECODE 0", "DECODE 1", "DECODE 2", "DECODE 3",
                "DECODE 4", "DECODE 5", "DECODE 6", "DECODE 7",
            ];

            let available_cores = thread::available_parallelism().map(NonZero::get).unwrap_or(MAX_DECODE_THREADS);
            let num_workers = MAX_DECODE_THREADS.min(available_cores);
            let (decode_job_tx, decode_job_rx) = mpsc::channel::<PathBuf>();
            let decode_job_rx = Arc::new(Mutex::new(decode_job_rx));

            // workers for decoding
            for decode_thread_id in DECODE_THREAD_NAMES.iter().take(num_workers) {
                let decode_job_rx = Arc::clone(&decode_job_rx);
                let ui_message_tx = ui_message_tx.clone();
                let shutdown_flag = Arc::clone(&shutdown_flag);
                let ctx = cc.egui_ctx.clone();
                let logger = LOGGER.new_handle(decode_thread_id);
                let decode_worker_handle = thread::spawn(move || {
                    loop {
                        if shutdown_flag.load(atomic::Ordering::Acquire) {
                            break;
                        }
                        if logger.ui_ack_required() {
                            thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                        let decode_job = decode_job_rx.lock().unwrap().recv_timeout(Duration::from_millis(32));

                        if let Ok(file_path) = decode_job {
                            // TODO Instead of reading the file again, maybe we should just
                            // serialize the costume and use _those_ bytes? The costume data is
                            // owned by a hashmap behind a mutex though... Or maybe we need to
                            // store the CostumeSaveFiles themselves behind an RwLock.
                            let jpeg_bytes = match fs::read(&file_path) {
                                Ok(bytes) => bytes,
                                Err(err) => {
                                    logger.log(LogLevel::Warn, format!("failed to decode {:?}: {}", file_path, err).as_str());
                                    // TODO send message to UI that we failed to decode this jpeg
                                    // so we can maybe display a warning icon or something
                                    continue;
                                }
                            };

                            let mut decoder = zune_jpeg::JpegDecoder::new(jpeg_bytes);
                            // TODO when we implement logging, if this fails send to the UI as an error to display.
                            if let Ok(pixels) = decoder.decode() {
                                // TODO default if doesn't exist
                                let info = decoder.info().expect("no jpeg info");
                                let image = egui::ColorImage::from_rgb([info.width as usize, info.height as usize], &pixels);
                                let texture_handle = ctx.load_texture(file_path.to_str().unwrap(), image, egui::TextureOptions::default());
                                logger.log(LogLevel::Info, format!("decoded {:?}", file_path).as_str());
                                _ = ui_message_tx.send(UiMessage::JpegDecoded { file_path, texture_handle });
                                ctx.request_repaint();
                            }
                        }
                    }

                    logger.log(LogLevel::Info, "shutting down");
                });

                support_thread_handles.push(decode_worker_handle);
            }


            // TODO maybe store some struct that contains the last modified date of the file and the
            // costume save metadata? Then if the file was modified underneath us we can reload it.
            // struct Something { last_modified: LastModifiedTimestamp, save: CostumeSaveFile }
            // NOTE If we do this, then we don't have to get the file metadata during sorting since
            // it'll already be here in the hashmap.
            let costume_entries: Arc<Mutex<HashMap<PathBuf, CostumeEntry>>> = Arc::new(Mutex::new(HashMap::new()));
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // SCANNING THREAD
            {
                let costume_dir = Arc::clone(&costume_dir);
                let costume_entries = Arc::clone(&costume_entries);
                let shutdown_flag = Arc::clone(&shutdown_flag);
                let frame = cc.egui_ctx.clone();
                let logger = LOGGER.new_handle("SCANNER");
                let scanner_handle = thread::spawn(move || {
                    let mut last_modified_time: Option<SystemTime> = None;
                    loop {
                        if shutdown_flag.load(atomic::Ordering::Acquire) {
                            break;
                        }
                        if logger.ui_ack_required() {
                            thread::sleep(Duration::from_millis(100));
                            continue;
                        }

                        // If the UI initiated file system changes we need to know so that we don't
                        // misidentify an external file system change.
                        //
                        // FIXME we might miss some messages, resulting in erroneous file system
                        // detections. I think that because we haven't blocked trying to lock the
                        // saves hashmap yet, the UI can still technically send messages through
                        // this pipe AFTER the scanner thread has passed it this loop iteration...
                        while let Ok(ui_last_modified_time) = scanner_rx.try_recv() {
                            last_modified_time = Some(ui_last_modified_time);
                        }

                        let directory_entries_to_check = (|| {
                            // TODO cleanup
                            // TODO use match statements to log on fs failures
                            if let Ok(costume_dir_guard) = costume_dir.try_read() {
                                if let Some(dir_to_read) = costume_dir_guard.as_ref() {
                                    if let Ok(modified_time) = fs::metadata(dir_to_read).and_then(|m| m.modified()) {
                                        if last_modified_time.is_none_or(|lmt| modified_time != lmt) {
                                            logger.log(LogLevel::Info, "detected file system change");
                                            last_modified_time = Some(modified_time);
                                            return fs::read_dir(dir_to_read).ok();
                                        }
                                    }
                                }
                            }

                            None
                        })();

                        if let Some(directory_entries_to_check) = directory_entries_to_check {
                            let mut costume_entries = costume_entries.lock().unwrap();
                            let mut missing_files: HashSet<PathBuf> = HashSet::from_iter(costume_entries.keys().cloned());
                            let mut num_new_files = 0;
                            for directory_entry in directory_entries_to_check.flatten() {
                                let file_path = directory_entry.path();
                                #[allow(clippy::map_entry)]
                                if costume_entries.contains_key(&file_path) {
                                    missing_files.remove(file_path.as_path());
                                } else if costume::is_valid_costume_file_name(&file_path) {
                                    let jpeg_raw = match fs::read(&file_path) {
                                        Ok(contents) => contents,
                                        Err(err) => {
                                            logger.log(LogLevel::Warn, format!("error reading {file_path:?}: {}", err).as_str());
                                            continue;
                                        }
                                    };

                                    let save = match costume::CostumeSave::parse(&jpeg_raw) {
                                        Ok(parsed) => parsed,
                                        Err(err) => {
                                            logger.log(LogLevel::Warn, format!("failed to parse save file {file_path:?}: {}", err).as_str());
                                            continue;
                                        }
                                    };

                                    let costume_entry = CostumeEntry::new(&file_path, save);
                                    costume_entries.insert(file_path, costume_entry);
                                    num_new_files += 1;
                                }
                            }
                            let num_missing_files = missing_files.len();
                            for missing_file in missing_files {
                                // TODO figure out if we need to explicitly forget image textures here.
                                costume_entries.remove(&missing_file);
                            }
                            logger.log(LogLevel::Info, format!("added {num_new_files} new costumes, removed {num_missing_files} missing costumes").as_str());
                            _ = ui_priority_message_tx.send(UiPriorityMessage::FileListChangedExternally);
                            frame.request_repaint();
                        }

                        thread::sleep(Duration::from_millis(100));
                    }

                    logger.log(LogLevel::Info, "shutting down");
                });

                support_thread_handles.push(scanner_handle);
            }

            let args = AppArgs {
                costume_dir,
                costume_entries,
                shutdown_flag,
                support_thread_handles,
                ui_priority_message_rx,
                ui_message_rx,
                scanner_tx,
                decode_job_tx,
                logger: LOGGER.new_handle("UI"),
            };

            Ok(Box::new(App::new(cc, args)))
        })
    );
}
