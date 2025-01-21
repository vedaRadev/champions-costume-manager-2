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

enum InspectType { Short, Long }

#[derive(Default)]
struct AppArgs {
    /// Required. The file path of the costume save.
    costume_save_file_path: Option<std::path::PathBuf>,
    /// New account name to set in costume jpeg metadata.
    new_account_name: Option<String>,
    /// New character name to set in costume jpeg metadata.
    new_character_name: Option<String>,
    /// New save name to set excluding the "Costume_" prefix and j2000 timestamp suffix.
    new_save_name: Option<String>,
    /// Whether or not to strip the J2000 timestamp from the end of the filename.
    should_strip_timestamp: bool,
    /// If costume metadata and in-game save display should be displayed. Defaults to short
    /// inspection, which does not include costume hash and costume specification. To specify long
    /// inspection, use "--inspect long". If specified with with mutative options such as
    /// --set-character-name or --strip-timestamp, the mutations are applied first then the
    /// save is inspected.
    inspect_type: Option<InspectType>,
    /// Should mutative options be ignored? Really only useful for seeing how potential changes
    /// will cause the save to appear in-game.
    dry_run: bool,
}

static HELP_STRING: &str = r#"
A tool for organizing and managing costume saves for Champions Online.

CAUTION: This tool will effectively OVERWRITE costume saves, so be careful! If
you want to view the results of potential changes before overwriting a file, use
--dry-run in conjunction with --inspect. --dry-run does not need to be specified
if no mutative options are used.

This tool is early in development, so back up your saves and use at your own risk!

Usage: ccm.exe <costume save file path> [options]

-h, --help
    Show this usage information.

-c, --set-character-name <character_name>
    Set the character name that will be displayed in-game.

-a, --set-account-name <account_name>
    Set the account name that will be displayed in-game.

-s, --set-save-name <save_name>
    Set the portion of the filename between the "Costume_" prefix and the j2000
    timestamp suffix (if it exists).

-t, --strip-timestamp
    Strips the j2000 timestamp suffix from the save file name, removing the date
    display from its entry in the in-game save menu. If there is no j2000
    timestamp or if the timestamp is invalid, this is effectively a no-op.
    NOTE: If you want to re-add an in-game date display, you will need to
    calculate your own j2000 timestamp and append it to the end of the file name
    yourself in the form "_<timestamp>".

-i, --inspect [short|long]
    Print file name, in-game save display, and costume metadata. Defaults to
    short if no specification is supplied. Long will print the costume hash and
    proprietary costume specification as well as all the information that short
    displays.
    If this option is supplied in conjunction with mutative options such as
    --set-character-name or --strip-timestamp, the mutative options are applied
    first then the updated information is inspected and displayed.

--dry-run
    Applies mutative options to the in-memory costume save but does not write
    the results to disk. Use in conjunction with --inspect to see the results of
    potential changes. This option does not need to be specified if no mutative
    options are used.
"#;

fn run_command_line_util(raw_args: std::env::Args) {
    let mut raw_args = raw_args.skip(1).peekable();
    let mut app_args: AppArgs = AppArgs::default();
    while let Some(arg) = raw_args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{HELP_STRING}");
                std::process::exit(0);
            },

            // TODO Should we allow for setting account names to empty strings?
            "--set-account-name" | "-a" => {
                app_args.new_account_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected account name");
                    std::process::exit(1);
                });
            },

            // TODO Should we allow for setting character names to empty strings?
            "--set-character-name" | "-c" => {
                app_args.new_character_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected character name");
                    std::process::exit(1);
                });
            },

            "--set-save-name" | "-s" => {
                app_args.new_save_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected file name");
                    std::process::exit(1);
                });
            },

            "--strip-timestamp" | "-t" => {
                app_args.should_strip_timestamp = true;
            },

            // NOTE If the user specifies this more than once, the last-specified InspectType will be used.
            "--inspect" | "-i" => {
                let maybe_specifier = raw_args.peek();
                if maybe_specifier.is_none_or(|s| s.starts_with('-')) {
                    // NOTE: Default to short if no specifier is provided.
                    app_args.inspect_type = Some(InspectType::Short);
                } else {
                    app_args.inspect_type = match maybe_specifier.unwrap().as_str() {
                        "short" => Some(InspectType::Short),
                        "long" => Some(InspectType::Long),
                        _ => {
                            eprintln!("Unrecognized inspection specifier: {}", maybe_specifier.unwrap());
                            std::process::exit(1);
                        }
                    };

                    raw_args.next();
                }
            },

            "--dry-run" => {
                app_args.dry_run = true;
            }
            
            _ => {
                if arg.starts_with('-') {
                    eprintln!("Unrecognized option: {}", arg);
                    std::process::exit(1);
                }

                if app_args.costume_save_file_path.is_some() {
                    eprintln!("Multiple files specified");
                    std::process::exit(1);
                }

                app_args.costume_save_file_path = Some(std::path::PathBuf::from(arg));
            },
        }
    }
    
    // filename validation
    if app_args.costume_save_file_path.is_none() {
        eprintln!("Costume save file path is required");
        std::process::exit(1);
    }
    if !app_args.costume_save_file_path.as_ref().unwrap().file_stem().unwrap().to_string_lossy().starts_with("Costume_") {
        eprintln!(r#"Invalid costume save file: file name must begin with "Costume_""#);
        std::process::exit(1);
    }
    if app_args.costume_save_file_path.as_ref().unwrap().extension().is_none_or(|ext| ext.to_string_lossy().to_lowercase() != "jpg") {
        eprintln!(r#"Invalid costume save file: must have ".jpg" extension"#);
        std::process::exit(1);
    }

    // SAFETY: costume_save_file_path has been determined to be a Some value at this point
    let mut costume_save = CostumeSaveFile::new_from_path(app_args.costume_save_file_path.as_ref().unwrap()).unwrap_or_else(|err| {
        eprintln!("Failed to create costume save: {err}");
        std::process::exit(1);
    });

    let mut dirty = false;
    if let Some(new_account_name) = app_args.new_account_name {
        costume_save.set_account_name(new_account_name);
        dirty = true;
    }
    if let Some(new_character_name) = app_args.new_character_name {
        costume_save.set_character_name(new_character_name);
        dirty = true;
    }
    if let Some(new_save_name) = app_args.new_save_name {
        costume_save.save_name = new_save_name;
        dirty = true;
    }
    if app_args.should_strip_timestamp {
        if costume_save.j2000_timestamp.is_some() {
            costume_save.j2000_timestamp = None;
            dirty = true;
        } else {
            println!("WARNING: --strip-timestamp was specified but there is no j2000 timestamp to strip from the filename");
        }
    }

    let costume_file_name = costume_save.get_file_name();
    let full_path = app_args.costume_save_file_path
        .as_ref().unwrap()
        .parent().unwrap()
        .join(costume_file_name);

    if let Some(inspect_type) = app_args.inspect_type {
        let account_name = costume_save.get_account_name();
        let character_name = costume_save.get_character_name();
        let in_game_display = costume_save.get_in_game_display_name();
        println!("File: {}", full_path.to_str().unwrap());
        println!(r#"Displayed in-game as: "{}""#, in_game_display);
        println!("Account: {}", account_name);
        println!("Character: {}", character_name);
        if matches!(inspect_type, InspectType::Long) {
            let costume_hash = costume_save.get_costume_hash();
            let costume_spec = costume_save.get_costume_spec();
            println!("Costume Hash: {}", costume_hash);
            println!("Costume Spec: {}", costume_spec);
        }
    }

    if !app_args.dry_run && dirty {
        println!();
        use std::io::prelude::*;
        let serialized = costume_save.jpeg.serialize();
        let mut file = std::fs::File::create(&full_path).unwrap_or_else(|err| {
            eprintln!("Failed to open {:?} for writing: {err}", full_path);
            std::process::exit(1);
        });
        if let Err(err) = file.write_all(&serialized) {
            eprintln!("failed to write to file {:?}: {err}", full_path);
            std::process::exit(1);
        } else {
            println!("wrote file {:?}", full_path);
        }

        if &full_path != app_args.costume_save_file_path.as_ref().unwrap() {
            // Copy file creation time from old file to new file, not applicable on unix systems
            // TODO If something fails when trying to copy file creation time from the old file to
            // the new file, should we just continue instead of failing?
            #[cfg(windows)]
            {
                use std::os::windows::fs::FileTimesExt;
                let old_file = std::fs::File::open(app_args.costume_save_file_path.as_ref().unwrap()).unwrap_or_else(|err| {
                    eprintln!("failed to open original file {:?} for reading: {err}", app_args.costume_save_file_path.as_ref().unwrap());
                    std::process::exit(1);
                });
                let old_metadata = old_file.metadata().unwrap_or_else(|err| {
                    eprintln!("failed to get metadata for original file {:?}: {err}", app_args.costume_save_file_path.as_ref().unwrap());
                    std::process::exit(1);
                });
                let new_metadata = file.metadata().unwrap_or_else(|err| {
                    eprintln!("failed to get metadata for new file {full_path:?}: {err}");
                    std::process::exit(1);
                });
                // SAFETY: This section is conditionally compiled for windows so
                // setting/getting the file creation time should not error.
                let times = std::fs::FileTimes::new()
                    .set_created(old_metadata.created().unwrap())
                    .set_accessed(new_metadata.accessed().unwrap())
                    .set_modified(new_metadata.modified().unwrap());
                if let Err(err) = file.set_times(times) {
                    eprintln!("failed to update filetimes for {full_path:?}: {err}");
                    std::process::exit(1);
                }
                println!("updated filetimes for {full_path:?}");
            }

            if let Err(err) = std::fs::remove_file(app_args.costume_save_file_path.as_ref().unwrap()) {
                eprintln!("failed to remove original file {:?}: {err}", app_args.costume_save_file_path);
                std::process::exit(1);
            } else {
                println!("removed file: {:?}", app_args.costume_save_file_path.unwrap());
            }
        }
    }
}

fn main() {
    let args = std::env::args();
    if args.len() > 1 {
        run_command_line_util(args);
        return;
    }

    let costume_dir = std::env::var("COSTUMES_DIR").expect("COSTUMES_DIR env var not set");
    std::env::set_current_dir(&costume_dir).expect("failed to set current directory to COSTUME_DIR");
    use std::collections::HashMap;
    let mut saves: HashMap<std::ffi::OsString, CostumeSaveFile> = HashMap::new();
    for entry in std::fs::read_dir(&costume_dir).unwrap().flatten() {
        let path = entry.path();
        if let Ok(costume_save) = CostumeSaveFile::new_from_path(path.as_path()) {
            saves.insert(path.into_os_string(), costume_save);
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

    let mut selected_display: Option<std::ffi::OsString> = None;

    // NOTE If we want to write app state to disk we need to enable the "persistence" feature for
    // eframe and use eframe::run_native() instead of eframe::run_simple_native().
    // https://docs.rs/eframe/latest/eframe/
    _ = eframe::run_simple_native("Champions Costume Manager", options, move |ctx, _| {
        egui::SidePanel::right("details_display").show(ctx, |ui| {
            match selected_display.as_ref() {
                Some(save_id) => {
                    let save = saves.get(save_id).unwrap();
                    ui.label(&save.save_name);
                },
                None => {
                    ui.label("Select a save to view details");
                }
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
                for save_id in ui_save_display.iter() {
                    let save = saves.get(save_id).unwrap();
                    ui.selectable_value(
                        &mut selected_display,
                        Some(save_id.clone()),
                        match display_type {
                            DisplayType::DisplayName => save.get_in_game_display_name(),
                            DisplayType::FileName => save.get_file_name(),
                        }
                    );
                }
            });
        });
    });
}
