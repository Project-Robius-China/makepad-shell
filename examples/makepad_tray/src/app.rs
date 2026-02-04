#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::sync::Mutex;

use makepad_widgets::*;
use makepad_widgets::makepad_platform::CxOsOp;
use makepad_widgets::makepad_platform::thread::SignalToUI;
use makepad_widgets::makepad_platform::WindowId;
use makepad_shell::{
    CommandId, Key, Modifiers, Shortcut, Tray, TrayCommandItem, TrayHandle, TrayIcon,
    TrayMenuItem, TrayMenuItemRole, TrayMenuModel, TrayModel,
};

const CMD_TOGGLE_GRID: u64 = 1;
const CMD_CLOSE_TO_TRAY: u64 = 2;
const CMD_QUIT: u64 = 3;

#[cfg(any(target_os = "macos", target_os = "windows"))]
static TRAY_COMMAND: Mutex<Option<CommandId>> = Mutex::new(None);
#[cfg(any(target_os = "macos", target_os = "windows"))]
static TRAY_ACTIVATE: Mutex<bool> = Mutex::new(false);

live_design! {
    use link::theme::*;
    use link::widgets::*;

    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: {title: "Makepad Tray Example"}
                caption_bar = {
                    windows_buttons = <View> {
                        width: Fit,
                        height: Fit,
                        flow: Right,
                        min = <DesktopButton> {draw_bg: {button_type: WindowsMin}}
                        max = <DesktopButton> {draw_bg: {button_type: WindowsMax}}
                        tray_close = <DesktopButton> {draw_bg: {button_type: WindowsClose}}
                    }
                }
                body = <View> {
                    width: Fill,
                    height: Fill,
                    flow: Down,
                    align: {x: 0.5, y: 0.5},
                    spacing: 12,
                    show_bg: true,
                    draw_bg: {
                        color: #2b2b2b
                    }

                    <Label> {
                        draw_text: {color: #fff}
                        text: "点击托盘图标：激活应用；右键：打开菜单。"
                    }

                    status_label = <Label> {
                        draw_text: {color: #9}
                        text: "Last command: (none)"
                    }
                }
            }
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    show_grid: bool,
    #[rust]
    last_command: Option<CommandId>,
    #[rust]
    tray: Option<TrayHandle>,
    #[rust]
    close_to_tray: bool,
    #[rust]
    tray_signal: SignalToUI,
    #[rust]
    window_id: Option<WindowId>,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_widgets::live_design(cx);
    }
}

impl App {
    fn install_tray(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if self.tray.is_some() {
                return;
            }

            let icon = tray_icon(self.show_grid);
            let menu = build_tray_menu(self.show_grid, self.close_to_tray);
            let model = TrayModel::new(icon, menu).with_tooltip("Makepad Shell Tray");
            let menu_signal = self.tray_signal.clone();
            let activate_signal = self.tray_signal.clone();

            let result = Tray::create(
                model,
                move |cmd| {
                    log!("tray menu invoked: {}", cmd.as_u64());
                    eprintln!("tray menu invoked: {}", cmd.as_u64());
                    if let Ok(mut slot) = TRAY_COMMAND.lock() {
                        *slot = Some(cmd);
                    }
                    menu_signal.set();
                },
                move || {
                    log!("tray activate");
                    eprintln!("tray activate");
                    if let Ok(mut slot) = TRAY_ACTIVATE.lock() {
                        *slot = true;
                    }
                    activate_signal.set();
                },
            );

            match result {
                Ok(handle) => self.tray = Some(handle),
                Err(err) => {
                    log!("create tray failed: {:?}", err);
                    eprintln!("create tray failed: {:?}", err);
                }
            }
        }
    }

    fn drain_tray_events(&mut self, cx: &mut Cx) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if let Ok(mut slot) = TRAY_COMMAND.lock() {
                if let Some(cmd) = slot.take() {
                    self.apply_command(cx, cmd);
                }
            }
            if let Ok(mut slot) = TRAY_ACTIVATE.lock() {
                if *slot {
                    *slot = false;
                    self.handle_tray_activate(cx);
                }
            }

        }
    }

    fn handle_tray_activate(&mut self, cx: &mut Cx) {
        let Some(window_id) = self.window_id else {
            return;
        };
        cx.push_unique_platform_op(CxOsOp::RestoreWindow(window_id));
        #[cfg(target_os = "windows")]
        {
            cx.push_unique_platform_op(CxOsOp::SetTopmost(window_id, true));
            cx.push_unique_platform_op(CxOsOp::SetTopmost(window_id, false));
        }
    }

    fn handle_tray_close_button(&mut self, cx: &mut Cx) {
        let Some(window_id) = self.window_id else {
            return;
        };
        if self.close_to_tray {
            #[cfg(target_os = "windows")]
            {
                cx.push_unique_platform_op(CxOsOp::MinimizeWindow(window_id));
            }
            #[cfg(not(target_os = "windows"))]
            {
                cx.push_unique_platform_op(CxOsOp::HideWindow(window_id));
            }
        } else {
            cx.quit();
        }
    }

    fn apply_command(&mut self, cx: &mut Cx, cmd: CommandId) {
        self.last_command = Some(cmd);
        match cmd.as_u64() {
            CMD_TOGGLE_GRID => {
                self.show_grid = !self.show_grid;
                if let Some(handle) = self.tray.as_mut() {
                    let _ = handle.update_menu(build_tray_menu(self.show_grid, self.close_to_tray));
                    let _ = handle.update_icon(tray_icon(self.show_grid));
                    let tip = if self.show_grid { "Grid: ON" } else { "Grid: OFF" };
                    let _ = handle.update_tooltip(Some(tip.to_string()));
                }
            }
            CMD_CLOSE_TO_TRAY => {
                self.close_to_tray = !self.close_to_tray;
                if let Some(handle) = self.tray.as_mut() {
                    let _ = handle.update_menu(build_tray_menu(self.show_grid, self.close_to_tray));
                }
            }
            CMD_QUIT => {
                cx.quit();
            }
            _ => {}
        }
        self.update_status_label(cx);
    }

    fn update_status_label(&mut self, cx: &mut Cx) {
        let text = match self.last_command {
            Some(cmd) => format!(
                "Last command: {} (show_grid: {}, close_to_tray: {})",
                cmd.as_u64(),
                self.show_grid,
                self.close_to_tray
            ),
            None => format!(
                "Last command: (none) (show_grid: {}, close_to_tray: {})",
                self.show_grid, self.close_to_tray
            ),
        };
        self.ui.label(ids!(status_label)).set_text(cx, &text);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, _cx: &mut Cx) {
        self.close_to_tray = true;
        self.install_tray();
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        if let Event::WindowGeomChange(ev) = event {
            self.window_id = Some(ev.window_id);
        }
        if let Event::WindowGotFocus(window_id) = event {
            self.window_id = Some(*window_id);
        }
        if let Event::WindowCloseRequested(ev) = event {
            self.window_id = Some(ev.window_id);
            if self.close_to_tray {
                ev.accept_close.set(false);
                #[cfg(target_os = "windows")]
                {
                    cx.push_unique_platform_op(CxOsOp::MinimizeWindow(ev.window_id));
                }
                #[cfg(not(target_os = "windows"))]
                {
                    cx.push_unique_platform_op(CxOsOp::HideWindow(ev.window_id));
                }
            }
        }
        if let Event::Actions(actions) = event {
            let close_uid = self
                .ui
                .widget(ids!(main_window.caption_bar.windows_buttons.tray_close))
                .widget_uid();
            if close_uid.0 != 0 {
                if let ButtonAction::Clicked(_) = actions.find_widget_action(close_uid).cast_ref() {
                    self.handle_tray_close_button(cx);
                }
            }
        }
        self.install_tray();
        self.drain_tray_events(cx);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

fn build_tray_menu(show_grid: bool, close_to_tray: bool) -> TrayMenuModel {
    let mut toggle_grid =
        TrayCommandItem::new(CommandId::new(CMD_TOGGLE_GRID).unwrap(), "Show Grid");
    toggle_grid.checked = show_grid;
    toggle_grid.shortcut = Some(Shortcut {
        mods: Modifiers {
            meta: true,
            ..Modifiers::default()
        },
        key: Key::Char('g'),
    });

    let mut close_to_tray_item = TrayCommandItem::new(
        CommandId::new(CMD_CLOSE_TO_TRAY).unwrap(),
        "Close to Tray",
    );
    close_to_tray_item.checked = close_to_tray;

    TrayMenuModel::new(vec![
        TrayMenuItem::Command(toggle_grid),
        TrayMenuItem::Command(close_to_tray_item),
        TrayMenuItem::Separator,
        TrayMenuItem::Command(
            TrayCommandItem::new(CommandId::new(CMD_QUIT).unwrap(), "Quit")
                .with_role(TrayMenuItemRole::Quit),
        ),
    ])
}

fn tray_icon(show_grid: bool) -> TrayIcon {
    if show_grid {
        let bytes: &[u8] = include_bytes!("../assets/tray_alt.png");
        TrayIcon::from_png_bytes(bytes.to_vec()).with_template(true)
    } else {
        let bytes: &[u8] = include_bytes!("../assets/tray.png");
        TrayIcon::from_png_bytes(bytes.to_vec()).with_template(true)
    }
}
