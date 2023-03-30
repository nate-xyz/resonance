/* confirm_rename_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::cell::{Cell, RefCell};
use log::error;

use crate::database::DatabaseAction;
use crate::util::database;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    
    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/confirm_rename_playlist_dialog.ui")]
    pub struct ConfirmRenamePlaylistDialogPriv {
        #[template_child(id = "rename_title_box")]
        pub rename_title_box: TemplateChild<gtk::Box>,

        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "rename_desc_box")]
        pub rename_desc_box: TemplateChild<gtk::Box>,

        #[template_child(id = "desc_label")]
        pub desc_label: TemplateChild<gtk::Label>,

        pub title: RefCell<Option<String>>,
        pub desc: RefCell<Option<String>>,
        pub playlist_id: Cell<i64>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConfirmRenamePlaylistDialogPriv {
        const NAME: &'static str = "ConfirmRenamePlaylistDialog";
        type Type = super::ConfirmRenamePlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                rename_title_box: TemplateChild::default(),
                title_label: TemplateChild::default(),
                rename_desc_box: TemplateChild::default(),
                desc_label: TemplateChild::default(),
                title: RefCell::new(None),
                desc: RefCell::new(None),
                playlist_id: Cell::new(-1),
                db_sender: database().sender(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConfirmRenamePlaylistDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("done").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for ConfirmRenamePlaylistDialogPriv {}
    impl WindowImpl for ConfirmRenamePlaylistDialogPriv {}
    impl MessageDialogImpl for ConfirmRenamePlaylistDialogPriv {}
    impl ConfirmRenamePlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct ConfirmRenamePlaylistDialog(ObjectSubclass<imp::ConfirmRenamePlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ConfirmRenamePlaylistDialog {
    pub fn new(playlist_id: i64, new_title: Option<String>, new_description: Option<String>) -> ConfirmRenamePlaylistDialog {
        let dialog: ConfirmRenamePlaylistDialog = glib::Object::builder::<ConfirmRenamePlaylistDialog>().build();
        dialog.load(playlist_id, new_title, new_description);
        dialog
    }

    pub fn initialize(&self) {
        self.set_destroy_with_parent(true);
        self.connect_response(
            None,
            clone!(@strong self as this => move |_dialog, response| {
                this.dialog_response(response);
            }),
        );
    }

    fn load(&self, playlist_id: i64, new_title: Option<String>, new_description: Option<String>) {
        let imp = self.imp();
        imp.playlist_id.set(playlist_id);

        match new_title.clone() {
            Some(title) => imp.title_label.set_label(title.as_str()),
            None => imp.rename_title_box.hide(),
        }

        match new_description.clone() {
            Some(desc) => imp.desc_label.set_label(desc.as_str()),
            None => imp.rename_desc_box.hide(),
        }

        imp.title.replace(new_title);
        imp.desc.replace(new_description);
    }

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();
        let playlist_id = imp.playlist_id.get();
        if playlist_id == -1 {
            return;
        }
        if response == "change" {
            send!(imp.db_sender, DatabaseAction::ChangePlaylistTitleAndOrDescription((playlist_id, imp.title.take(), imp.desc.take())));
        }
        self.emit_by_name::<()>("done", &[]);
    }

}
