/* grid_art.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::subclass::prelude::*;
use gtk::{glib, prelude::*};

use gtk::{gdk_pixbuf::Pixbuf, gdk_pixbuf, gdk, gsk, graphene};

use std::rc::Rc;

use log::debug;

mod imp {use std::cell::{RefCell, Cell};
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default)]
    pub struct GridArt {
        pub size: Cell<i32>,
        pub error: Cell<bool>,
        // pub pixbufs: RefCell<Vec<Pixbuf>>,
        pub texture: RefCell<Option<gdk::Texture>>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GridArt {
        const NAME: &'static str = "GridArt";
        type Type = super::GridArt;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for GridArt {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("populated").build(),
                ]
            });

            SIGNALS.as_ref()
        }

    }

    impl WidgetImpl for GridArt {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(texture) = self.texture.borrow_mut().as_ref() {
                let rect = graphene::Rect::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);
                let rounded_rect = gsk::RoundedRect::from_rect(rect, 10.0);
                snapshot.push_rounded_clip(&rounded_rect);
                snapshot.append_texture(texture, &rect);
                snapshot.pop();

            }
        }
    }

    impl GridArt {}
}

glib::wrapper! {
    pub struct GridArt(ObjectSubclass<imp::GridArt>)
    @extends gtk::Widget;
}

impl GridArt {
    pub fn new(size: i32) -> GridArt {
        let album_art: GridArt =  glib::Object::builder::<GridArt>().build();
        album_art.imp().size.set(size);
        album_art
    }

    pub fn load(&self, pixbufs: Vec<Rc<Pixbuf>>) {
        if pixbufs.len() <= 1 {
            self.load_pixbuf(pixbufs);
        } else {
            self.load_multiple_pixbuf(pixbufs);
        }        
    }

    pub fn load_multiple_pixbuf(&self, pixbufs: Vec<Rc<gdk_pixbuf::Pixbuf>>) {
        debug!("load_multiple_pixbuf");
        let imp = self.imp();

        let full_pixbuf = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, imp.size.get(), imp.size.get()).unwrap();

        let mut pixbuf_array = pixbufs.clone();
        let mut two_by_two = true;
        let mut scaled_size = imp.size.get()/2;
        let mut array_size = 4;
    
        //max size 9
        if pixbuf_array.len() > 9 {
            pixbuf_array = pixbuf_array[0..9].to_vec();
        }

        //determine whether 2x2 or 3x3
        if pixbuf_array.len() > 4 {
            two_by_two = false;
            scaled_size = (imp.size.get() as f32 / 3.0 ).floor() as i32;
            array_size = 9;
        }

        //debug!("load_multiple_pixbuf {} {} {}", two_by_two, scaled_size, array_size);

        //fill extra
        if pixbuf_array.len() == 2 {
            //debug!("EXTENDING ARRAY");
            let mut reverse = pixbuf_array.clone();
            reverse.reverse();
            pixbuf_array = [pixbuf_array.clone(), reverse].concat();
        } else if pixbuf_array.len() < array_size {
            let extension = pixbuf_array.clone()[0..(array_size - pixbuf_array.len())].to_vec();
            pixbuf_array = [pixbuf_array.clone(), extension].concat();
        } 
        
        for (pos, pixbuf) in pixbuf_array.iter().enumerate() {
            let mut offset_x = 0;
            let mut offset_y = 0;
            if two_by_two {
                match pos {
                    1 => {
                        offset_x = scaled_size;
                    },  
                    2 => {
                        offset_y = scaled_size;
                    },
                    3 => {
                        offset_x = scaled_size;
                        offset_y = scaled_size;
                    },
                    _ => {}
                }
            } else {
                match pos {
                    1 => {
                        offset_x = scaled_size;
                    },  
                    2 => {
                        offset_x = scaled_size*2;
                    },
                    3 => {
                        offset_y = scaled_size;
                    },
                    4 => {
                        offset_y = scaled_size;
                        offset_x = scaled_size;
                    },
                    5 => {
                        offset_y = scaled_size;
                        offset_x = scaled_size*2;
                    },
                    6 => {
                        offset_y = scaled_size*2;
                    },
                    7 => {
                        offset_y = scaled_size*2;
                        offset_x = scaled_size;
                    },
                    8 => {
                        offset_y = scaled_size*2;
                        offset_x = scaled_size*2;
                    },
                    _ => {}
                }
            }

            pixbuf.scale_simple(scaled_size, scaled_size, gdk_pixbuf::InterpType::Bilinear)
                .unwrap()
                .composite(
                    &full_pixbuf, offset_x, offset_y, 
                    scaled_size, scaled_size,
                    offset_x as f64, offset_y as f64, 
                    1.0, 1.0, gdk_pixbuf::InterpType::Nearest, 255
                );

        }

        self.add_pixbuf(Some(full_pixbuf));
    }

    pub fn load_pixbuf(&self, pixbuf: Vec<Rc<gdk_pixbuf::Pixbuf>>) {
        let imp = self.imp();
        if pixbuf.len() > 0 {
            let new_pixbuf = match pixbuf[0].scale_simple(imp.size.get(), imp.size.get(), gdk_pixbuf::InterpType::Bilinear) {
                Some(pixbuf) => pixbuf,
                None => {
                    return;
                }
            };
            self.add_pixbuf(Some(new_pixbuf));
        } else {
            self.add_pixbuf(None);
        }
    }

    fn add_pixbuf(&self, pixbuf: Option<gdk_pixbuf::Pixbuf>) {
        let imp = self.imp();
        match pixbuf {
            Some(pixbuf) => {
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                imp.texture.replace(Some(texture));
                //self.pixbuf.replace(Some(pixbuf));
                imp.error.set(false);
            },
            None => {
                imp.error.set(true);
            }
        }
        self.emit_by_name::<()>("populated", &[]);
    }
}