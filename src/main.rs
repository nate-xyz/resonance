/* main.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod app;
mod config;
mod database;
mod util;
mod model;
mod player;
mod views;
mod i18n;
mod toasts;
mod importer;
mod search;
mod sort;
mod web;

use self::app::App;
use self::views::window::Window;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, glib, prelude::*};

use std::{env, process};
use log::{debug, error, LevelFilter};

fn main() -> glib::ExitCode  {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter_module("scraper", LevelFilter::Off)
        .filter_module("selectors", LevelFilter::Off)
        .filter_module("html5ever", LevelFilter::Off)
        .filter_module("reqwest", LevelFilter::Info)
        .init();

    env::set_var("RUST_BACKTRACE", "1");
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources
    debug!("Loading resources");
    let resources = match env::var("MESON_DEVENV") {
        Err(_) => gio::Resource::load(PKGDATADIR.to_owned() + "/resonance.gresource")
            .expect("Unable to find resonance.gresource"),
        Ok(_) => match env::current_exe() {
            Ok(path) => {
                let mut resource_path = path;
                resource_path.pop();
                resource_path.push("resonance.gresource");
                gio::Resource::load(&resource_path)
                    .expect("Unable to find resonance.gresource in devenv")
            }
            Err(err) => {
                error!("Unable to find the current path: {}", err);
                process::exit(0x0100);
            }
        },
    };

    gio::resources_register(&resources);

    //Set application name
    glib::set_application_name("Resonance");
    glib::set_program_name(Some("resonance"));

    //Start GStreamer
    gst::init().expect("Unable to init GStreamer");

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = App::new("io.github.nate_xyz.Resonance", &gio::ApplicationFlags::empty());

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    app.run()
}
