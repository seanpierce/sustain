// SPDX-License-Identifier: GPL-3.0-or-later

//! Synchronize GTK4 appearance with Cinnamon's portal theme setting.

use gtk::{gio, glib, prelude::*};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS: &str = "org.freedesktop.portal.Settings";
const NAMESPACE: &str = "org.gnome.desktop.interface";
const KEY: &str = "gtk-theme";

fn apply_theme(theme: &str) {
    let Some(settings) = gtk::Settings::default() else {
        return;
    };

    let dark = theme.to_ascii_lowercase().contains("dark");

    settings.set_gtk_theme_name(Some("Adwaita"));
    settings.set_gtk_application_prefer_dark_theme(dark);

    eprintln!("Sustain: desktop appearance updated: {theme} (dark={dark})");
}

pub(crate) fn install(app: &gtk::Application) {
    app.connect_startup(|app| {
        let proxy = match gio::DBusProxy::for_bus_sync(
            gio::BusType::Session,
            gio::DBusProxyFlags::NONE,
            None,
            PORTAL,
            PATH,
            SETTINGS,
            None::<&gio::Cancellable>,
        ) {
            Ok(proxy) => proxy,
            Err(error) => {
                eprintln!("Sustain: appearance portal unavailable: {error}");
                return;
            }
        };

        // Read the initial desktop theme.
        use glib::variant::ToVariant;

        if let Ok(reply) = proxy.call_sync(
            "Read",
            Some(&(NAMESPACE, KEY).to_variant()),
            gio::DBusCallFlags::NONE,
            1500,
            None::<&gio::Cancellable>,
        ) && let Some(outer) = reply.child_value(0).as_variant()
            && let Some(inner) = outer.as_variant()
            && let Some(theme) = inner.str()
        {
            apply_theme(theme);
        }

        // Receive changes while the application is running.
        proxy.connect_g_signal(|_, _, signal, parameters| {
            if signal != "SettingChanged" || parameters.n_children() != 3 {
                return;
            }

            let namespace = parameters.child_value(0);
            let key = parameters.child_value(1);

            if namespace.str() != Some(NAMESPACE) || key.str() != Some(KEY) {
                return;
            }

            if let Some(value) = parameters.child_value(2).as_variant()
                && let Some(theme) = value.str()
            {
                apply_theme(theme);
            }
        });

        // Keep the proxy alive for the application's lifetime.
        app.connect_shutdown(move |_| {
            let _ = &proxy;
        });
    });
}
