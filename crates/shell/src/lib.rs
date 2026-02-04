#[cfg(all(feature = "context-menu", target_os = "macos"))]
use core::ffi::c_void;

#[cfg(feature = "command")]
pub use makepad_shell_core::command::*;
#[cfg(feature = "menu-model")]
pub use makepad_shell_core::menu::*;
#[cfg(feature = "notification")]
pub use makepad_shell_core::notification::*;
#[cfg(feature = "shortcut")]
pub use makepad_shell_core::shortcut::*;
#[cfg(feature = "tray")]
pub use makepad_shell_core::tray::*;

#[derive(Debug)]
pub enum ShellError {
    Unsupported,
}

#[cfg(feature = "context-menu")]
pub struct ContextMenu;

#[cfg(feature = "context-menu")]
impl ContextMenu {
    pub fn popup(
        menu: MenuModel,
        anchor: MenuAnchor,
        trigger: MenuTrigger,
        on_command: impl Fn(CommandId) + 'static,
    ) -> Result<(), ShellError> {
        popup_context_menu(menu, anchor, trigger, on_command)
    }

    #[cfg(target_os = "macos")]
    pub fn popup_macos(
        menu: MenuModel,
        anchor: MenuAnchor,
        trigger: MenuTrigger,
        ns_view: *mut c_void,
        ns_event: *mut c_void,
        on_command: impl Fn(CommandId) + 'static,
    ) -> Result<(), ShellError> {
        #[cfg(feature = "platforms")]
        {
            return makepad_shell_platforms::menu::macos::popup_context_menu_macos(
                menu,
                anchor,
                trigger,
                ns_view,
                ns_event,
                Box::new(on_command),
            )
            .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(not(feature = "platforms"))]
        {
            let _ = menu;
            let _ = anchor;
            let _ = trigger;
            let _ = ns_view;
            let _ = ns_event;
            let _ = on_command;
            return Err(ShellError::Unsupported);
        }
    }
}

#[cfg(feature = "app-menu")]
pub struct AppMenu;

#[cfg(feature = "app-menu")]
impl AppMenu {
    pub fn set(menu: MenuBarModel, on_command: impl Fn(CommandId) + 'static) -> Result<(), ShellError> {
        set_app_menu(menu, on_command)
    }

    pub fn clear() -> Result<(), ShellError> {
        clear_app_menu()
    }
}

#[cfg(feature = "tray")]
pub struct TrayHandle {
    #[cfg(all(target_os = "macos", feature = "platforms"))]
    inner: makepad_shell_platforms::tray::macos::MacTrayHandle,
    #[cfg(all(target_os = "windows", feature = "platforms"))]
    inner: makepad_shell_platforms::tray::windows::WindowsTrayHandle,
}

#[cfg(feature = "tray")]
impl TrayHandle {
    pub fn update_menu(&mut self, menu: TrayMenuModel) -> Result<(), ShellError> {
        #[cfg(all(target_os = "macos", feature = "platforms"))]
        {
            return self
                .inner
                .update_menu(&menu)
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "macos", not(feature = "platforms")))]
        {
            let _ = menu;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(target_os = "windows", feature = "platforms"))]
        {
            return self
                .inner
                .update_menu(&menu)
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "windows", not(feature = "platforms")))]
        {
            let _ = menu;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let _ = menu;
            Err(ShellError::Unsupported)
        }
    }

    pub fn update_icon(&mut self, icon: TrayIcon) -> Result<(), ShellError> {
        #[cfg(all(target_os = "macos", feature = "platforms"))]
        {
            return self
                .inner
                .update_icon(&icon)
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "macos", not(feature = "platforms")))]
        {
            let _ = icon;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(target_os = "windows", feature = "platforms"))]
        {
            return self
                .inner
                .update_icon(&icon)
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "windows", not(feature = "platforms")))]
        {
            let _ = icon;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let _ = icon;
            Err(ShellError::Unsupported)
        }
    }

    pub fn update_tooltip(&mut self, tooltip: Option<String>) -> Result<(), ShellError> {
        #[cfg(all(target_os = "macos", feature = "platforms"))]
        {
            return self
                .inner
                .update_tooltip(tooltip.as_deref())
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "macos", not(feature = "platforms")))]
        {
            let _ = tooltip;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(target_os = "windows", feature = "platforms"))]
        {
            return self
                .inner
                .update_tooltip(tooltip.as_deref())
                .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(all(target_os = "windows", not(feature = "platforms")))]
        {
            let _ = tooltip;
            Err(ShellError::Unsupported)
        }
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let _ = tooltip;
            Err(ShellError::Unsupported)
        }
    }
}

#[cfg(feature = "tray")]
pub struct Tray;

#[cfg(feature = "tray")]
impl Tray {
    pub fn create(
        model: TrayModel,
        on_command: impl Fn(CommandId) + 'static,
        on_activate: impl Fn() + 'static,
    ) -> Result<TrayHandle, ShellError> {
        #[cfg(target_os = "macos")]
        {
            #[cfg(feature = "platforms")]
            {
                let inner = makepad_shell_platforms::tray::macos::create_tray_macos(
                    model,
                    Box::new(on_command),
                    Box::new(on_activate),
                )
                .map_err(|_| ShellError::Unsupported)?;
                return Ok(TrayHandle { inner });
            }
            #[cfg(not(feature = "platforms"))]
            {
                let _ = model;
                let _ = on_command;
                let _ = on_activate;
                return Err(ShellError::Unsupported);
            }
        }

        #[cfg(target_os = "windows")]
        {
            #[cfg(feature = "platforms")]
            {
                let inner = makepad_shell_platforms::tray::windows::create_tray_windows(
                    model,
                    Box::new(on_command),
                    Box::new(on_activate),
                )
                .map_err(|_| ShellError::Unsupported)?;
                return Ok(TrayHandle { inner });
            }
            #[cfg(not(feature = "platforms"))]
            {
                let _ = model;
                let _ = on_command;
                let _ = on_activate;
                return Err(ShellError::Unsupported);
            }
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let _ = model;
            let _ = on_command;
            let _ = on_activate;
            Err(ShellError::Unsupported)
        }
    }
}

#[cfg(feature = "notification")]
pub struct Notifications;

#[cfg(feature = "notification")]
impl Notifications {
    pub fn show(
        notification: Notification,
        on_command: impl Fn(CommandId) + 'static,
    ) -> Result<(), ShellError> {
        #[cfg(target_os = "macos")]
        {
            #[cfg(feature = "platforms")]
            {
                return makepad_shell_platforms::notification::macos::show_notification_macos(
                    notification,
                    Box::new(on_command),
                )
                .map_err(|_| ShellError::Unsupported);
            }
            #[cfg(not(feature = "platforms"))]
            {
                let _ = notification;
                let _ = on_command;
                return Err(ShellError::Unsupported);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = notification;
            let _ = on_command;
            Err(ShellError::Unsupported)
        }
    }
}

#[cfg(feature = "context-menu")]
pub fn popup_context_menu(
    _menu: MenuModel,
    _anchor: MenuAnchor,
    _trigger: MenuTrigger,
    _on_command: impl Fn(CommandId) + 'static,
) -> Result<(), ShellError> {
    // 未来这里会转发到 platforms
    Err(ShellError::Unsupported)
}

#[cfg(feature = "app-menu")]
pub fn set_app_menu(
    menu: MenuBarModel,
    on_command: impl Fn(CommandId) + 'static,
) -> Result<(), ShellError> {
    #[cfg(target_os = "macos")]
    {
        #[cfg(feature = "platforms")]
        {
            return makepad_shell_platforms::menu::macos::set_app_menu_macos(
                menu,
                Box::new(on_command),
            )
            .map_err(|_| ShellError::Unsupported);
        }
        #[cfg(not(feature = "platforms"))]
        {
            let _ = menu;
            let _ = on_command;
            return Err(ShellError::Unsupported);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = menu;
        let _ = on_command;
        Err(ShellError::Unsupported)
    }
}

#[cfg(feature = "app-menu")]
pub fn clear_app_menu() -> Result<(), ShellError> {
    set_app_menu(MenuBarModel::new(Vec::new()), |_| {})
}
