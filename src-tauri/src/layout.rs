use crate::windowsutil::*;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::*;

// ─────────────────────────────────────────────────────────────
// 窗口管理层  → windowsutil::list_eligible_windows
// 布局计算层  → geometry_for / rects_for
// 窗口执行层  → animate / set_rect
// 布局状态层  → STATE（记住上次布局把哪些窗口放进了哪些槽）
// 互换层      → rotate（Ctrl+Tab 整体轮换）/ swap_direction（Ctrl+方向）
// ─────────────────────────────────────────────────────────────

/// 上次应用的布局：mode = 布局编号(1-4)，slots[i] = 当前位于第 i 个槽位的窗口。
/// 互换 / 预设功能都依赖它来知道「每个窗口在哪个位置」。
struct LayoutState {
    mode: u8,
    slots: Vec<HWND>,
}

static STATE: Mutex<Option<LayoutState>> = Mutex::new(None);

/// Arrange eligible windows on the active monitor according to `mode`:
/// 1 = 单窗口满屏, 2 = 左右双分, 3 = 左一整窗+右上+右下(L形三窗), 4 = 四宫格.
/// The panel's own HWND (`exclude`) is never tiled. Also records the slot
/// assignment so later rotate / swap / preset operations know each window's position.
pub fn arrange(mode: u8, exclude: HWND) {
    let windows = list_eligible_windows(exclude);
    if windows.is_empty() {
        return;
    }
    let area = work_area(target_monitor());
    let rects = geometry_for(mode, &area);
    let n = rects.len();
    if n == 0 {
        return;
    }
    let slots: Vec<HWND> = windows.iter().take(n).cloned().collect();
    let targets: Vec<(HWND, RECT)> = slots.iter().cloned().zip(rects).collect();
    if let Ok(mut s) = STATE.lock() {
        *s = Some(LayoutState {
            mode,
            slots: slots.clone(),
        });
    }
    // Run off the caller thread so the panel can dismiss instantly.
    thread::spawn(move || animate(targets));
}

/// Fractional rects (x, y, w, h) of the work area for each layout mode,
/// returned in slot order so slot i always maps to rects[i].
pub fn geometry_for(mode: u8, area: &RECT) -> Vec<RECT> {
    match mode {
        1 => rects_for(area, &[(0.0, 0.0, 1.0, 1.0)]),
        2 => rects_for(
            area,
            &[(0.0, 0.0, 0.5, 1.0), (0.5, 0.0, 0.5, 1.0)],
        ),
        3 => rects_for(
            area,
            &[
                (0.0, 0.0, 0.5, 1.0), // 左：整高
                (0.5, 0.0, 0.5, 0.5), // 右上
                (0.5, 0.5, 0.5, 0.5), // 右下
            ],
        ),
        4 => rects_for(
            area,
            &[
                (0.0, 0.0, 0.5, 0.5),
                (0.5, 0.0, 0.5, 0.5),
                (0.0, 0.5, 0.5, 0.5),
                (0.5, 0.5, 0.5, 0.5),
            ],
        ),
        _ => Vec::new(),
    }
}

/// `parts` are fractional rectangles (x, y, w, h) of the work area.
fn rects_for(area: &RECT, parts: &[(f64, f64, f64, f64)]) -> Vec<RECT> {
    let w = (area.right - area.left) as f64;
    let h = (area.bottom - area.top) as f64;
    parts
        .iter()
        .map(|&(fx, fy, fw, fh)| RECT {
            left: area.left + (fx * w).round() as i32,
            top: area.top + (fy * h).round() as i32,
            right: area.left + ((fx + fw) * w).round() as i32,
            bottom: area.top + ((fy + fh) * h).round() as i32,
        })
        .collect()
}

/// Move the given slots to their layout rects. Shared by arrange / rotate /
/// swap / preset-restore. Also records the new arrangement as the live state.
pub fn apply_slots(mode: u8, slots: Vec<HWND>) {
    if let Ok(mut s) = STATE.lock() {
        *s = Some(LayoutState {
            mode,
            slots: slots.clone(),
        });
    }
    let area = work_area(target_monitor());
    let rects = geometry_for(mode, &area);
    let n = rects.len().min(slots.len());
    let targets: Vec<(HWND, RECT)> = slots
        .iter()
        .take(n)
        .cloned()
        .zip(rects.into_iter().take(n))
        .collect();
    thread::spawn(move || animate(targets));
}

/// Snapshot of the currently applied layout (mode + per-slot windows),
/// used by the V1.4 preset feature to remember / save the live arrangement.
pub fn current_state() -> Option<(u8, Vec<HWND>)> {
    STATE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| (s.mode, s.slots.clone())))
}

/// Ctrl+Tab: rotate every window one slot forward in layout order
/// (1→2→3→4→1). No-op if no layout applied, only one window, or a window closed.
pub fn rotate() {
    let mut guard = match STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut st = match guard.take() {
        Some(s) => s,
        None => return,
    };
    if !st.slots.iter().all(|&h| is_window(h)) {
        *guard = Some(st); // windows changed; require re-apply
        return;
    }
    let n = st.slots.len();
    if n <= 1 {
        *guard = Some(st);
        return;
    }
    // slot i receives the window from slot (i-1); window at slot0 wraps to last.
    let rotated: Vec<HWND> = (0..n).map(|i| st.slots[(i + n - 1) % n]).collect();
    st.slots = rotated.clone();
    let mode = st.mode;
    *guard = Some(st);
    drop(guard);
    apply_slots(mode, rotated);
}

/// Ctrl+方向键: swap the foreground window with the neighbour viewport in
/// `dir` (0=Left, 1=Right, 2=Up, 3=Down). No-op if no layout / fg not tiled /
/// no neighbour in that direction.
pub fn swap_direction(dir: u8) {
    let fg = foreground_window();
    let mut guard = match STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut st = match guard.take() {
        Some(s) => s,
        None => return,
    };
    if !st.slots.iter().all(|&h| is_window(h)) {
        *guard = Some(st);
        return;
    }
    let idx = match st.slots.iter().position(|&h| h == fg) {
        Some(i) => i,
        None => {
            *guard = Some(st);
            return;
        }
    };
    let nbr = match neighbor_slot(st.mode, idx, dir) {
        Some(j) => j,
        None => {
            *guard = Some(st);
            return;
        }
    };
    st.slots.swap(idx, nbr);
    let mode = st.mode;
    let slots = st.slots.clone();
    *guard = Some(st);
    drop(guard);
    apply_slots(mode, slots);
}

/// Neighbour slot index for `idx` in `dir`, or None if there is no viewport
/// in that direction for the current layout.
/// dir: 0=Left, 1=Right, 2=Up, 3=Down
fn neighbor_slot(mode: u8, idx: usize, dir: u8) -> Option<usize> {
    match mode {
        2 => match (idx, dir) {
            // slots: 0=left, 1=right
            (0, 1) => Some(1),
            (1, 0) => Some(0),
            _ => None,
        },
        3 => match (idx, dir) {
            // slots: 0=left(整高), 1=右上, 2=右下
            (0, 1) => Some(1), // 左 → 右上
            (1, 0) => Some(0),
            (1, 3) => Some(2), // 右上 → 右下
            (2, 0) => Some(0),
            (2, 2) => Some(1), // 右下 → 右上
            _ => None,
        },
        4 => match (idx, dir) {
            // slots: 0=左上, 1=右上, 2=左下, 3=右下
            (0, 1) => Some(1),
            (0, 3) => Some(2),
            (1, 0) => Some(0),
            (1, 3) => Some(3),
            (2, 1) => Some(3),
            (2, 2) => Some(0),
            (3, 0) => Some(2),
            (3, 2) => Some(1),
            _ => None,
        },
        _ => None,
    }
}

/// Window execution layer: smoothly move every window to its target rect.
/// Maximized (Zoomed) windows ignore `SetWindowPos`, so each is restored
/// first — this is what makes Chrome and other apps resizable on layout.
fn animate(targets: Vec<(HWND, RECT)>) {
    let mut needs_wait = false;
    for (h, _) in &targets {
        if is_maximized(*h) {
            restore(*h);
            needs_wait = true;
        }
    }
    if needs_wait {
        // Give the OS a beat to apply the restore before we read/size.
        thread::sleep(Duration::from_millis(80));
    }
    let starts: Vec<(HWND, RECT)> = targets.iter().map(|(h, _)| (*h, get_rect(*h))).collect();
    let total_ms = 200u64;
    let frames = 12u64;
    for f in 1..=frames {
        let t = f as f64 / frames as f64;
        let e = ease_in_out(t);
        for ((h, s), (_, tgt)) in starts.iter().zip(targets.iter()) {
            set_rect(*h, lerp(*s, *tgt, e));
        }
        thread::sleep(Duration::from_millis(total_ms / frames));
    }
    for (h, tgt) in &targets {
        set_rect(*h, *tgt);
    }
}

fn lerp(a: RECT, b: RECT, t: f64) -> RECT {
    RECT {
        left: a.left + ((b.left - a.left) as f64 * t).round() as i32,
        top: a.top + ((b.top - a.top) as f64 * t).round() as i32,
        right: a.right + ((b.right - a.right) as f64 * t).round() as i32,
        bottom: a.bottom + ((b.bottom - a.bottom) as f64 * t).round() as i32,
    }
}

/// easeInOutQuad — natural acceleration/deceleration, no teleport.
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}
