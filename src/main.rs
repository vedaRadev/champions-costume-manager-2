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

// TODO store the filename and timestamp separately, can probably get rid of the path (unless we
// want to store the path to the file's parent directory). Then will need to update the logic for
// changing file name and stripping timestamp.
struct CostumeSaveFile {
    // TODO path should probably be stored as a PathBuf because we need to allow editing file names.
    path: String, // TODO Should this be a cow?
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
    /// New file name to set excluding the "Costume_" prefix and j2000 timestamp postfix.
    new_file_name: Option<String>,
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
    // TODO Should this be a "commit" (i.e. must include option to save changes) instead of a
    // "dry-run" (i.e. include to NOT save changes)?
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

            // TODO Should we allow for setting account names to empty strings?
            "--set-account-name" | "-a" => {
                // TODO Do we really need to guard against this?
                if app_args.new_account_name.is_some() {
                    eprintln!("Multiple account names specified");
                    std::process::exit(1);
                }

                app_args.new_account_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected account name");
                    std::process::exit(1);
                });
            },

            // TODO Should we allow for setting character names to empty strings?
            "--set-character-name" | "-c" => {
                // TODO Do we really need to guard against this?
                if app_args.new_character_name.is_some() {
                    eprintln!("Multiple character names specified");
                    std::process::exit(1);
                }

                app_args.new_character_name = raw_args.next().or_else(|| {
                    eprintln!("Unexpected end of input stream, expected character name");
                    std::process::exit(1);
                });
            },

            // TODO Maybe there's a better name to use here since we're not setting the FULL file
            // name, just the part between "Costume_" and the timestamp (if there is one). Maybe
            // documenting in the help string is enough.
            "--set-file-name" | "-f" => {
                // TODO Do we really need to guard against this?
                if app_args.new_file_name.is_some() {
                    eprintln!("Multiple file renames specified");
                    std::process::exit(1);
                }

                app_args.new_file_name = raw_args.next().or_else(|| {
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

                if !app_args.costume_save_file_path.is_empty() {
                    eprintln!("Multiple files specified");
                    std::process::exit(1);
                }

                app_args.costume_save_file_path = arg;
            },
        }
    }

    if app_args.costume_save_file_path.is_empty() {
        eprintln!("Costume save file path is required");
        std::process::exit(1);
    }

    let jpeg_raw = std::fs::read(&app_args.costume_save_file_path).unwrap_or_else(|err| {
        eprintln!("Failed to read costume jpeg: {}", err);
        std::process::exit(1);
    });
    // FIXME Jpeg unpacking should return a result, not just panic (see jpeg implementation)
    let costume_jpeg = Jpeg::unpack(jpeg_raw);
    let mut costume_save = CostumeSaveFile {
        // FIXME unnecessary clone if user doesn't modify file name (should path be a cow string?)
        // Maybe this one clone is fine... (piggy time)
        path: app_args.costume_save_file_path.clone(),
        jpeg: costume_jpeg,
    };

    let mut dirty = false;
    if let Some(new_account_name) = app_args.new_account_name {
        costume_save.set_account_name(new_account_name);
        dirty = true;
    }
    if let Some(new_character_name) = app_args.new_character_name {
        costume_save.set_character_name(new_character_name);
        dirty = true;
    }
    if let Some(new_file_name) = app_args.new_file_name {
        let j2000_timestamp = costume_save.get_j2000_timestamp();
        let mut new_path = std::path::PathBuf::from(costume_save.path);
        if let Some(j2000_timestamp) = j2000_timestamp {
            new_path.set_file_name(format!("Costume_{new_file_name}_{j2000_timestamp}"));
        } else {
            new_path.set_file_name(format!("Costume_{new_file_name}"));
        }
        new_path.set_extension("jpg");
        costume_save.path = new_path.into_os_string().into_string().unwrap();
        dirty = true;
    }
    if app_args.should_strip_timestamp {
        if costume_save.get_j2000_timestamp().is_some() {
            // TODO Update CostumeSaveFile to store the path as a PathBuf and the pain of all this
            // converting between str and String and OsStr goes away (at least a little)...
            let mut new_path = std::path::PathBuf::from(costume_save.path);
            // SAFETY: at this point we should have asserted that path has a valid file name.
            let old_file_name = new_path.file_name().unwrap().to_str().unwrap();
            // SAFETY: get_j2000_timestamp should only return a Some value if the last part of the
            // file name is "_<j2000 timestamp>".
            // FIXME: We lose the extension here! Is that okay if we just replace it later, or
            // should we be more sophisticated in excising the timestamp?
            let new_file_name = old_file_name.split_at(old_file_name.rfind('_').unwrap()).0.to_owned();
            new_path.set_file_name(new_file_name);
            // FIXME hardcoded, though we should never get this far if the file didn't have a .jpg
            // extension in the first place.
            new_path.set_extension("jpg");
            costume_save.path = new_path.into_os_string().into_string().unwrap();
            dirty = true;
        } else {
            println!("WARNING: --strip-timestamp was specified but there is no value j2000 timestamp to strip from the filename");
        }
    }

    if let Some(inspect_type) = app_args.inspect_type {
        let account_name = costume_save.get_account_name();
        let character_name = costume_save.get_character_name();
        let in_game_display = costume_save.get_in_game_display_name();
        println!("File: {}", costume_save.path);
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
        let packed_data = costume_save.jpeg.pack();
        println!();
        if let Err(err) = std::fs::write(&costume_save.path, packed_data) {
            eprintln!("failed to write file {}: {}", costume_save.path, err);
            std::process::exit(1);
        } else {
            println!("wrote file {}", costume_save.path);
        }
        // TODO Copy over file creation time, set file updated time to now. Note that setting the
        // file creation time is only available on windows!
        // NOTE: This is only valid so long as the costume save stores the ENTIRE path, not just
        // the name of the file!
        if costume_save.path != app_args.costume_save_file_path {
            if let Err(err) = std::fs::remove_file(&app_args.costume_save_file_path) {
                eprintln!("failed to remove original file {}: {}", app_args.costume_save_file_path, err);
                std::process::exit(1);
            } else {
                println!("removed file: {}", app_args.costume_save_file_path);
            }
        }
    }
}
