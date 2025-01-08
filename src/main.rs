// NOTE I'm not sure if I like how the structure of the jpeg/costume stuff is turning out. From a
// pure workflow standpoint, the whole point is to decode (unpack) a jpeg, hold it in memory,
// update some of the app13 metadata, then save (repack) it to disk by overwriting the previous
// file. HOWEVER we also will also have to display the jpeg image itself in the gui, so do we hold
// a raw copy of the jpeg AND the decoded jpeg in memory at the same time (I think this is
// essentially what would happen if we decode the jpeg ourselves AND use a 3rd party lib to display
// the image from the file [e.g. egui image widget] -- though the 3rd party lib might just decode
// the image then send it to the GPU, idk what I'm talking about here).

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

struct CostumeSaveFile {
    path: String,
    jpeg: Jpeg,
}

#[allow(dead_code)]
// TODO constructor that returns a result, maybe just take the file path and unpack from that.
impl CostumeSaveFile {
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

    fn get_j2000_timestamp(&self) -> Option<i64> {
        std::path::Path::new(&self.path)
            .file_stem().unwrap()
            .to_str().unwrap()
            .split('_')
            .last().unwrap()
            .parse::<i64>().ok()
    }

    // FIXME The max date that the game can display is 2068-01-19 03:14:07 but we don't handle this
    // edge case. Our simulated in-game display name datestring will go (almost) arbitrarily high.
    fn get_in_game_display_name(&self) -> String {
        let account_name = self.get_account_name();
        let character_name = self.get_character_name();
        let maybe_datetime_string = self.get_j2000_timestamp().and_then(|j2000_timestamp| {
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
}

enum InspectType { Short, Long }

#[derive(Default)]
struct AppArgs {
    /// Required. The file path of the costume save.
    costume_save_file_path: String,
    /// New account name to set in costume jpeg metadata.
    new_account_name: Option<String>,
    /// New character name to set in costume jpeg metadata.
    new_character_name: Option<String>,
    /// Whether or not to strip the J2000 timestamp from the end of the filename.
    should_strip_timestamp: bool,
    /// If costume metadata and in-game save display should be displayed. Defaults to short
    /// inspection, which does not include costume hash and costume specification. To specify long
    /// inspection, use "--inspect long". If specified with with mutative options such as
    /// --set-character-name or --strip-timestamp, the mutations are applied first then the
    /// metadata is printed.
    inspect_type: Option<InspectType>,
    /// Should mutative options be ignored? Really only useful for seeing how potential changes
    /// will cause the save to appear in-game.
    dry_run: bool,
}

fn main() {
    let mut raw_args = std::env::args().skip(1).peekable();
    let mut app_args: AppArgs = AppArgs::default();
    while let Some(arg) = raw_args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                todo!();
            },

            "--set-account-name" | "-a" => {
                if app_args.new_account_name.is_some() {
                    eprintln!("Multiple account names specified");
                    std::process::exit(1);
                }

                app_args.new_account_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected account name");
                    std::process::exit(1);
                });
            },

            "--set-character-name" | "-c" => {
                if app_args.new_character_name.is_some() {
                    eprintln!("Multiple character names specified");
                    std::process::exit(1);
                }

                app_args.new_character_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected character name");
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

                if !app_args.costume_save_file_path.is_empty() {
                    eprintln!("Multiple files specified");
                    std::process::exit(1);
                }

                app_args.costume_save_file_path = arg;
            },
        }
    }

    let jpeg_raw = std::fs::read(&app_args.costume_save_file_path).unwrap_or_else(|err| {
        eprintln!("Failed to read costume jpeg: {}", err);
        std::process::exit(1);
    });
    // FIXME Jpeg unpacking should return a result, not just panic (see jpeg implementation)
    let costume_jpeg = Jpeg::unpack(jpeg_raw);
    let mut costume_save = CostumeSaveFile {
        path: app_args.costume_save_file_path,
        jpeg: costume_jpeg,
    };

    if let Some(new_account_name) = app_args.new_account_name {
        costume_save.set_account_name(new_account_name);
    }
    if let Some(new_character_name) = app_args.new_character_name {
        costume_save.set_character_name(new_character_name);
    }
    if app_args.should_strip_timestamp {
        // TODO
        // Need to keep track of the original file name because we'll need to do one of the
        // following:
        // A) delete the original file then save a new file without the timestamp
        // B) rename the original file and exclude the timestamp
        //
        // Maybe updating the name of the file itself should be part of the CostumeSaveFile impl?
    }

    if let Some(inspect_type) = app_args.inspect_type {
        let account_name = costume_save.get_account_name();
        let character_name = costume_save.get_character_name();
        let in_game_display = costume_save.get_in_game_display_name();
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

    if !app_args.dry_run {
        // TODO repack jpeg and update file
    }
}
