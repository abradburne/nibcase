//! Environment calls from the pinned API header. Unsupported interfaces return false.
use super::{callback, ffi::*, text, Context, OptionDef};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CString};

extern "C" {
    fn nibcase_core_log(level: retro_log_level, format: *const c_char, ...);
}

fn install(c: &mut Context, definitions: Vec<OptionDef>) {
    let mut options = BTreeMap::new();
    for def in definitions {
        if def.values.is_empty() {
            continue;
        }
        let existing = c.values.get(&def.key).and_then(|s| s.to_str().ok());
        let selected = c
            .overrides
            .get(&def.key)
            .map(String::as_str)
            .or(existing)
            .unwrap_or(&def.default);
        let selected = if def.values.iter().any(|v| v.0 == selected) {
            selected
        } else {
            if c.overrides.contains_key(&def.key) {
                c.error = Some(format!("invalid value {selected} for {}", def.key));
            }
            &def.default
        };
        if let Ok(value) = CString::new(selected) {
            c.set_value(def.key.clone(), value);
        }
        options.insert(def.key.clone(), def);
    }
    c.options = options;
}

unsafe fn option_values(values: &[retro_core_option_value]) -> Vec<(String, String)> {
    values
        .iter()
        .take_while(|v| !v.value.is_null())
        .map(|v| {
            // SAFETY: option arrays and their strings are core-owned per the environment contract.
            unsafe {
                let value = text(v.value);
                let label = text(v.label);
                (value.clone(), if label.is_empty() { value } else { label })
            }
        })
        .collect()
}

unsafe fn v1(c: &mut Context, ptr: *const retro_core_option_definition) -> bool {
    if ptr.is_null() {
        return false;
    }
    let mut defs = Vec::new();
    for i in 0..4096 {
        // SAFETY: the core supplies a NULL-key-terminated array per libretro.h.
        let d = unsafe { &*ptr.add(i) };
        if d.key.is_null() {
            install(c, defs);
            return true;
        }
        // SAFETY: these strings and fixed-length values are valid until this callback returns.
        unsafe {
            let values = option_values(&d.values);
            let default = text(d.default_value);
            let default = if values.iter().any(|v| v.0 == default) {
                default
            } else {
                values.first().map(|v| v.0.clone()).unwrap_or_default()
            };
            defs.push(OptionDef {
                key: text(d.key),
                label: text(d.desc),
                info: text(d.info),
                category: String::new(),
                values,
                default,
                visible: true,
            });
        }
    }
    false
}

unsafe fn v2(c: &mut Context, ptr: *const retro_core_options_v2) -> bool {
    if ptr.is_null() {
        return false;
    }
    // SAFETY: caller supplied retro_core_options_v2 and NULL-terminated arrays per libretro.h.
    unsafe {
        let options = &*ptr;
        c.categories.clear();
        if !options.categories.is_null() {
            for i in 0..256 {
                let category = &*options.categories.add(i);
                if category.key.is_null() {
                    break;
                }
                c.categories.insert(text(category.key), text(category.desc));
            }
        }
        if options.definitions.is_null() {
            return false;
        }
        let mut defs = Vec::new();
        for i in 0..4096 {
            let d = &*options.definitions.add(i);
            if d.key.is_null() {
                install(c, defs);
                return true;
            }
            let values = option_values(&d.values);
            let default = text(d.default_value);
            let default = if values.iter().any(|v| v.0 == default) {
                default
            } else {
                values.first().map(|v| v.0.clone()).unwrap_or_default()
            };
            let category = c
                .categories
                .get(&text(d.category_key))
                .cloned()
                .unwrap_or_default();
            defs.push(OptionDef {
                key: text(d.key),
                label: text(if d.desc_categorized.is_null() {
                    d.desc
                } else {
                    d.desc_categorized
                }),
                info: text(d.info),
                category,
                values,
                default,
                visible: true,
            });
        }
    }
    false
}

pub(super) unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
    callback(false, |c| {
        if command == RETRO_ENVIRONMENT_GET_INPUT_BITMASKS {
            return true;
        }
        if command == RETRO_ENVIRONMENT_SHUTDOWN {
            c.shutdown = true;
            return true;
        }
        if data.is_null() {
            return false;
        }
        // SAFETY: libretro.h specifies the type/alignment of data for each command. The core
        // owns input structs/arrays and writable output storage for this synchronous callback.
        // Every string we return is owned by the boxed context for the core's lifetime.
        unsafe {
            match command {
                RETRO_ENVIRONMENT_GET_CAN_DUPE => {
                    *data.cast::<bool>() = true;
                    true
                }
                RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY => {
                    *data.cast::<*const c_char>() = c.system_dir.as_ptr();
                    true
                }
                RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY => {
                    *data.cast::<*const c_char>() = c.save_dir.as_ptr();
                    true
                }
                RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY => {
                    *data.cast::<*const c_char>() = c.assets_dir.as_ptr();
                    true
                }
                RETRO_ENVIRONMENT_GET_LIBRETRO_PATH => {
                    *data.cast::<*const c_char>() = c.core_path.as_ptr();
                    true
                }
                RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
                    let format = *data.cast::<u32>();
                    if format > 2 {
                        return false;
                    }
                    c.format = format;
                    true
                }
                RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO => {
                    c.av = *data.cast::<retro_system_av_info>();
                    true
                }
                RETRO_ENVIRONMENT_SET_GEOMETRY => {
                    let geometry = *data.cast::<retro_game_geometry>();
                    if geometry.base_width == 0
                        || geometry.base_height == 0
                        || geometry.base_width > 8192
                        || geometry.base_height > 8192
                    {
                        return false;
                    }
                    c.av.geometry = geometry;
                    true
                }
                RETRO_ENVIRONMENT_SET_ROTATION => {
                    let rotation = *data.cast::<u32>();
                    if rotation > 3 {
                        return false;
                    }
                    c.rotation = rotation;
                    true
                }
                RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION => {
                    *data.cast::<u32>() = 2;
                    true
                }
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS => v1(c, data.cast()),
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL => {
                    v1(c, (*data.cast::<retro_core_options_intl>()).us)
                }
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2 => v2(c, data.cast()),
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL => {
                    v2(c, (*data.cast::<retro_core_options_v2_intl>()).us)
                }
                RETRO_ENVIRONMENT_SET_VARIABLES => {
                    let ptr = data.cast::<retro_variable>();
                    let mut definitions = Vec::new();
                    for i in 0..4096 {
                        let variable = &*ptr.add(i);
                        if variable.key.is_null() {
                            install(c, definitions);
                            return true;
                        }
                        let description = text(variable.value);
                        let Some((label, choices)) = description.split_once(';') else {
                            continue;
                        };
                        let values: Vec<_> = choices
                            .trim()
                            .split('|')
                            .map(|v| (v.to_owned(), v.to_owned()))
                            .collect();
                        definitions.push(OptionDef {
                            key: text(variable.key),
                            label: label.into(),
                            info: String::new(),
                            category: String::new(),
                            default: values.first().map(|v| v.0.clone()).unwrap_or_default(),
                            values,
                            visible: true,
                        });
                    }
                    false
                }
                RETRO_ENVIRONMENT_GET_VARIABLE => {
                    let var = &mut *data.cast::<retro_variable>();
                    var.value = c
                        .values
                        .get(&text(var.key))
                        .map_or(std::ptr::null(), |v| v.as_ptr());
                    !var.value.is_null()
                }
                RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
                    *data.cast::<bool>() = std::mem::take(&mut c.variables_changed);
                    true
                }
                RETRO_ENVIRONMENT_SET_VARIABLE => {
                    let var = &*data.cast::<retro_variable>();
                    let (key, value) = (text(var.key), text(var.value));
                    if c.options
                        .get(&key)
                        .is_some_and(|d| d.values.iter().any(|v| v.0 == value))
                    {
                        if let Ok(value) = CString::new(value) {
                            c.set_value(key, value);
                            c.variables_changed = true;
                            return true;
                        }
                    }
                    false
                }
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY => {
                    let display = &*data.cast::<retro_core_option_display>();
                    if let Some(option) = c.options.get_mut(&text(display.key)) {
                        option.visible = display.visible;
                        true
                    } else {
                        false
                    }
                }
                RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK => {
                    c.update_display =
                        (*data.cast::<retro_core_options_update_display_callback>()).callback;
                    true
                }
                RETRO_ENVIRONMENT_GET_LOG_INTERFACE => {
                    (*data.cast::<retro_log_callback>()).log = Some(nibcase_core_log);
                    true
                }
                RETRO_ENVIRONMENT_SET_MESSAGE => {
                    c.messages.push(text((*data.cast::<retro_message>()).msg));
                    true
                }
                RETRO_ENVIRONMENT_SET_MESSAGE_EXT => {
                    c.messages
                        .push(text((*data.cast::<retro_message_ext>()).msg));
                    true
                }
                RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION => {
                    *data.cast::<u32>() = 1;
                    true
                }
                RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS => {
                    c.quirks = *data.cast::<u64>();
                    true
                }
                RETRO_ENVIRONMENT_GET_SAVESTATE_CONTEXT => {
                    *data.cast::<u32>() = retro_savestate_context_RETRO_SAVESTATE_CONTEXT_NORMAL;
                    true
                }
                RETRO_ENVIRONMENT_SET_FRAME_TIME_CALLBACK => {
                    c.frame_time = *data.cast::<retro_frame_time_callback>();
                    true
                }
                RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK => {
                    c.audio_status = *data.cast::<retro_audio_buffer_status_callback>();
                    true
                }
                RETRO_ENVIRONMENT_GET_FASTFORWARDING => {
                    *data.cast::<bool>() = c.fast_forward;
                    true
                }
                RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE => {
                    *data.cast::<i32>() = 3;
                    true
                }
                RETRO_ENVIRONMENT_GET_TARGET_REFRESH_RATE => {
                    *data.cast::<f32>() = 59.94;
                    true
                }
                RETRO_ENVIRONMENT_GET_THROTTLE_STATE => {
                    let throttle = &mut *data.cast::<retro_throttle_state>();
                    throttle.mode = if c.fast_forward { 3 } else { 0 };
                    throttle.rate = if c.fast_forward {
                        2.0
                    } else {
                        c.av.timing.fps as f32
                    };
                    true
                }
                // Games of several discs (PlayStation): the extended interface adds labels.
                RETRO_ENVIRONMENT_GET_DISK_CONTROL_INTERFACE_VERSION => {
                    *data.cast::<u32>() = 1;
                    true
                }
                RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE => {
                    c.disk = Some(*data.cast::<retro_disk_control_ext_callback>());
                    true
                }
                RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE => {
                    let d = *data.cast::<retro_disk_control_callback>();
                    c.disk = Some(retro_disk_control_ext_callback {
                        set_eject_state: d.set_eject_state,
                        get_eject_state: d.get_eject_state,
                        get_image_index: d.get_image_index,
                        set_image_index: d.set_image_index,
                        get_num_images: d.get_num_images,
                        replace_image_index: d.replace_image_index,
                        add_image_index: d.add_image_index,
                        ..Default::default()
                    });
                    true
                }
                RETRO_ENVIRONMENT_GET_LANGUAGE => {
                    *data.cast::<u32>() = retro_language_RETRO_LANGUAGE_ENGLISH;
                    true
                }
                // The first player's joypad, for the menu's Controls rows; other ports and
                // devices aren't shown.
                RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS => {
                    c.inputs.clear();
                    let list = data.cast::<retro_input_descriptor>();
                    for i in 0..256 {
                        // A core-owned array ending with a NULL description, per libretro.h;
                        // read only during this call (this whole handler is one unsafe block).
                        let d = &*list.add(i);
                        if d.description.is_null() {
                            break;
                        }
                        if d.port == 0 && d.device == RETRO_DEVICE_JOYPAD && d.index == 0 {
                            c.inputs.push((d.id, text(d.description)));
                        }
                    }
                    true
                }
                // The pads the first port can take (an array of ports ending in an empty one).
                RETRO_ENVIRONMENT_SET_CONTROLLER_INFO => {
                    let ports = data.cast::<retro_controller_info>();
                    c.controllers.clear();
                    let first = &*ports;
                    if !first.types.is_null() {
                        for i in 0..first.num_types.min(64) as usize {
                            let t = &*first.types.add(i);
                            if !t.desc.is_null() {
                                c.controllers.push((text(t.desc), t.id));
                            }
                        }
                    }
                    true
                }
                // Metadata accepted and not used.
                RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL | RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME => {
                    true
                }
                _ => {
                    if c.unknown.insert(command) {
                        eprintln!("nibcase: unsupported environment call {command}");
                    }
                    false
                }
            }
        }
    })
}
