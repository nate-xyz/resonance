/* util.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use gtk::gio;
use gtk::{gdk, glib, gdk_pixbuf::Pixbuf};

use std::rc::Rc;

use color_thief::{get_palette, ColorFormat};
use scraper::{Html, Selector};
use reqwest::header::{HeaderMap, USER_AGENT};

use crate::views::art::rounded_album_art::RoundedAlbumArt;
use crate::views::window::Window;

use super::model::model::Model;
use super::database::Database;
use super::app::App;
use super::player::player::Player;

pub fn window() -> Option<Window> {
    let app = gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<gtk::Application>()
        .ok()?;

    let win = app
        .active_window()?
        .downcast::<Window>()
        .ok()?;
    
    Some(win)
}

pub fn fetch_artist_image_discog(artist_name: &str) -> Option<(String, Vec<u8>)> {
    let url = format!("https://www.discogs.com/search/?q={}&type=artist", urlencoding::encode(artist_name));
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0".parse().unwrap());

    let response = reqwest::blocking::Client::builder()
    .default_headers(headers.clone())
        .build()
        .unwrap()
        .get(&url)
        .send()
        .ok()?;

    let html_content = response.text().ok()?;
    let document = Html::parse_document(&html_content);

    let search_results_selector = Selector::parse("#search_results").unwrap();
    let card_selector = Selector::parse(".card").unwrap();
    let thumbnail_link_selector = Selector::parse(".thumbnail_link").unwrap();

    let artist_link = document.select(&search_results_selector)
        .next()?
        .select(&card_selector)
        .next()?
        .select(&thumbnail_link_selector)
        .next()?
        .value()
        .attr("href")?;

    let url = format!("https://www.discogs.com{}/images", artist_link);
    //debug!("{}", url);

    let response = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
        .get(&url)
        .send()
        .ok()?;

    let html_content = response.text().ok()?;
    let document = Html::parse_document(&html_content);

    let view_images_selector = Selector::parse("#view_images").unwrap();
    let thumbnail_link_selector = Selector::parse(".thumbnail_link").unwrap();

    let image_url = document.select(&view_images_selector)
        .next()?
        .select(&thumbnail_link_selector)
        .next()?
        .select(&Selector::parse("img").unwrap())
        .next()?
        .value()
        .attr("src")?;

    let img_bytes = reqwest::blocking::get(image_url).ok()?.bytes().ok()?.to_vec();

    Some((image_url.to_owned(), img_bytes))
}



pub fn load_cover_art_pixbuf(cover_art_id: i64, size: i32) -> Result<RoundedAlbumArt, String> {
    let cover_art = model().cover_art(cover_art_id)?;
    let pixbuf = cover_art.pixbuf()?;
    let art = RoundedAlbumArt::new(size);
    art.load(pixbuf);
    Ok(art)
}

pub fn settings_manager() -> gio::Settings {
    // // We ship a single schema for both default and development profiles
    // let app_id = APPLICATION_ID.trim_end_matches(".Devel");
    let app_id = "io.github.nate_xyz.Resonance";
    gio::Settings::new(app_id)
}


pub fn player() -> Rc<Player> {
    gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<App>()
        .unwrap()
        .player()
        .clone()
}

pub fn model() -> Rc<Model>{
    gio::Application::default()
    .expect("Failed to retrieve application singleton")
    .downcast::<App>()
    .unwrap()
    .model()
}

pub fn database() -> Rc<Database>{
    gio::Application::default()
    .expect("Failed to retrieve application singleton")
    .downcast::<App>()
    .unwrap()
    .database()
}

#[allow(dead_code)]
pub fn active_window() -> Option<gtk::Window> {
    let app = gio::Application::default()
    .expect("Failed to retrieve application singleton")
    .downcast::<gtk::Application>()
    .unwrap();

    let win = app
    .active_window();

    win
}

pub fn get_child_by_index<U, W>(w: &U, pos: usize) -> W
where
    U: WidgetExt,
    W: IsA<glib::Object>,
{
    w.observe_children()
        .item(pos as u32)
        .unwrap()
        .clone()
        .downcast::<W>()
        .unwrap()
}

pub fn seconds_to_string(duration: f64) -> String {
    let duration = duration as i32;
    let seconds = duration;
    let minutes = seconds / 60;
    let seconds = seconds % 60;

    format!("{}:{:02}", minutes, seconds)
}

pub fn seconds_to_string_longform(duration: f64) -> String {
    let hours = duration / 3600.0;
    let minutes = (duration - (3600.0 * hours.floor())) / 60.0;
    let seconds = duration - (3600.0 * hours) - (minutes * 60.0);

    let mut desc = String::new();

    let hours_u64= hours.floor() as u64;
    
    if hours_u64 > 1 {
        desc.push_str(&format!("{} hours", hours_u64));
    } else if hours_u64 == 1 {
        desc.push_str(&format!("1 hour"));
    }

    let minutes_u64 = minutes.round() as u64;


    if minutes.floor() > 0.0 {
        if !desc.is_empty() {
            desc.push_str(" and ");
        }

        if minutes < 1.0 {
            if seconds > 1.0 {
                let seconds = seconds.round() as u64;
                desc.push_str(&format!("{} seconds", seconds));
            }
        } else if minutes_u64 > 1 {
            desc.push_str(&format!("{} minutes", minutes_u64));
        } else {
            desc.push_str(&format!("1 minute"));
        }


    }

    desc
}

fn color_format(has_alpha: bool) -> ColorFormat {
    if has_alpha {
        ColorFormat::Rgba
    } else {
        ColorFormat::Rgb
    }
}

pub fn load_palette(pixbuf: &Pixbuf) -> Option<Vec<gdk::RGBA>> {
    if let Ok(palette) = get_palette(
        pixbuf.pixel_bytes().unwrap().as_ref(),
        color_format(pixbuf.has_alpha()),
        10,
        3,
    ) {
        let colors: Vec<gdk::RGBA> = palette
            .iter()
            .map(|c| {
                gdk::RGBA::new(
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                    1.0,
                )
            })
            .collect();

        return Some(colors);
    }

    None
}


pub(crate) fn win(widget: &gtk::Widget) -> gtk::Window {
    widget.root().unwrap().downcast::<gtk::Window>().unwrap()
}

