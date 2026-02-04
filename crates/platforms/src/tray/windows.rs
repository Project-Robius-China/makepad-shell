#![cfg(target_os = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Cursor;
use std::iter::once;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut, NonNull};

use makepad_shell_core::command::CommandId;
use makepad_shell_core::tray::{TrayCommandItem, TrayIcon, TrayMenuItem, TrayMenuModel, TrayModel};
use png::ColorType;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BITMAPV5HEADER, BI_BITFIELDS,
    DIB_RGB_COLORS, HDC,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, GetCursorPos, GetWindowLongPtrW, PostMessageW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, SetWindowLongPtrW, TrackPopupMenu, HICON,
    WNDCLASSW, GWLP_USERDATA, HMENU, ICONINFO, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP,
    WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP,
};

const TRAY_WINDOW_CLASS: &str = "MakepadShellTrayWindow";
const WM_TRAYICON: u32 = WM_APP + 1;

#[derive(Debug)]
pub enum WindowsTrayError {
    Unsupported,
    BadIcon,
    ApiFailed(&'static str),
}

pub struct WindowsTrayHandle {
    hwnd: HWND,
    state: NonNull<TrayState>,
}

impl WindowsTrayHandle {
    pub fn update_menu(&mut self, menu: &TrayMenuModel) -> Result<(), WindowsTrayError> {
        unsafe {
            self.state.as_mut().menu = menu.clone();
        }
        Ok(())
    }

    pub fn update_icon(&mut self, icon: &TrayIcon) -> Result<(), WindowsTrayError> {
        let new_icon = icon_to_hicon(icon)?;
        unsafe {
            let state = self.state.as_mut();
            let old_icon = state.icon;
            state.icon = new_icon;
            state.nid.hIcon = new_icon;
            state.nid.uFlags = NIF_ICON | NIF_MESSAGE;
            if Shell_NotifyIconW(NIM_MODIFY, &mut state.nid as *mut _) == 0 {
                state.icon = old_icon;
                state.nid.hIcon = old_icon;
                let _ = DestroyIcon(new_icon);
                return Err(WindowsTrayError::ApiFailed("Shell_NotifyIconW"));
            }
            if !old_icon.is_null() {
                let _ = DestroyIcon(old_icon);
            }
        }
        Ok(())
    }

    pub fn update_tooltip(&mut self, tooltip: Option<&str>) -> Result<(), WindowsTrayError> {
        unsafe {
            let state = self.state.as_mut();
            write_tooltip(&mut state.nid, tooltip);
            state.nid.uFlags = NIF_TIP | NIF_MESSAGE;
            if Shell_NotifyIconW(NIM_MODIFY, &mut state.nid as *mut _) == 0 {
                return Err(WindowsTrayError::ApiFailed("Shell_NotifyIconW"));
            }
        }
        Ok(())
    }
}

impl Drop for WindowsTrayHandle {
    fn drop(&mut self) {
        unsafe {
            let state = self.state.as_mut();
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut state.nid as *mut _);
            if !state.icon.is_null() {
                let _ = DestroyIcon(state.icon);
            }
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}

pub fn create_tray_windows(
    model: TrayModel,
    on_command: Box<dyn Fn(CommandId) + 'static>,
    on_activate: Box<dyn Fn() + 'static>,
) -> Result<WindowsTrayHandle, WindowsTrayError> {
    unsafe {
        let hinstance = GetModuleHandleW(null());
        if hinstance.is_null() {
            return Err(WindowsTrayError::ApiFailed("GetModuleHandleW"));
        }

        let class_name = to_wide(TRAY_WINDOW_CLASS);
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        let _ = RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            hinstance,
            null_mut(),
        );
        if hwnd.is_null() {
            return Err(WindowsTrayError::ApiFailed("CreateWindowExW"));
        }

        let icon = match icon_to_hicon(&model.icon) {
            Ok(icon) => icon,
            Err(err) => {
                let _ = DestroyWindow(hwnd);
                return Err(err);
            }
        };
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = icon;
        write_tooltip(&mut nid, model.tooltip.as_deref());
        if model.tooltip.is_some() {
            nid.uFlags |= NIF_TIP;
        }

        if Shell_NotifyIconW(NIM_ADD, &mut nid as *mut _) == 0 {
            let _ = DestroyIcon(icon);
            let _ = DestroyWindow(hwnd);
            return Err(WindowsTrayError::ApiFailed("Shell_NotifyIconW"));
        }

        let taskbar_restart_msg = RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr());

        let state = Box::new(TrayState {
            hwnd,
            nid,
            icon,
            menu: model.menu,
            menu_map: HashMap::new(),
            on_command,
            on_activate,
            taskbar_restart_msg,
        });
        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        Ok(WindowsTrayHandle {
            hwnd,
            state: NonNull::new(state_ptr).unwrap(),
        })
    }
}

// ------------------------------
// Internal helpers
// ------------------------------

struct TrayState {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    icon: HICON,
    menu: TrayMenuModel,
    menu_map: HashMap<u32, CommandId>,
    on_command: Box<dyn Fn(CommandId) + 'static>,
    on_activate: Box<dyn Fn() + 'static>,
    taskbar_restart_msg: u32,
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState;
    if !state_ptr.is_null() {
        let state = &mut *state_ptr;
        if state.taskbar_restart_msg != 0 && msg == state.taskbar_restart_msg {
            let _ = Shell_NotifyIconW(NIM_ADD, &mut state.nid as *mut _);
            return 0;
        }
        if msg == WM_TRAYICON {
            let event = lparam as u32;
            match event {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (state.on_activate)();
                    }));
                }
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    show_tray_menu(state);
                }
                _ => {}
            }
            return 0;
        }
        if msg == WM_DESTROY {
            return 0;
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn show_tray_menu(state: &mut TrayState) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        state.menu_map.clear();
        let mut next_id: u32 = 1;
        build_menu_items(menu, &state.menu.items, &mut state.menu_map, &mut next_id);

        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            let _ = DestroyMenu(menu);
            return;
        }

        let _ = SetForegroundWindow(state.hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            point.x,
            point.y,
            0,
            state.hwnd,
            null(),
        );
        if selected != 0 {
            if let Some(cmd) = state.menu_map.get(&(selected as u32)).copied() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (state.on_command)(cmd);
                }));
            }
        }
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(state.hwnd, WM_NULL, 0, 0);
    }
}

fn build_menu_items(
    menu: HMENU,
    items: &[TrayMenuItem],
    map: &mut HashMap<u32, CommandId>,
    next_id: &mut u32,
) {
    for item in items {
        match item {
            TrayMenuItem::Separator => {
                unsafe {
                    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
                }
            }
            TrayMenuItem::Command(cmd) => {
                append_command_item(menu, cmd, map, next_id);
            }
            TrayMenuItem::Submenu(sub) => {
                let submenu = unsafe { CreatePopupMenu() };
                if submenu.is_null() {
                    continue;
                }
                build_menu_items(submenu, &sub.items, map, next_id);
                let wide = to_wide(&sub.label);
                unsafe {
                    let _ = AppendMenuW(menu, MF_POPUP | MF_STRING, submenu as usize, wide.as_ptr());
                }
            }
        }
    }
}

fn append_command_item(
    menu: HMENU,
    cmd: &TrayCommandItem,
    map: &mut HashMap<u32, CommandId>,
    next_id: &mut u32,
) {
    let id = *next_id;
    *next_id = next_id.saturating_add(1);
    map.insert(id, cmd.id);

    let wide = to_wide(&cmd.label);
    let mut flags = MF_STRING;
    if !cmd.enabled {
        flags |= MF_GRAYED;
    }
    if cmd.checked {
        flags |= MF_CHECKED;
    }
    unsafe {
        let _ = AppendMenuW(menu, flags, id as usize, wide.as_ptr());
    }
}

fn write_tooltip(nid: &mut NOTIFYICONDATAW, tooltip: Option<&str>) {
    nid.szTip.fill(0);
    if let Some(text) = tooltip {
        let wide: Vec<u16> = OsStr::new(text).encode_wide().collect();
        let max = nid.szTip.len().saturating_sub(1);
        let len = wide.len().min(max);
        nid.szTip[..len].copy_from_slice(&wide[..len]);
    }
}

fn icon_to_hicon(icon: &TrayIcon) -> Result<HICON, WindowsTrayError> {
    match icon {
        TrayIcon::Png { bytes, .. } => png_to_hicon(bytes),
    }
}

fn png_to_hicon(bytes: &[u8]) -> Result<HICON, WindowsTrayError> {
    if bytes.is_empty() {
        return Err(WindowsTrayError::BadIcon);
    }
    let cursor = Cursor::new(bytes);
    let mut decoder = png::Decoder::new(cursor);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| WindowsTrayError::BadIcon)?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|_| WindowsTrayError::BadIcon)?;
    let data = &buf[..info.buffer_size()];

    let rgba = match info.color_type {
        ColorType::Rgba => data.to_vec(),
        ColorType::Rgb => {
            let mut out = Vec::with_capacity((data.len() / 3) * 4);
            for chunk in data.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        ColorType::Grayscale => {
            let mut out = Vec::with_capacity(data.len() * 4);
            for g in data {
                out.extend_from_slice(&[*g, *g, *g, 255]);
            }
            out
        }
        ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((data.len() / 2) * 4);
            for chunk in data.chunks_exact(2) {
                let g = chunk[0];
                let a = chunk[1];
                out.extend_from_slice(&[g, g, g, a]);
            }
            out
        }
        _ => return Err(WindowsTrayError::BadIcon),
    };

    if info.width == 0 || info.height == 0 {
        return Err(WindowsTrayError::BadIcon);
    }

    let mut bgra = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
    }

    unsafe { bgra_to_hicon(info.width, info.height, &bgra) }
}

unsafe fn bgra_to_hicon(
    width: u32,
    height: u32,
    bgra: &[u8],
) -> Result<HICON, WindowsTrayError> {
    let mut header: BITMAPV5HEADER = zeroed();
    header.bV5Size = size_of::<BITMAPV5HEADER>() as u32;
    header.bV5Width = width as i32;
    header.bV5Height = -(height as i32);
    header.bV5Planes = 1;
    header.bV5BitCount = 32;
    header.bV5Compression = BI_BITFIELDS;
    header.bV5RedMask = 0x00FF0000;
    header.bV5GreenMask = 0x0000FF00;
    header.bV5BlueMask = 0x000000FF;
    header.bV5AlphaMask = 0xFF000000;
    header.bV5SizeImage = (width * height * 4) as u32;

    let mut bits: *mut core::ffi::c_void = null_mut();
    let hdc: HDC = GetDC(null_mut());
    let hbitmap = CreateDIBSection(
        hdc,
        &header as *const _ as *const _,
        DIB_RGB_COLORS,
        &mut bits,
        null_mut(),
        0,
    );
    let _ = ReleaseDC(null_mut(), hdc);
    if hbitmap.is_null() || bits.is_null() {
        return Err(WindowsTrayError::BadIcon);
    }

    std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());

    let mask = CreateBitmap(width as i32, height as i32, 1, 1, null());
    if mask.is_null() {
        let _ = DeleteObject(hbitmap as _);
        return Err(WindowsTrayError::BadIcon);
    }

    let mut icon_info: ICONINFO = zeroed();
    icon_info.fIcon = 1;
    icon_info.xHotspot = 0;
    icon_info.yHotspot = 0;
    icon_info.hbmMask = mask;
    icon_info.hbmColor = hbitmap;

    let hicon = CreateIconIndirect(&icon_info);
    let _ = DeleteObject(hbitmap as _);
    let _ = DeleteObject(mask as _);

    if hicon.is_null() {
        return Err(WindowsTrayError::BadIcon);
    }
    Ok(hicon)
}

fn to_wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(once(0)).collect()
}
