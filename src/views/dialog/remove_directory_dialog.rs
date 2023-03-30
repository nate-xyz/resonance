/* remove_directory_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::cell::RefCell;
use log::error;

use crate::database::DatabaseAction;
use crate::util::database;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/remove_directory_dialog.ui")]
    pub struct RemoveDirectoryDialogPriv {
        pub directory: RefCell<String>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RemoveDirectoryDialogPriv {
        const NAME: &'static str = "RemoveDirectoryDialog";
        type Type = super::RemoveDirectoryDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                directory: RefCell::new("".to_string()),
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

    impl ObjectImpl for RemoveDirectoryDialogPriv {
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

    impl WidgetImpl for RemoveDirectoryDialogPriv {}
    impl WindowImpl for RemoveDirectoryDialogPriv {}
    impl MessageDialogImpl for RemoveDirectoryDialogPriv {}
    impl RemoveDirectoryDialogPriv {}
}

glib::wrapper! {
    pub struct RemoveDirectoryDialog(ObjectSubclass<imp::RemoveDirectoryDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RemoveDirectoryDialog {
    pub fn new(dir_to_remove: String) -> RemoveDirectoryDialog {
        let dialog: RemoveDirectoryDialog = glib::Object::builder::<RemoveDirectoryDialog>().build();
        dialog.imp().directory.replace(dir_to_remove);
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

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();
        let dir_to_remove = imp.directory.borrow().clone();

        if dir_to_remove.is_empty() {
            error!("No directory entered, cannot remove.");
            return;
        }

        if response == "remove" {
            send!(imp.db_sender, DatabaseAction::RemoveDirectory(dir_to_remove));
            self.emit_by_name::<()>("done", &[]);
        }
    }
}
