/* track_entry.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, rc::Rc};

use crate::model::track::Track;
use crate::util::{player, seconds_to_string};
use crate::i18n::i18n_k;

mod imp {
    use super::*;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_track_entry.ui")]
    pub struct TrackEntryPriv {
        #[template_child(id = "content_box")]
        pub content_box: TemplateChild<gtk::Box>,

        #[template_child(id = "track_button")]
        pub track_button: TemplateChild<gtk::Button>,

        #[template_child(id = "play_icon")]
        pub play_icon: TemplateChild<gtk::Image>,

        #[template_child(id = "number_label")]
        pub number_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_name_label")]
        pub track_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "time_label")]
        pub time_label: TemplateChild<gtk::Label>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        pub track: RefCell<Option<Rc<Track>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrackEntryPriv {
        const NAME: &'static str = "TrackEntry";
        type Type = super::TrackEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for TrackEntryPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for TrackEntryPriv {}
    impl BoxImpl for TrackEntryPriv {}
    impl TrackEntryPriv {}
}

glib::wrapper! {
    pub struct TrackEntry(ObjectSubclass<imp::TrackEntryPriv>)
    @extends gtk::Box, gtk::Widget;

}

impl TrackEntry {
    pub fn new(track: Rc<Track>, track_n: i32, disc_n: i32, char_width: i32) -> TrackEntry {
        let track_button: TrackEntry = glib::Object::builder::<TrackEntry>().build();
        track_button.construct(track, track_n, disc_n, char_width);
        track_button
    }

    fn construct(&self, track: Rc<Track>, track_n: i32, disc_n: i32, char_width: i32) {
        let imp = self.imp();

        //imp.popover.set_parent(self);

        imp.track_name_label.set_max_width_chars(char_width);

        imp.track_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().clear_play_track(this.track());
            })
        );

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().add_track(this.track());
            })
        );
   
        imp.track_button.set_tooltip_text(Some(&i18n_k("Play {track_title}", &[("track_title", &track.title())])));
        imp.add_button.set_tooltip_text(Some(&i18n_k("Add {track_title} to Playlist", &[("track_title", &track.title())])));
        
        imp.track_name_label.set_label(&track.title());
        imp.time_label.set_label(&seconds_to_string(track.duration()));
        
        if disc_n <= 1 {
            imp.number_label.set_label(&format!("{:02} - ", track_n));
        } else {
            imp.number_label.set_label(&format!("{:02}:{:02} - ", disc_n+1, track_n));
        }

        //imp.popover.set_menu_model(Some(track.menu_model()));
        
        imp.track.replace(Some(track));
                
        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();
                imp.add_button.show();
                imp.play_icon.show();
            })
        );
        ctrl.connect_leave(clone!(@strong self as this => move |_controller| {
                let imp = this.imp();
                imp.add_button.hide();
                imp.play_icon.hide();
            })
        );
        self.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, _y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }

    fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }
}
    