/* queue_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, gdk, glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, cell::Cell, rc::Rc};
use log::{debug, error};

use crate::model::track::Track;
use crate::views::{
    scale::Scale,
    window::WindowPage,
    volume_widget::VolumeWidget,
};
use crate::player::queue::RepeatMode;
use crate::util::{player, model, seconds_to_string};
use crate::i18n::i18n;

mod imp {
    use super::*;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/queue_page.ui")]
    pub struct QueuePagePriv {
        #[template_child(id = "track_info_box")]
        pub track_info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "scale_clamp")]
        pub scale_clamp: TemplateChild<adw::Clamp>,

        #[template_child(id = "volume_widget")]
        pub volume_widget: TemplateChild<VolumeWidget>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "previous_button")]
        pub previous_button: TemplateChild<gtk::Button>,
 
        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "next_button")]
        pub next_button: TemplateChild<gtk::Button>,

        #[template_child(id = "spent_time_label")]
        pub spent_time_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "play_pause_image")]
        pub play_pause_image: TemplateChild<gtk::Image>,

        #[template_child(id = "track_name_label")]
        pub track_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label")]
        pub album_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_name_label")]
        pub artist_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "repeat_button")]
        pub repeat_button: TemplateChild<gtk::Button>,

        #[template_child(id = "shuffle_button")]
        pub shuffle_button: TemplateChild<gtk::Button>,

        #[template_child(id = "loop_button")]
        pub loop_button: TemplateChild<gtk::Button>,

        #[template_child(id = "show_queue_button")]
        pub show_queue_button: TemplateChild<gtk::Button>,

        #[template_child(id = "progress_scale")]
        pub progress_scale: TemplateChild<Scale>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,
        
        pub picture: RefCell<Option<gtk::Picture>>,
        pub track: RefCell<Option<Rc<Track>>>,
        pub window_page: Cell<WindowPage>,

    }

    #[glib::object_subclass]
    impl ObjectSubclass for QueuePagePriv {
        const NAME: &'static str = "QueuePage";
        type Type = super::QueuePage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for QueuePagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for QueuePagePriv {}
    impl BoxImpl for QueuePagePriv {}
    impl QueuePagePriv {}
}

glib::wrapper! {
    pub struct QueuePage(ObjectSubclass<imp::QueuePagePriv>)
    @extends gtk::Box, gtk::Widget;
}


impl QueuePage {
    pub fn new() -> QueuePage {
        let queue_page: QueuePage = glib::Object::builder::<QueuePage>().build();
        queue_page
    }

    pub fn initialize(&self) {
        self.scale().set_id("QUEUE SCALE.");
        self.scale().initialize();

        self.button_connections();
        self.bind_state();
        self.create_menu();

        self.imp().popover.set_parent(self);

        let ctrl = gtk::GestureClick::new();
        ctrl.connect_unpaired_release(
            clone!(@strong self as this => move |_gesture_click, x, y, button, _sequence| {
                let imp = this.imp();
                if button == gdk::BUTTON_SECONDARY {
                    let height = this.height();
                    let width = this.width() / 2;
                    let y = y as i32;
                    let x = x as i32;

                    //let offset = imp.popover.offset();
                    //debug!("{} {} {:?}", width, x, offset);

                    imp.popover.set_offset(x - width, y - height);

                    //let offset = imp.popover.offset();
                    //debug!("{} {} {:?}", width, x, offset);

                    imp.popover.popup();
                }
            })
        );
        self.add_controller(ctrl);   
    }

    // Bind the PlayerState to the UI
    fn bind_state(&self) {
        let player = player();
        // Update current track
        
        player.state().connect_notify_local(
            Some("song"),
            clone!(@weak self as this => move |_, _| {
                this.update_current_track();
            }),
        );

        player.state().connect_notify_local(
            Some("position"),
            clone!(@strong self as this => move |_, _| {
                this.update_position();
            }),
        );

        player.state().connect_notify_local(
            Some("playing"),
            clone!(@weak self as this => move |_, _| {
                this.sync_playing();
            }),
        );

        let scale = self.scale();
        scale.connect_notify_local(
            Some("time-position"),
            clone!(@weak self as this => move |_, _| {
                this.update_position();
            }),
        );

        player.state().connect_local(
            "queue-repeat-mode",
            false,
            clone!(@strong self as this => move |value| {
                let imp = this.imp();
                let mode = value.get(1).unwrap().get::<RepeatMode>().ok().unwrap();
                debug!("repeat mode {:?}", mode);
                match mode {
                    RepeatMode::Normal => {
                        imp.shuffle_button.remove_css_class("suggested-action");
                        imp.loop_button.remove_css_class("suggested-action");
                        imp.repeat_button.remove_css_class("suggested-action");
                    },
                    RepeatMode::Loop => {
                        imp.shuffle_button.remove_css_class("suggested-action");
                        imp.loop_button.add_css_class("suggested-action");
                        imp.repeat_button.remove_css_class("suggested-action");
                    },
                    RepeatMode::LoopSong => {
                        imp.shuffle_button.remove_css_class("suggested-action");
                        imp.loop_button.remove_css_class("suggested-action");
                        imp.repeat_button.add_css_class("suggested-action");
                    },
                    RepeatMode::Shuffle => {
                        imp.shuffle_button.add_css_class("suggested-action");
                        imp.loop_button.remove_css_class("suggested-action");
                        imp.repeat_button.remove_css_class("suggested-action");
                    },
                }
                None
            }),
        );

    }

    fn sync_playing(&self) {
        let imp = self.imp();
        if player().state().playing() {
            debug!("playing, setting button to pause");
            imp.play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
            imp.play_button.set_tooltip_text(Some(&i18n("Pause")));
        } else {
            debug!("not playing, setting button to play");
            imp.play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
            imp.play_button.set_tooltip_text(Some(&i18n("Play")));
        }
    }

    fn button_connections(&self) {
        let imp = self.imp();
        imp.play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().toggle_play_pause();
            })
        );

        imp.previous_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().prev();
            })
        );

        imp.next_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().next();
            })
        );

        imp.loop_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::Loop);
            })
        );

        imp.repeat_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::LoopSong);
            })
        );

        imp.shuffle_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::Shuffle);
            })
        );

    }

    fn update_current_track(&self) {
        let imp = self.imp();
        let player = player();
        let state = player.state();
        imp.track.replace(state.current_track());
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();

        if let Some(track) = imp.track.borrow().as_ref() {
            imp.track_info_box.show();
            imp.scale_clamp.show();
            
            imp.spent_time_label.set_label("0:00");
            imp.track_name_label.set_label(&track.title());
            imp.album_name_label.set_label(&track.album());
            imp.artist_name_label.set_label(&track.artist());
            imp.duration_label.set_label(&seconds_to_string(track.duration()));
            
            self.sync_prev_next();
            self.load_art(track.cover_art_option());
        } else {
            imp.track_info_box.hide();
            imp.scale_clamp.hide();
            imp.art_bin.hide();
            //imp.art_bin.set_child(gtk::Widget::NONE);
            imp.previous_button.set_sensitive(false);
            imp.next_button.set_sensitive(false);
        }
    }

    fn load_art(&self, art: Option<i64>) {
        let imp = self.imp();

        if let Some(id) = art {
            if let Ok(art) = self.add_art(id, 425) {
                imp.art_bin.set_child(Some(&art));
                imp.art_bin.show();
                return;
            }
        }

        //imp.art_bin.set_child(gtk::Widget::NONE);
        imp.art_bin.hide();
    }

    fn add_art(&self, cover_art_id: i64, _size: i32) -> Result<gtk::Picture, String> {
        let cover_art = model().cover_art(cover_art_id)?;
        let pixbuf = cover_art.pixbuf()?;

        let picture = if pixbuf.height() < 668 {
            let scaled_up = pixbuf.scale_simple(668, 668, gtk::gdk_pixbuf::InterpType::Bilinear)
            .unwrap();
            gtk::Picture::for_pixbuf(&scaled_up)
        } else {
            gtk::Picture::for_pixbuf(&pixbuf)
        };

        // picture.set_can_shrink(true);
        picture.set_css_classes(&[&"card"]);
        picture.set_halign(gtk::Align::Center);
        picture.set_halign(gtk::Align::Center);
        picture.set_width_request(300);
        picture.set_height_request(300);

        picture.set_content_fit(gtk::ContentFit::Cover);
        Ok(picture)
    }

    fn sync_prev_next(&self) {
        let imp = self.imp();
        let player = player();
        let has_next = player.has_next();
        let has_previous = player.has_previous();
        imp.play_button.set_sensitive(has_next);
        imp.previous_button.set_sensitive(has_previous);
        imp.next_button.set_sensitive(has_next);
    }

    fn update_position(&self) {
        let position = player().state().position() as f64;
        self.imp().spent_time_label.set_label(&seconds_to_string(position));
    }

    pub fn show_queue_button(&self) -> &gtk::Button {
        self.imp().show_queue_button.as_ref()
    }

    fn scale(&self) -> &Scale {
        &self.imp().progress_scale
    }

    pub fn set_window_page(&self, state: WindowPage) {
        self.imp().window_page.set(state);
    }

    fn _window_page(&self) -> WindowPage {
        self.imp().window_page.get()
    }

    fn create_menu(&self) {
        let imp = self.imp();
    
        let menu = gio::Menu::new();
    
        let menu_item = gio::MenuItem::new(Some(&format!("Toggle Play Pause")), None);
        menu_item.set_action_and_target_value(Some("win.toggle-play-pause"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Prev")), None);
        menu_item.set_action_and_target_value(Some("win.prev"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Next")), None);
        menu_item.set_action_and_target_value(Some("win.next"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("End Queue")), None);
        menu_item.set_action_and_target_value(Some("win.end-queue"), None);
        menu.append_item(&menu_item);

        imp.popover.set_menu_model(Some(&menu));
    }

}
    