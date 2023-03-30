/* toasts.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use gtk::gio;

use super::i18n::i18n_k;
use crate::views::window::Window;

pub static SUCCESS_GREEN: &str = "\"#57e389\"";
pub static ERROR_RED: &str = "\"#c01c28\"";

#[allow(dead_code)]
pub fn add_toast_markup(msg: &str) {
    let app = gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<gtk::Application>()
        .unwrap();
    
    let win = app
        .active_window()
        .unwrap()
        .downcast::<Window>()
        .unwrap();

        let toast = adw::Toast::new(msg);
        toast.set_timeout(1);
        win.add_toast(toast);
}

#[allow(dead_code)]
pub fn add_success_toast(verb: &str, msg: &str) {
    let app = gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<gtk::Application>()
        .unwrap();
    
    let win = app
        .active_window()
        .unwrap()
        .downcast::<Window>()
        .unwrap();

        let toast = adw::Toast::new(format!("<span foreground={}>{}</span> {}", SUCCESS_GREEN, verb, msg).as_str());
        toast.set_timeout(2);
        
        win.add_toast(toast);
}

#[allow(dead_code)]
pub fn add_error_toast(msg: String) {
    let app = gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<gtk::Application>()
        .unwrap();
    
    let win = app
        .active_window()
        .unwrap()
        .downcast::<Window>()
        .unwrap();

        // Translators: Only replace "Error!". Reorder if necessary
        let toast = adw::Toast::new(&i18n_k("<span foreground={ERROR_RED}>Error!</span> {error_msg}", &[("ERROR_RED", ERROR_RED), ("error_msg", &msg)]));

        toast.set_timeout(2);
        win.add_toast(toast);
}