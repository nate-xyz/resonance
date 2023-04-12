/* app.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, glib::clone};

use std::rc::Rc;

use crate::config::VERSION;
use crate::Window;
use crate::database::Database;
use crate::model::model::Model;
use crate::player::player::Player;
use crate::views::preferences_window::PreferencesWindow;
use crate::i18n::i18n;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct AppPriv {
        pub database: Rc<Database>,
        pub model: Rc<Model>,
        pub player: Rc<Player>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppPriv {
        const NAME: &'static str = "App";
        type Type = super::App;
        type ParentType = adw::Application;

        fn new() -> Self {
            let database = Database::new();
            let model = Model::new();

            let receiver = database.imp().model_receiver.borrow_mut().take().unwrap();
            receiver.attach(
                None,
                clone!(@strong model as this => move |action| this.process_action(action)),
            );
            
            model.load_database(database.clone());           
            Self {
                database,
                model: Rc::new(model),
                player: Player::new(),
            }
        }
    }

    impl ObjectImpl for AppPriv {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for AppPriv {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            // Get the current window or create one if necessary
            self.parent_activate();
            let app = &self.obj();
            let window = if let Some(window) = app.active_window() {
                window
            } else {
                let window = Window::new(app);
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for AppPriv {}
    impl AdwApplicationImpl for AppPriv {}
    impl AppPriv {}
}

glib::wrapper! {
    pub struct App(ObjectSubclass<imp::AppPriv>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl App {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| app.show_preferences())
            .build();

        self.add_action_entries([quit_action, about_action, preferences_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .modal(true)
            .application_name("Resonance")
            .application_icon("io.github.nate_xyz.Resonance")
            .version(VERSION)
            .developers(vec!["nate-xyz"])
            .copyright("© 2023 nate-xyz")
            .license_type(gtk::License::Gpl30Only)
            .website("https://github.com/nate-xyz/resonance")
            .issue_url("https://github.com/nate-xyz/resonance/issues")
            .build();

        // Translator credits. Replace "translator-credits" with your name/username, and optionally an email or URL. 
        // One name per line, please do not remove previous names.
        about.set_translator_credits(&i18n("translator-credits"));

        about.present();
    }

    fn show_preferences(&self) {
        let preferences = PreferencesWindow::new();
        let window = self.active_window().unwrap();
        preferences.set_transient_for(Some(&window));
        preferences.show();
    }

    pub fn model(&self) -> Rc<Model> {
        self.imp().model.clone()
    }
    
    pub fn database(&self) -> Rc<Database> {
        self.imp().database.clone()
    }

    pub fn player(&self) -> Rc<Player> {
        self.imp().player.clone()
    }
}
