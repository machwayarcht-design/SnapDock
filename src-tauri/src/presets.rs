use crate::layout::apply_slots;
use crate::layout::current_state;
use crate::windowsutil::*;
use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// A saved window identity — by executable name + title, because raw HWNDs
/// are only valid within a single session. After a restart we re-match these
/// against the currently open windows.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SlotSig {
    exe: String,
    title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Scheme {
    mode: u8,
    wins: Vec<SlotSig>,
}

const N: usize = 4;

static PRESETS: Mutex<[Option<Scheme>; N]> = Mutex::new([None, None, None, None]);
static PENDING: Mutex<Option<u8>> = Mutex::new(None);

pub enum SaveResult {
    Saved(u8),
    Confirm(u8),
    NoLayout,
}

pub enum RestoreResult {
    Restored(u8),
    Empty(u8),
    NoMatch(u8),
}

fn presets_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("presets.json")))
}

fn persist() {
    if let Some(path) = presets_path() {
        if let Ok(s) = serde_json::to_string_pretty(&*PRESETS.lock().unwrap()) {
            let _ = fs::write(&path, s);
        }
    }
}

/// Load saved schemes from disk at startup (no-op if the file is absent).
pub fn load_on_startup() {
    if let Some(path) = presets_path() {
        if let Ok(s) = fs::read_to_string(&path) {
            if let Ok(arr) = serde_json::from_str::<[Option<Scheme>; N]>(&s) {
                *PRESETS.lock().unwrap() = arr;
            }
        }
    }
}

/// Capture the current layout as saveable signatures.
fn capture() -> Option<(u8, Vec<SlotSig>)> {
    let (mode, slots) = current_state()?;
    let sigs: Vec<SlotSig> = slots
        .iter()
        .map(|&h| SlotSig {
            exe: exe_name(h).unwrap_or_default(),
            title: window_title(h),
        })
        .collect();
    Some((mode, sigs))
}

/// Ctrl+Shift+1..4 : save current layout to `slot`. If the slot is empty we
/// commit immediately; if it already holds a scheme we ask for confirmation
/// (the frontend shows a modal; confirm_save / cancel_save finish the flow).
pub fn request_save(slot: u8) -> SaveResult {
    let (mode, sigs) = match capture() {
        Some(c) => c,
        None => return SaveResult::NoLayout,
    };
    let mut p = PRESETS.lock().unwrap();
    if p[slot as usize].is_some() {
        drop(p);
        *PENDING.lock().unwrap() = Some(slot);
        SaveResult::Confirm(slot)
    } else {
        p[slot as usize] = Some(Scheme { mode, wins: sigs });
        drop(p);
        persist();
        SaveResult::Saved(slot)
    }
}

/// Commit the pending save (called from the confirm modal). Saves whatever the
/// current layout is *now* — the user may have tweaked windows before confirming.
pub fn confirm_save(slot: u8) {
    let pending = *PENDING.lock().unwrap();
    if pending != Some(slot) {
        return;
    }
    let (mode, sigs) = match capture() {
        Some(c) => c,
        None => {
            *PENDING.lock().unwrap() = None;
            return;
        }
    };
    let mut p = PRESETS.lock().unwrap();
    p[slot as usize] = Some(Scheme { mode, wins: sigs });
    drop(p);
    *PENDING.lock().unwrap() = None;
    persist();
}

pub fn cancel_save() {
    *PENDING.lock().unwrap() = None;
}

/// Ctrl+1..4 : re-apply the saved scheme in `slot`. Unlike the V1.4 behaviour,
/// this now restores a real *workspace*: it enumerates ALL windows (including
/// minimized / backgrounded ones), matches each saved app by exe (+ title when
/// ambiguous), brings the matched windows to the foreground, arranges them,
/// then minimizes the temporary programs that were open in the meantime — so
/// the user snaps straight back to their original working layout.
pub fn restore(slot: u8) -> RestoreResult {
    let scheme = {
        let p = PRESETS.lock().unwrap();
        match &p[slot as usize] {
            Some(s) => s.clone(),
            None => {
                return RestoreResult::Empty(slot);
            }
        }
    };
    // Enumerate every window (not just the visible/eligible set) so that
    // workspace apps hidden behind temp programs or minimized are found.
    let live = list_all_meta();
    let mut used = vec![false; live.len()];
    let mut slots: Vec<HWND> = Vec::new();
    for sig in &scheme.wins {
        // 1) exact (exe + title) match wins.
        let mut best: Option<usize> = None;
        for (i, (_, exe, title)) in live.iter().enumerate() {
            if used[i] {
                continue;
            }
            if exe == &sig.exe && title == &sig.title {
                best = Some(i);
                break;
            }
        }
        // 2) fall back to exe-only (title may have changed since saving).
        if best.is_none() {
            for (i, (_, exe, _)) in live.iter().enumerate() {
                if used[i] {
                    continue;
                }
                if exe == &sig.exe {
                    best = Some(i);
                    break;
                }
            }
        }
        if let Some(i) = best {
            used[i] = true;
            slots.push(live[i].0);
        }
    }
    if slots.is_empty() {
        return RestoreResult::NoMatch(slot);
    }
    // Bring the *saved* workspace windows forward, and ONLY those:
    //  - if a saved window is minimized, restore it so it can be resized;
    //  - raise it above the temporary programs WITHOUT stealing keyboard focus
    //    (this is what previously caused every background program to pop up).
    for &h in &slots {
        if is_iconic(h) {
            crate::windowsutil::restore(h);
        }
        raise_window(h);
    }
    apply_slots(scheme.mode, slots.clone());
    // Tuck away the temporary programs that were open on the *same monitor as
    // the restored workspace* (not the foreground window's monitor), leaving
    // the workspace clean and screen-exclusive. Windows that are part of the
    // saved scheme are never touched. Only touch windows that are currently
    // visible and not already minimized — background / IME helpers have already
    // been filtered out, so this should only affect a small handful of apps.
    let ws: std::collections::HashSet<HWND> = slots.iter().copied().collect();
    let mon = slots
        .first()
        .map(|&h| monitor_of(h))
        .unwrap_or_else(target_monitor);
    for &(h, _, _) in &live {
        if !ws.contains(&h)
            && is_on_monitor(h, mon)
            && is_window_visible(h)
            && !is_iconic(h)
        {
            minimize_window(h);
        }
    }
    RestoreResult::Restored(slot)
}
