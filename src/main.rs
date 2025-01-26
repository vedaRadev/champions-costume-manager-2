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
    collections::{HashMap, HashSet},
    io::prelude::*,
    ffi::OsString,
    path::Path,
    env,
    fs,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, SystemTime},
};

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

// TODO list:
// * Caching of in-game display names, only recalc when changed to improve GUI perf.
// * Maybe caching of file name as well (currently requires dynamic creation of string)?
struct CostumeSaveFile {
    jpeg: Jpeg,
    /// The name of the save file as it appears between the "Costume_" prefix and j2000 timestamp
    /// (if included) suffix.
    save_name: String,
    j2000_timestamp: Option<i64>,
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
    fn new_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let jpeg_raw = fs::read(path)?;
        let costume_jpeg = Jpeg::parse(jpeg_raw)?;
        let file_stem = path
            .file_stem().unwrap()
            .to_str().unwrap();
        let j2000_timestamp = file_stem
            .split('_')
            .last().unwrap()
            .parse::<i64>().ok();
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
        })
    }

    fn get_app13_payload(&self) -> &JpegApp13Payload {
        let app13_segment = self.jpeg.get_segment(JpegSegmentType::APP13).unwrap()[0];
        let app13_payload = app13_segment.get_payload_as::<JpegApp13Payload>();
        app13_payload
    }

    fn get_app13_payload_mut(&mut self) -> &mut JpegApp13Payload {
        let app13_segment = self.jpeg.get_segment_mut(JpegSegmentType::APP13).unwrap().swap_remove(0);
        let app13_payload = app13_segment.get_payload_as_mut::<JpegApp13Payload>();
        app13_payload
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

enum UiMessage {
    FileListChanged
}

#[derive(PartialEq)]
enum CostumeEditType { Simple, Advanced }
impl Default for CostumeEditType {
    fn default() -> Self { Self::Simple }
}

#[derive(Default)]
struct CostumeEdit {
    edit_type: CostumeEditType,
    strip_timestamp: bool,
    timestamp: Option<i64>,
    simple_name: String,
    save_name: String,
    account_name: String,
    character_name: String,
}

#[derive(PartialEq, Copy, Clone)]
enum DisplayType { DisplayName, FileName }

// TODO maybe tie the selected costume and costume edit together so they can never get out of sync?
struct App {
    saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>>,
    ui_message_tx: mpsc::Sender<UiMessage>,
    ui_message_rx: mpsc::Receiver<UiMessage>,

    file_exists_warning_modal_open: bool,
    sorted_saves: Vec<OsString>,
    selected_costume: Option<OsString>,
    display_type: DisplayType,
    costume_edit: Option<CostumeEdit>,
}

impl App {
    // TODO customization
    fn new(
        _cc: &eframe::CreationContext,
        saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>>,
        ui_message_tx: mpsc::Sender<UiMessage>,
        ui_message_rx: mpsc::Receiver<UiMessage>,
    ) -> Self
    {
        Self {
            saves,
            ui_message_tx,
            ui_message_rx,

            file_exists_warning_modal_open: false,
            sorted_saves: vec![],
            selected_costume: None,
            display_type: DisplayType::DisplayName,
            costume_edit: None,
        }
    }

    fn sort_saves(display_type: DisplayType, keys_to_sort: &mut [OsString], locked_saves: &std::sync::MutexGuard<HashMap<OsString, CostumeSaveFile>>) {
        match display_type {
            DisplayType::DisplayName => {
                keys_to_sort.sort_by_key(|k| {
                    let save = &locked_saves[k];
                    get_in_game_display_name(save.get_account_name(), save.get_character_name(), save.j2000_timestamp)
                });
            },

            DisplayType::FileName => {
                keys_to_sort.sort();
            },
        }
    }
}

impl eframe::App for App {
    // TODO once in-house jpeg image decoding (SOS) is implemented we can probably get rid of the
    // image and maybe a few of the egui_extras dependencies
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut saves = self.saves.lock().unwrap();

        while let Ok(notification) = self.ui_message_rx.try_recv() {
            match notification {
                UiMessage::FileListChanged => {
                    self.sorted_saves = saves.keys().cloned().collect();
                    Self::sort_saves(self.display_type, &mut self.sorted_saves, &saves);
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

        egui::SidePanel::right("details_display").show(ctx, |ui| {
            // NOTE: For now we're just assuming that the selected costume and the costume edit
            // data are properly tied together. Maybe we should tie these together better so that
            // they can't possibly get out of sync.
            if let Some(costume_file_name) = self.selected_costume.as_ref() {
                let costume = saves.get(costume_file_name).unwrap();
                let costume_edit = self.costume_edit.as_mut().unwrap();

                egui_extras::install_image_loaders(ctx);
                let file = format!("file://{}", costume_file_name.to_str().unwrap());
                let image = egui::Image::new(file.as_str())
                    .maintain_aspect_ratio(true)
                    .max_height(500.0);
                ui.add(image);

                // FIXME we probably do not want to construct the file name every frame. Maybe
                // cache it in the CostumeEdit struct itself?
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("File Name:");
                    ui.label(get_file_name(&costume_edit.save_name, costume_edit.timestamp));
                });
                // FIXME again, don't want to construct this every frame
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("In-Game Display:");
                    ui.label(match costume_edit.edit_type {
                        CostumeEditType::Simple => get_in_game_display_name(&costume_edit.simple_name, "", costume_edit.timestamp),
                        CostumeEditType::Advanced => get_in_game_display_name(&costume_edit.account_name, &costume_edit.character_name, costume_edit.timestamp),
                    });
                });

                ui.separator();

                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("Edit Type:");
                    ui.selectable_value(&mut costume_edit.edit_type, CostumeEditType::Simple, "Simple");
                    ui.selectable_value(&mut costume_edit.edit_type, CostumeEditType::Advanced, "Advanced");
                });

                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
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

                if costume_edit.edit_type == CostumeEditType::Simple {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.label("Name (in-game):");
                        ui.text_edit_singleline(&mut costume_edit.simple_name);
                    });
                } else {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.label("Account Name:");
                        ui.text_edit_singleline(&mut costume_edit.account_name);
                    });
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.label("Character Name:");
                        ui.text_edit_singleline(&mut costume_edit.character_name);
                    });
                }

                // TODO If the user is not on windows, maybe we need to change how we're saving
                // things. Instead of writing a new file then deleting the old and attempting to
                // update the file creation times, maybe we should just overwrite the old so that
                // we keep the file creation time.
                // FIXME Logging, not crashing!
                if ui.button("Save").clicked() {
                    let old_file_name = costume_file_name;
                    let new_file_name = OsString::from(get_file_name(&costume_edit.save_name, costume_edit.timestamp));
                    let file_name_changed = *new_file_name != *old_file_name;

                    if file_name_changed && saves.contains_key(&new_file_name) {
                        self.file_exists_warning_modal_open = true;
                    } else {
                        let costume = saves.get_mut(costume_file_name).unwrap();
                        costume.save_name = costume_edit.save_name.clone();
                        costume.j2000_timestamp = costume_edit.timestamp;
                        if costume_edit.edit_type == CostumeEditType::Simple {
                            costume.set_account_name(costume_edit.simple_name.clone());
                            costume.set_character_name(String::from(""));
                        } else {
                            costume.set_account_name(costume_edit.account_name.clone());
                            costume.set_character_name(costume_edit.character_name.clone());
                        }
                        let serialized = costume.jpeg.serialize();

                        let mut file = fs::File::create(&new_file_name).unwrap_or_else(|err| {
                            eprintln!("Failed to open {:?} for writing: {err}", new_file_name);
                            std::process::exit(1);
                        });
                        if let Err(err) = file.write_all(&serialized) {
                            eprintln!("failed to write to file {:?}: {err}", new_file_name);
                            std::process::exit(1);
                        }

                        if file_name_changed {
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
                                let new_metadata = file.metadata().unwrap_or_else(|err| {
                                    eprintln!("failed to get metadata for new file {new_file_name:?}: {err}");
                                    std::process::exit(1);
                                });
                                // SAFETY: This section is conditionally compiled for windows so
                                // setting/getting the file creation time should not error.
                                let times = fs::FileTimes::new()
                                    .set_created(old_metadata.created().unwrap())
                                    .set_accessed(new_metadata.accessed().unwrap())
                                    .set_modified(new_metadata.modified().unwrap());
                                if let Err(err) = file.set_times(times) {
                                    eprintln!("failed to update filetimes for {new_file_name:?}: {err}");
                                    std::process::exit(1);
                                }
                            }

                            if let Err(err) = fs::remove_file(old_file_name) {
                                eprintln!("failed to remove original file {old_file_name:?}: {err}");
                                std::process::exit(1);
                            }

                            // HACK for updating hashmap and display vec after save
                            // TODO find a better way to do this (event system, periodic file system
                            // scanning on another thread, whatever)
                            // FIXME _reselect_ the costume after saving. We need to repopulate
                            // CostumeEdit data with the new data in the file itself. For example,
                            // if we do a Simple save then swap to the Advanced view, the account
                            // and character fields are still populated with fields from the last
                            // selection even though the file now doesn't have a character name.
                            {
                                // TODO maybe temporary? Once periodic file system scanning is implemented,
                                // might be able to get rid of this.
                                // Maybe use a ui message to notify that a file should be removed
                                // or added?
                                let costume = saves.remove(old_file_name).unwrap();
                                saves.insert(new_file_name.clone(), costume);

                                // FIXME maybe go through the little ui messaging system to signal
                                // that an update is needed.
                                self.sorted_saves = saves.keys().cloned().collect();
                                App::sort_saves(self.display_type, &mut self.sorted_saves, &saves);
                                self.selected_costume = Some(new_file_name);
                            }
                        }
                    }
                }
            } else {
                ui.label("Select a save to view details");
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                let display_name_button = ui.selectable_value(&mut self.display_type, DisplayType::DisplayName, "Display Name");
                let file_name_button = ui.selectable_value(&mut self.display_type, DisplayType::FileName, "File Name");
                if display_name_button.clicked() || file_name_button.clicked() {
                    Self::sort_saves(self.display_type, &mut self.sorted_saves, &saves);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for save_file_name in self.sorted_saves.iter() {
                    let save = &saves[save_file_name];
                    if ui.selectable_value(
                        &mut self.selected_costume,
                        Some(save_file_name.clone()),
                        match self.display_type {
                            DisplayType::DisplayName => get_in_game_display_name(save.get_account_name(), save.get_character_name(), save.j2000_timestamp),
                            DisplayType::FileName => get_file_name(&save.save_name, save.j2000_timestamp),
                        }
                    ).clicked() {
                        let save_name = save.save_name.clone();
                        let account_name = save.get_account_name().to_owned();
                        let character_name = save.get_character_name().to_owned();
                        let timestamp = save.j2000_timestamp;
                        let simple_name = format!("{}{}", account_name, character_name);

                        if let Some(costume_edit) = self.costume_edit.as_mut() {
                            costume_edit.save_name = save_name;
                            costume_edit.simple_name = simple_name;
                            costume_edit.account_name = account_name;
                            costume_edit.character_name = character_name;
                            costume_edit.timestamp = timestamp;
                        } else {
                            self.costume_edit = Some(CostumeEdit {
                                simple_name,
                                save_name,
                                account_name,
                                character_name,
                                timestamp,
                                ..Default::default()
                            });
                        }
                    }
                }
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

            // TODO maybe store some struct that contains the last modified date of the file and the
            // costume save metadata? Then if the file was modified underneath us we can reload it.
            // struct Something { last_modified: LastModifiedTimestamp, save: CostumeSaveFile }
            let saves: Arc<Mutex<HashMap<OsString, CostumeSaveFile>>> = Arc::new(Mutex::new(HashMap::new()));

            {
                let saves = Arc::clone(&saves);
                let frame = cc.egui_ctx.clone();
                let ui_message_tx = ui_message_tx.clone();
                thread::spawn(move || {
                    let mut last_modified_time: Option<SystemTime> = None;
                    loop {
                        let modified_time = fs::metadata(&costume_dir).unwrap().modified().unwrap();
                        if last_modified_time.is_none_or(|lmt| modified_time != lmt) {
                            last_modified_time = Some(modified_time);
                            let mut saves = saves.lock().unwrap();
                            let mut missing_files: HashSet<OsString> = HashSet::from_iter(saves.keys().cloned());
                            for entry in fs::read_dir(&costume_dir).unwrap().flatten() {
                                // TODO check that the file starts with Costume_ and is a jpeg file. If not,
                                // continue. Should that logic be a part of CostumeSaveFile?
                                let file_name = entry.file_name();
                                #[allow(clippy::map_entry)]
                                if saves.contains_key(&file_name) {
                                    missing_files.remove(&file_name);
                                    // TODO maybe log if we failed to parse the costume save?
                                } else if let Ok(save) = CostumeSaveFile::new_from_path(Path::new(&file_name)) {
                                    saves.insert(file_name, save);
                                }
                            }
                            for missing_file in missing_files {
                                saves.remove(&missing_file);
                            }
                            _ = ui_message_tx.send(UiMessage::FileListChanged);
                            frame.request_repaint();
                        }
                        thread::sleep(Duration::from_millis(1000));
                    }
                });
            }

            Ok(Box::new(App::new(
                cc,
                saves,
                ui_message_tx,
                ui_message_rx,
            )))
        })
    );
}
