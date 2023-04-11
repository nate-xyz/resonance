/* add_tracks_to_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::clone, CompositeTemplate};

use crate::util::settings_manager;
use crate::i18n::i18n;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/alpha_dialog.ui")]
    pub struct AlphaDialogPriv {
        #[template_child(id = "message_box")]
        pub message_box: TemplateChild<gtk::Box>,

    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlphaDialogPriv {
        const NAME: &'static str = "AlphaDialog";
        type Type = super::AlphaDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlphaDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for AlphaDialogPriv {}
    impl WindowImpl for AlphaDialogPriv {}
    impl MessageDialogImpl for AlphaDialogPriv {}
    impl AlphaDialogPriv {}
}

glib::wrapper! {
    pub struct AlphaDialog(ObjectSubclass<imp::AlphaDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AlphaDialog {
    pub fn new() -> AlphaDialog {
        let dialog: AlphaDialog = glib::Object::builder::<AlphaDialog>().build();
        dialog
    }

    pub fn initialize(&self) {
        let imp = self.imp();
        self.set_destroy_with_parent(true);

        let msg = &i18n("Resonance is early alpha stage software, there will be bugs.\nIf you find a bug or would like to request a feature, please open an issue on the github repo:\n<a href=\"https://github.com/nate-xyz/resonance/issues\">github.com/nate-xyz/resonance/issues</a>");

        let label = gtk::Label::new(Some(msg));
        label.set_use_markup(true);
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Center);
        label.set_justify(gtk::Justification::Center);

        imp.message_box.append(&label);

        self.connect_response(
            None,
            clone!(@strong self as this => move |_dialog, response| {
                this.dialog_response(response);
            }),
        );
    }

    fn dialog_response(&self, response: &str) {
        if response == "dontshow" {
            let settings = settings_manager();
            _ = settings.set_boolean("show-alpha", false);
        }
    }
}
