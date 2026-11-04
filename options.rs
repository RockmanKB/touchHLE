/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Parsing and management of user-configurable options, e.g. for input methods.

use crate::gles::GLESImplementation;
use crate::window::{DeviceFamily, DeviceOrientation};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, ToSocketAddrs};
use std::num::NonZeroU32;
use std::path::PathBuf;

pub const OPTIONS_HELP: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/OPTIONS_HELP.txt"));

/// Game controller button for `--button-to-touch=` option.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub enum Button {
    DPadLeft,
    DPadUp,
    DPadRight,
    DPadDown,
    Start,
    A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    Back,
    LeftStick,
    RightStick,
    /// Local: analog triggers (LT/L2, RT/R2), used as buttons.
    LeftTrigger,
    RightTrigger,
}

/// Local: parse a controller button name as used by `--button-to-touch=`.
pub fn parse_button_name(name: &str) -> Result<Button, String> {
    Ok(match name {
        "DPadLeft" => Button::DPadLeft,
        "DPadUp" => Button::DPadUp,
        "DPadRight" => Button::DPadRight,
        "DPadDown" => Button::DPadDown,
        "Start" => Button::Start,
        "A" => Button::A,
        "B" => Button::B,
        "X" => Button::X,
        "Y" => Button::Y,
        "LeftShoulder" => Button::LeftShoulder,
        "RightShoulder" => Button::RightShoulder,
        "Back" => Button::Back,
        "LeftStick" => Button::LeftStick,
        "RightStick" => Button::RightStick,
        "LeftTrigger" => Button::LeftTrigger,
        "RightTrigger" => Button::RightTrigger,
        _ => return Err(format!("Invalid button name {name:?}")),
    })
}

/// Local: a screen-dependent tap (see `--context-button=`/`--context-key=`).
#[derive(Clone, Debug)]
pub struct ContextTap {
    pub contexts: Vec<u32>,
    pub button: Option<Button>,
    pub key: Option<String>,
    pub x: f32,
    pub y: f32,
}

/// Local: a screen where directional input steps through choices.
#[derive(Clone, Debug)]
pub struct ContextMenu {
    pub contexts: Vec<u32>,
    pub items: Vec<(f32, f32)>,
    /// If true, directional input only moves an on-screen pointer and the
    /// confirm input taps; if false, each step taps (for menus that
    /// highlight on the first tap and act on the second).
    pub pointer: bool,
}

/// Local: a scrolling list of rows (e.g. a settings screen). The first tap
/// on a row selects it; tapping the selected row changes its value.
#[derive(Debug, Clone)]
pub struct ListMenu {
    pub contexts: Vec<u32>,
    pub x: f32,
    pub y0: f32,
    pub step: f32,
    pub w: f32,
    pub h: f32,
    pub visible: usize,
    pub total: usize,
    pub up: (f32, f32),
    pub down: (f32, f32),
    pub back: (f32, f32),
}

fn parse_contexts(s: &str) -> Result<Vec<u32>, String> {
    s.split('|')
        .map(|v| {
            let v = v.trim();
            match v.strip_prefix("0x") {
                Some(h) => u32::from_str_radix(h, 16),
                None => v.parse(),
            }
            .map_err(|_| format!("Invalid screen value {v:?}"))
        })
        .collect()
}

/// Local: GameKit peer-to-peer link between two touchHLE instances.
#[derive(Clone, Debug)]
pub enum GameKitLink {
    Host(u16),
    Join(SocketAddr),
}

/// Struct containing all user-configurable options.
#[derive(Clone)]
pub struct Options {
    pub fullscreen: bool,
    pub device_family: Option<DeviceFamily>,
    pub initial_orientation: DeviceOrientation,
    pub scale_hack: NonZeroU32,
    pub deadzone: f32,
    pub analog_stick_tilt_controls: bool,
    pub x_tilt_range: f32,
    pub y_tilt_range: f32,
    pub x_tilt_offset: f32,
    pub y_tilt_offset: f32,
    pub button_to_touch: HashMap<Button, (f32, f32)>,
    pub dpad_to_touch: Option<(f32, f32, f32, f32)>,
    pub stick_to_touch: Option<(f32, f32, f32, f32)>,
    pub stabilize_virtual_cursor: Option<(f32, f32)>,
    pub gles1_implementation: Option<GLESImplementation>,
    pub direct_memory_access: bool,
    pub gdb_listen_addrs: Option<Vec<SocketAddr>>,
    pub preferred_languages: Option<Vec<String>>,
    pub headless: bool,
    pub print_fps: bool,
    pub fps_limit: Option<f64>,
    pub force_composition: bool,
    pub network_access: bool,
    pub popup_errors: bool,
    pub dumping_options: DumpingOptions,
    pub dumping_file: PathBuf,
    pub ignore_gl_errors: bool,
    pub zero_stack_after_guest_to_host_call: Option<u32>,
    pub gamekit_link: Option<GameKitLink>,
    pub controller_index: Option<u32>,
    pub context_probe: Option<(u32, Vec<u32>)>,
    pub context_taps: Vec<ContextTap>,
    pub context_menus: Vec<ContextMenu>,
    pub list_menus: Vec<ListMenu>,
    /// Local: where a list menu's state is in app memory, as offsets from
    /// the context probe base: (screens, scroll px (<= 0), max scroll, selected row).
    pub list_states: Vec<(Vec<u32>, u32, u32, u32)>,
    /// Local: classic combos. Left/right + the button at this spot becomes
    /// up + that button.
    pub classic_headbutt: Option<(f32, f32)>,
    /// Local: classic combos. Pressing the buttons at the first two spots
    /// together becomes down + the button at the third spot.
    pub classic_elbow: Option<[(f32, f32); 3]>,
    pub menu_confirm_buttons: Vec<Button>,
    pub menu_confirm_keys: Vec<String>,
    pub touch_plates: bool,
    pub window_scale: f32,
    pub stall_dumps: Vec<(u32, u32, u32, u32)>,
    /// Local: key that shows/hides the window's border and title bar.
    pub border_toggle_key: Option<String>,
    /// Local: start without the window border.
    pub borderless: bool,
    pub patches_u32: Vec<(u32, u32, u32)>,
    pub extra_plates: Vec<(Vec<u32>, Vec<[f32; 4]>)>,
    pub menu_back_buttons: Vec<Button>,
    pub menu_back_keys: Vec<String>,
    pub plate_nav_exclude: Vec<u32>,
    pub key_to_touch: Vec<(String, f32, f32)>,
    pub keys_to_dpad: Option<[String; 4]>,
    pub device_id: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            fullscreen: false,
            device_family: None,
            initial_orientation: DeviceOrientation::Portrait,
            scale_hack: NonZeroU32::new(1).unwrap(),
            analog_stick_tilt_controls: true,
            deadzone: 0.1,
            x_tilt_range: 60.0,
            y_tilt_range: 60.0,
            x_tilt_offset: 0.0,
            y_tilt_offset: 0.0,
            button_to_touch: HashMap::new(),
            dpad_to_touch: None,
            stick_to_touch: None,
            stabilize_virtual_cursor: None,
            gles1_implementation: None,
            direct_memory_access: true,
            gdb_listen_addrs: None,
            preferred_languages: None,
            headless: false,
            print_fps: false,
            fps_limit: Some(60.0), // Original iPhone is 60Hz and uses v-sync,
            force_composition: false,
            network_access: false,
            popup_errors: true,
            dumping_options: Default::default(),
            dumping_file: crate::paths::user_data_base_path().join("DUMP.txt"),
            ignore_gl_errors: false,
            zero_stack_after_guest_to_host_call: None,
            gamekit_link: None,
            controller_index: None,
            context_probe: None,
            context_taps: Vec::new(),
            context_menus: Vec::new(),
            list_menus: Vec::new(),
            list_states: Vec::new(),
            classic_headbutt: None,
            classic_elbow: None,
            menu_confirm_buttons: Vec::new(),
            menu_confirm_keys: Vec::new(),
            touch_plates: false,
            window_scale: 1.0,
            stall_dumps: Vec::new(),
            border_toggle_key: None,
            borderless: false,
            patches_u32: Vec::new(),
            extra_plates: Vec::new(),
            menu_back_buttons: Vec::new(),
            menu_back_keys: Vec::new(),
            plate_nav_exclude: Vec::new(),
            key_to_touch: Vec::new(),
            keys_to_dpad: None,
            device_id: None,
        }
    }
}

impl Options {
    /// Parse the command-line argument syntax for an option. Returns `Ok(true)`
    /// if the option was valid and has been applied, or `Ok(false)` if the
    /// option was not recognized.
    pub fn parse_argument(&mut self, arg: &str) -> Result<bool, String> {
        fn parse_degrees(arg: &str, name: &str) -> Result<f32, String> {
            let arg: f32 = arg
                .parse()
                .map_err(|_| format!("Value for {name} is invalid"))?;
            if !arg.is_finite() || !(-360.0..=360.0).contains(&arg) {
                return Err(format!("Value for {name} is out of range"));
            }
            Ok(arg)
        }

        if let Some(value) = arg.strip_prefix("--gamekit-host=") {
            let port: u16 = value
                .parse()
                .map_err(|_| "--gamekit-host= requires a port number".to_string())?;
            self.gamekit_link = Some(GameKitLink::Host(port));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--gamekit-join=") {
            use std::net::ToSocketAddrs;
            let addr = value
                .to_socket_addrs()
                .map_err(|_| "--gamekit-join= requires ADDRESS:PORT".to_string())?
                .next()
                .ok_or_else(|| "--gamekit-join= requires ADDRESS:PORT".to_string())?;
            self.gamekit_link = Some(GameKitLink::Join(addr));
            if self.device_id.is_none() {
                self.device_id = Some("touchHLEdevice2".to_string());
            }
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--controller=") {
            let n: u32 = value
                .parse()
                .ok()
                .filter(|&n| n >= 1)
                .ok_or_else(|| "--controller= requires a number (1, 2, ...)".to_string())?;
            self.controller_index = Some(n);
            return Ok(true);
        } else if let Some(values) = arg.strip_prefix("--key-to-touch=") {
            // Key names may contain commas only at the start (e.g. ",") so
            // split from the right.
            let mut parts = values.rsplitn(3, ',');
            let y = parts.next();
            let x = parts.next();
            let key = parts.next();
            let (Some(key), Some(x), Some(y)) = (key, x, y) else {
                return Err("--key-to-touch= requires KEY,X,Y".to_string());
            };
            let x: f32 = x.parse().map_err(|_| "--key-to-touch= X is invalid".to_string())?;
            let y: f32 = y.parse().map_err(|_| "--key-to-touch= Y is invalid".to_string())?;
            self.key_to_touch.push((key.to_string(), x, y));
            return Ok(true);
        } else if let Some(values) = arg.strip_prefix("--keys-to-dpad=") {
            let parts: Vec<&str> = values.split(',').collect();
            let [u, d, l, r] = parts[..] else {
                return Err("--keys-to-dpad= requires UP,DOWN,LEFT,RIGHT".to_string());
            };
            self.keys_to_dpad = Some([u.into(), d.into(), l.into(), r.into()]);
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-probe=") {
            fn num(s: &str) -> Result<u32, String> {
                let s = s.trim();
                if let Some(h) = s.strip_prefix("0x") {
                    u32::from_str_radix(h, 16).map_err(|_| format!("bad number {s}"))
                } else {
                    s.parse().map_err(|_| format!("bad number {s}"))
                }
            }
            let mut parts = value.split(',');
            let ptr = num(parts.next().unwrap_or(""))?;
            let offs = parts.map(num).collect::<Result<Vec<_>, _>>()?;
            self.context_probe = Some((ptr, offs));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-button=") {
            let p: Vec<&str> = value.split(',').collect();
            let [ctx, b, x, y] = p[..] else {
                return Err("--context-button= requires SCREEN,BUTTON,X,Y".to_string());
            };
            self.context_taps.push(ContextTap {
                contexts: parse_contexts(ctx)?,
                button: Some(parse_button_name(b)?),
                key: None,
                x: x.parse().map_err(|_| "--context-button= X is invalid".to_string())?,
                y: y.parse().map_err(|_| "--context-button= Y is invalid".to_string())?,
            });
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-key=") {
            let (ctx, rest) = value
                .split_once(',')
                .ok_or_else(|| "--context-key= requires SCREEN,KEY,X,Y".to_string())?;
            let mut parts = rest.rsplitn(3, ',');
            let (Some(y), Some(x), Some(key)) = (parts.next(), parts.next(), parts.next()) else {
                return Err("--context-key= requires SCREEN,KEY,X,Y".to_string());
            };
            self.context_taps.push(ContextTap {
                contexts: parse_contexts(ctx)?,
                button: None,
                key: Some(key.to_string()),
                x: x.parse().map_err(|_| "--context-key= X is invalid".to_string())?,
                y: y.parse().map_err(|_| "--context-key= Y is invalid".to_string())?,
            });
            return Ok(true);
        } else if let Some((pointer, value)) = arg
            .strip_prefix("--context-menu=")
            .map(|v| (false, v))
            .or_else(|| arg.strip_prefix("--context-pointer-menu=").map(|v| (true, v)))
        {
            let (ctx, rest) = value
                .split_once(',')
                .ok_or_else(|| "--context-menu= requires SCREEN,X1,Y1,...".to_string())?;
            let nums = rest
                .split(',')
                .map(|n| n.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "--context-menu= coordinates are invalid".to_string())?;
            if nums.is_empty() || nums.len() % 2 != 0 {
                return Err("--context-menu= requires X,Y pairs".to_string());
            }
            self.context_menus.push(ContextMenu {
                contexts: parse_contexts(ctx)?,
                items: nums.chunks(2).map(|c| (c[0], c[1])).collect(),
                pointer,
            });
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--menu-confirm-button=") {
            self.menu_confirm_buttons.push(parse_button_name(value)?);
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--menu-back-button=") {
            self.menu_back_buttons.push(parse_button_name(value)?);
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--menu-back-key=") {
            self.menu_back_keys.push(value.to_string());
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--classic-headbutt=") {
            let n = value
                .split(',')
                .map(|n| n.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "--classic-headbutt= requires X,Y".to_string())?;
            if n.len() != 2 {
                return Err("--classic-headbutt= requires X,Y".to_string());
            }
            self.classic_headbutt = Some((n[0], n[1]));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--classic-elbow=") {
            let n = value
                .split(',')
                .map(|n| n.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "--classic-elbow= requires X1,Y1,X2,Y2,X3,Y3".to_string())?;
            if n.len() != 6 {
                return Err("--classic-elbow= requires X1,Y1,X2,Y2,X3,Y3".to_string());
            }
            self.classic_elbow = Some([(n[0], n[1]), (n[2], n[3]), (n[4], n[5])]);
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-list-state=") {
            // SCREEN,SCROLL_OFFSET,MAX_SCROLL_OFFSET,SELECTED_OFFSET
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() != 4 {
                return Err("--context-list-state= requires SCREEN,SCROLL,MAX,SELECTED".to_string());
            }
            let n = |s: &str| -> Result<u32, String> {
                let s = s.trim();
                if let Some(h) = s.strip_prefix("0x") {
                    u32::from_str_radix(h, 16)
                } else {
                    s.parse()
                }
                .map_err(|_| "--context-list-state= numbers are invalid".to_string())
            };
            self.list_states
                .push((parse_contexts(parts[0])?, n(parts[1])?, n(parts[2])?, n(parts[3])?));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-list-menu=") {
            // SCREEN,X,Y0,STEP,W,H,VISIBLE,TOTAL,UPX,UPY,DOWNX,DOWNY,BACKX,BACKY
            let (ctx, rest) = value
                .split_once(',')
                .ok_or_else(|| "--context-list-menu= requires SCREEN,...".to_string())?;
            let n = rest
                .split(',')
                .map(|n| n.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "--context-list-menu= numbers are invalid".to_string())?;
            if n.len() != 13 || n[5] < 1.0 || n[6] < 1.0 {
                return Err("--context-list-menu= requires 13 numbers".to_string());
            }
            self.list_menus.push(ListMenu {
                contexts: parse_contexts(ctx)?,
                x: n[0],
                y0: n[1],
                step: n[2],
                w: n[3],
                h: n[4],
                visible: n[5] as usize,
                total: n[6] as usize,
                up: (n[7], n[8]),
                down: (n[9], n[10]),
                back: (n[11], n[12]),
            });
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--context-extra-buttons=") {
            let (ctx, rest) = value
                .split_once(',')
                .ok_or_else(|| "--context-extra-buttons= requires SCREEN,X,Y,W,H,...".to_string())?;
            let nums = rest
                .split(',')
                .map(|n| n.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "--context-extra-buttons= numbers are invalid".to_string())?;
            if nums.is_empty() || nums.len() % 4 != 0 {
                return Err("--context-extra-buttons= requires X,Y,W,H groups".to_string());
            }
            self.extra_plates.push((
                parse_contexts(ctx)?,
                nums.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(),
            ));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--stall-dump=") {
            // SCREEN,SECONDS,OFFSET,LENGTH: if the app stays on SCREEN for
            // SECONDS, log LENGTH bytes at probe base + OFFSET (diagnostics).
            let n = |s: &str| -> Result<u32, String> {
                let s = s.trim();
                match s.strip_prefix("0x") {
                    Some(h) => u32::from_str_radix(h, 16),
                    None => s.parse(),
                }
                .map_err(|_| format!("--stall-dump=: bad number {s:?}"))
            };
            let p: Vec<&str> = value.split(',').collect();
            let [c, t, o, l] = p[..] else {
                return Err("--stall-dump= requires SCREEN,SECONDS,OFFSET,LENGTH".to_string());
            };
            self.stall_dumps.push((n(c)?, n(t)?, n(o)?, n(l)?.min(256)));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--patch-u32=") {
            let n = |s: &str| -> Result<u32, String> {
                let s = s.trim();
                match s.strip_prefix("0x") {
                    Some(h) => u32::from_str_radix(h, 16),
                    None => s.parse(),
                }
                .map_err(|_| format!("--patch-u32=: bad number {s:?}"))
            };
            let p: Vec<&str> = value.split(',').collect();
            let [a, e, v] = p[..] else {
                return Err("--patch-u32= requires ADDRESS,EXPECTED,NEW".to_string());
            };
            self.patches_u32.push((n(a)?, n(e)?, n(v)?));
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--window-scale=") {
            let v: f32 = value
                .parse()
                .ok()
                .filter(|v: &f32| v.is_finite() && (0.5..=8.0).contains(v))
                .ok_or_else(|| "--window-scale= requires a number from 0.5 to 8".to_string())?;
            self.window_scale = v;
            return Ok(true);
        } else if arg == "--touch-plate-nav" {
            self.touch_plates = true;
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--plate-nav-exclude=") {
            self.plate_nav_exclude.extend(parse_contexts(value)?);
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--menu-confirm-key=") {
            self.menu_confirm_keys.push(value.to_string());
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--sandbox=") {
            let _ = crate::paths::SANDBOX_DIR_OVERRIDE.set(value.to_string());
            return Ok(true);
        } else if let Some(value) = arg.strip_prefix("--device-id=") {
            self.device_id = Some(value.to_string());
            return Ok(true);
        }

        if let Some(v) = arg.strip_prefix("--border-toggle-key=") {
            self.border_toggle_key = Some(v.to_string());
        } else if arg == "--borderless" {
            self.borderless = true;
        } else if arg == "--fullscreen" {
            self.fullscreen = true;
        } else if arg == "--upside-down" {
            self.initial_orientation = DeviceOrientation::PortraitUpsideDown;
        } else if arg == "--landscape-left" {
            self.initial_orientation = DeviceOrientation::LandscapeLeft;
        } else if arg == "--landscape-right" {
            self.initial_orientation = DeviceOrientation::LandscapeRight;
        } else if let Some(value) = arg.strip_prefix("--device-family=") {
            let parsed =
                DeviceFamily::try_from(value).map_err(|_| "Invalid device family".to_string())?;
            self.device_family = Some(parsed);
        } else if let Some(value) = arg.strip_prefix("--scale-hack=") {
            self.scale_hack = value
                .parse()
                .map_err(|_| "Invalid scale hack factor".to_string())?;
        } else if arg == "--disable-analog-stick-tilt-controls" {
            self.analog_stick_tilt_controls = false;
        } else if let Some(value) = arg.strip_prefix("--deadzone=") {
            self.deadzone = parse_degrees(value, "deadzone")?;
        } else if let Some(value) = arg.strip_prefix("--x-tilt-range=") {
            self.x_tilt_range = parse_degrees(value, "X tilt range")?;
        } else if let Some(value) = arg.strip_prefix("--y-tilt-range=") {
            self.y_tilt_range = parse_degrees(value, "Y tilt range")?;
        } else if let Some(value) = arg.strip_prefix("--x-tilt-offset=") {
            self.x_tilt_offset = parse_degrees(value, "X tilt offset")?;
        } else if let Some(value) = arg.strip_prefix("--y-tilt-offset=") {
            self.y_tilt_offset = parse_degrees(value, "Y tilt offset")?;
        } else if let Some(values) = arg.strip_prefix("--button-to-touch=") {
            let (button, coords) = values
                .split_once(',')
                .ok_or_else(|| "--button-to-touch= requires three values".to_string())?;
            let (x, y) = coords
                .split_once(',')
                .ok_or_else(|| "--button-to-touch= requires three values".to_string())?;
            let button = match button {
                "DPadLeft" => Ok(Button::DPadLeft),
                "DPadUp" => Ok(Button::DPadUp),
                "DPadRight" => Ok(Button::DPadRight),
                "DPadDown" => Ok(Button::DPadDown),
                "Start" => Ok(Button::Start),
                "A" => Ok(Button::A),
                "B" => Ok(Button::B),
                "X" => Ok(Button::X),
                "Y" => Ok(Button::Y),
                "LeftShoulder" => Ok(Button::LeftShoulder),
                "RightShoulder" => Ok(Button::RightShoulder),
                "Back" => Ok(Button::Back),
                "LeftStick" => Ok(Button::LeftStick),
                "RightStick" => Ok(Button::RightStick),
                "LeftTrigger" => Ok(Button::LeftTrigger),
                "RightTrigger" => Ok(Button::RightTrigger),
                _ => Err("Invalid button for --button-to-touch=".to_string()),
            }?;
            let x: f32 = x
                .parse()
                .map_err(|_| "Invalid X co-ordinate for --button-to-touch=".to_string())?;
            let y: f32 = y
                .parse()
                .map_err(|_| "Invalid Y co-ordinate for --button-to-touch=".to_string())?;
            self.button_to_touch.insert(button, (x, y));
        } else if let Some(values) = arg.strip_prefix("--stick-to-touch=") {
            let nums: [f32; 4] = values
                .split(',')
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid --stick-to-touch".to_string())?
                .try_into()
                .map_err(|_| "--stick-to-touch= requires four values".to_string())?;

            self.stick_to_touch = Some((nums[0], nums[1], nums[2], nums[3]));
        } else if let Some(values) = arg.strip_prefix("--dpad-to-touch=") {
            let nums: [f32; 4] = values
                .split(',')
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid --dpad-to-touch".to_string())?
                .try_into()
                .map_err(|_| "--dpad-to-touch= requires four values".to_string())?;

            self.dpad_to_touch = Some((nums[0], nums[1], nums[2], nums[3]));
        } else if let Some(value) = arg.strip_prefix("--stabilize-virtual-cursor=") {
            let (smoothing_strength, sticky_radius) = value
                .split_once(',')
                .ok_or_else(|| "--stabilize-virtual-cursor= requires two values".to_string())?;
            let smoothing_strength: f32 = smoothing_strength
                .parse()
                .ok()
                .filter(|&s| s >= 0.0)
                .ok_or_else(|| {
                    "Invalid smoothing strength for --stabilize-virtual-cursor=".to_string()
                })?;
            let sticky_radius: f32 = sticky_radius
                .parse()
                .ok()
                .filter(|&s| s >= 0.0)
                .ok_or_else(|| {
                    "Invalid sticky radius for --stabilize-virtual-cursor=".to_string()
                })?;
            self.stabilize_virtual_cursor = Some((smoothing_strength, sticky_radius));
        } else if let Some(value) = arg.strip_prefix("--gles1=") {
            self.gles1_implementation = Some(
                GLESImplementation::from_short_name(value)
                    .map_err(|_| "Unrecognized --gles1= value".to_string())?,
            );
        } else if arg == "--disable-direct-memory-access" {
            self.direct_memory_access = false;
        } else if let Some(address) = arg.strip_prefix("--gdb=") {
            let addrs = address
                .to_socket_addrs()
                .map_err(|e| format!("Could not resolve GDB server listen address: {e}"))?
                .collect();
            self.gdb_listen_addrs = Some(addrs);
        } else if let Some(value) = arg.strip_prefix("--preferred-languages=") {
            self.preferred_languages = Some(value.split(',').map(ToOwned::to_owned).collect());
        } else if arg == "--headless" {
            self.headless = true;
            // Can't show the dialog box when headless!
            self.popup_errors = false;
        } else if arg == "--print-fps" {
            self.print_fps = true;
        } else if let Some(value) = arg.strip_prefix("--fps-limit=") {
            if value == "off" {
                self.fps_limit = None;
            } else {
                let limit: f64 = value
                    .parse()
                    .ok()
                    .filter(|&v| v > 0.0)
                    .ok_or_else(|| "Invalid value for --fps-limit=".to_string())?;
                self.fps_limit = Some(limit);
            }
        } else if arg == "--force-composition" {
            self.force_composition = true;
        } else if arg == "--allow-network-access" {
            self.network_access = true;
        } else if arg == "--no-error-popup" {
            self.popup_errors = false;
        } else if let Some(values) = arg.strip_prefix("--dump=") {
            self.dumping_options = parse_dump_options(values)?;
        } else if let Some(path) = arg.strip_prefix("--dump-file=") {
            self.dumping_file = crate::paths::user_data_base_path().join(path);
        } else if arg == "--ignore-gl-errors" {
            self.ignore_gl_errors = true;
        } else if let Some(value) = arg.strip_prefix("--zero-stack-after-guest-to-host-call=") {
            self.zero_stack_after_guest_to_host_call = Some(value.parse().map_err(|_| {
                "Invalid value for --zero-stack-after-guest-to-host-call=".to_string()
            })?);
        } else {
            return Ok(false);
        };
        Ok(true)
    }
}

/// Try to get app-specific options from a file.
///
/// Returns [Ok] if there is no error when reading the file, otherwise [Err].
/// The [Ok] value is a [Some] with the options if they could be found, or
/// [None] if no options were found for this app.
pub fn get_options_from_file<F: Read>(file: F, app_id: &str) -> Result<Option<String>, String> {
    let file = BufReader::new(file);
    for (line_no, line) in BufRead::lines(file).enumerate() {
        // Line numbering usually starts from 1
        let line_no = line_no + 1;

        let line = line.map_err(|e| format!("Error while reading line {line_no}: {e}"))?;

        // # for single-line comments
        let line = if let Some((rest, _)) = line.split_once('#') {
            rest
        } else {
            &line
        };

        // Empty/all-comment lines ignored
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (line_app_id, line_options) = line.split_once(':').ok_or_else(|| format!("Line {line_no} is not a comment and is missing a colon (:) to separate the app ID from the options"))?;
        let line_app_id = line_app_id.trim();

        if line_app_id != app_id {
            continue;
        }

        let line_options = line_options.trim();
        if line_options.is_empty() {
            return Ok(None);
        } else {
            return Ok(Some(line_options.to_string()));
        }
    }
    Ok(None)
}

#[derive(Default, Clone)]
pub struct DumpingOptions {
    pub linking_info: bool,
    pub symbols: bool,
}

impl DumpingOptions {
    /// Check if any of the dumping options are active.
    pub fn any(&self) -> bool {
        self.linking_info || self.symbols
    }
}

fn parse_dump_options(options: &str) -> Result<DumpingOptions, String> {
    let mut dumping_options = DumpingOptions::default();
    for opt in options.split(",") {
        if opt == "linking-info" {
            // Dumps linked symbols, classes and selectors for the given app
            dumping_options.linking_info = true;
        } else if opt == "symbols" {
            // Dumps touchHLE provided symbols and exits
            dumping_options.symbols = true;
        } else {
            return Err(format!("Unrecognized option {opt} for --dump=..."));
        }
    }
    Ok(dumping_options)
}
