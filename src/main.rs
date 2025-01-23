mod jpeg;

use jpeg::{
    Jpeg,
    JpegSegmentType,
    JpegApp13Payload,
    APP13_RECORD_APP,
    APP13_RECORD_APP_CAPTION,
    APP13_RECORD_APP_OBJECT_DATA_PREVIEW,
};

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
    fn new_from_path(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let jpeg_raw = std::fs::read(path)?;
        let costume_jpeg = Jpeg::parse(jpeg_raw)?;
        let file_stem = path
            .file_stem().unwrap()
            .to_str().unwrap();
        let j2000_timestamp = file_stem
            .split('_')
            .last().unwrap()
            .parse::<i64>().ok();
        let save_name = {
            let save_name_start = file_stem.find("_").unwrap() + 1;
            let save_name_end = if j2000_timestamp.is_some() { file_stem.rfind("_").unwrap() } else { file_stem.len() };
            file_stem[save_name_start .. save_name_end].to_owned()
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

    // FIXME The max date that the game can display is 2068-01-19 03:14:07 but we don't handle this
    // edge case. Our simulated in-game display name datestring will go (almost) arbitrarily high.
    fn get_in_game_display_name(&self) -> String {
        let account_name = self.get_account_name();
        let character_name = self.get_character_name();
        let maybe_datetime_string = self.j2000_timestamp.and_then(|j2000_timestamp| {
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

    /// Constructs and returns the full file name in one of two forms:
    /// A) If timestamp is present: "Costume_savename_timestamp.jpg"
    /// B) If timestamp not present: "Costume_savename.jpg"
    fn get_file_name(&self) -> String {
        if let Some(j2000_timestamp) = self.j2000_timestamp {
            format!("Costume_{}_{}.jpg", self.save_name, j2000_timestamp)
        } else {
            format!("Costume_{}.jpg", self.save_name)
        }
    }
}

fn main() {
    let costume_dir = std::env::var("COSTUMES_DIR").expect("COSTUMES_DIR env var not set");
    std::env::set_current_dir(&costume_dir).expect("failed to set current directory to COSTUME_DIR");
    use std::collections::HashMap;
    let mut saves: HashMap<std::ffi::OsString, CostumeSaveFile> = HashMap::new();
    for entry in std::fs::read_dir(&costume_dir).unwrap().flatten() {
        let file_name = entry.file_name();
        if let Ok(costume_save) = CostumeSaveFile::new_from_path(std::path::Path::new(&file_name)) {
            saves.insert(file_name, costume_save);
        }
    }

    #[derive(PartialEq)]
    enum DisplayType { DisplayName, FileName }
    let mut display_type = DisplayType::DisplayName;

    let mut ui_save_display: Vec<std::ffi::OsString> = saves.keys().cloned().collect();
    ui_save_display.sort();

    use eframe::egui;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    #[derive(Default)]
    struct CostumeEdit {
        strip_timestamp: bool,
        timestamp: Option<i64>,
        save_name: String,
        account_name: String,
        character_name: String,
    }

    // TODO maybe tie the selected costume and costume edit together so they can never get out of sync?
    let mut costume_edit: Option<CostumeEdit> = None;
    let mut selected_costume: Option<std::ffi::OsString> = None;

    // TODO once in-house jpeg image decoding (SOS) is implemented we can probably get rid of the
    // image and maybe a few of the egui_extras dependencies

    // NOTE If we want to write app state to disk we need to enable the "persistence" feature for
    // eframe and use eframe::run_native() instead of eframe::run_simple_native().
    // https://docs.rs/eframe/latest/eframe/
    _ = eframe::run_simple_native("Champions Costume Manager", options, move |ctx, _| {

        egui::SidePanel::right("details_display").show(ctx, |ui| {
            // NOTE: For now we're just assuming that the selected costume and the costume edit
            // data are properly tied together. Maybe we should tie these together better so that
            // they can't possibly get out of sync.
            if let Some(costume_file_name) = selected_costume.as_ref() {
                let costume = &saves[costume_file_name];
                let costume_edit = costume_edit.as_mut().unwrap();

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
                    // NOTE stolen from the CostumeSaveFile impl
                    // TODO maybe make a trait that has the get_file_name and
                    // get_in_game_display_name functions? Then stick it on the CostumeSaveFile and
                    // the CostumeEdit structs?
                    let file_name = if let Some(j2000_timestamp) = costume_edit.timestamp {
                        format!("Costume_{}_{}.jpg", costume_edit.save_name, j2000_timestamp)
                    } else {
                        format!("Costume_{}.jpg", costume_edit.save_name)
                    };
                    ui.label(file_name);
                });
                // FIXME again, don't want to construct this every frame
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("In-Game Display:");

                    // NOTE also stolen from CostumeSaveFile impl
                    let account_name = &costume_edit.account_name;
                    let character_name = &costume_edit.character_name;
                    let maybe_datetime_string = costume_edit.timestamp.and_then(|j2000_timestamp| {
                        const JAN_1_2000_UNIX_TIME: i64 = 946684800;
                        let unix_timestamp = JAN_1_2000_UNIX_TIME + j2000_timestamp;
                        chrono::DateTime::from_timestamp(unix_timestamp, 0)
                            .map(|utc_datetime| utc_datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                    });

                    let value = if let Some(datetime_string) = maybe_datetime_string {
                        format!("{}{} {}", account_name, character_name, datetime_string)
                    } else {
                        format!("{}{}", account_name, character_name)
                    };

                    ui.label(value);
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
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("Account Name:");
                    ui.text_edit_singleline(&mut costume_edit.account_name);
                });
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("Character Name:");
                    ui.text_edit_singleline(&mut costume_edit.character_name);
                });

                if ui.button("Save").clicked() {
                    // let save = &saves[costume_file_name];
                    println!("TODO saving");
                }
            } else {
                ui.label("Select a save to view details");
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                if ui.selectable_value(&mut display_type, DisplayType::DisplayName, "Display Name").clicked() {
                    ui_save_display.sort_by_key(|k| saves[k].get_in_game_display_name());
                }
                if ui.selectable_value(&mut display_type, DisplayType::FileName, "File Name").clicked() {
                    ui_save_display.sort();
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for save_file_name in ui_save_display.iter() {
                    let save = &saves[save_file_name];
                    if ui.selectable_value(
                        &mut selected_costume,
                        Some(save_file_name.clone()),
                        match display_type {
                            DisplayType::DisplayName => save.get_in_game_display_name(),
                            DisplayType::FileName => save.get_file_name(),
                        }
                    ).clicked() {
                        let save_name = save.save_name.clone();
                        let account_name = save.get_account_name().to_owned();
                        let character_name = save.get_character_name().to_owned();
                        let timestamp = save.j2000_timestamp;

                        if let Some(costume_edit) = costume_edit.as_mut() {
                            costume_edit.save_name = save_name;
                            costume_edit.account_name = account_name;
                            costume_edit.character_name = character_name;
                        } else {
                            costume_edit = Some(CostumeEdit {
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

    });
}
