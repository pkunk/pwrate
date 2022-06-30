use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::process::Command;
use std::sync::Mutex;
use std::{env, fs};

use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{Align, Application, ApplicationWindow, ComboBoxText};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct PwContextProperties {
    #[serde(rename = "default.clock.rate")]
    rate: u32,
    #[serde(rename = "default.clock.allowed-rates")]
    allowed_rates: Vec<u32>,
}

impl Default for PwContextProperties {
    fn default() -> Self {
        Self {
            rate: 48000,
            allowed_rates: vec![48000],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PwConfig {
    #[serde(rename = "context.properties")]
    properties: PwContextProperties,
}

fn conf() -> &'static Mutex<PwConfig> {
    static INSTANCE: OnceCell<Mutex<PwConfig>> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(Default::default()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/pipewire/pipewire.conf.d";
    let path = match env::var("XDG_CONFIG_HOME") {
        Ok(xdg) => xdg + path,
        Err(_e) => env::var("HOME")? + "/.config" + path,
    };
    let file = "pwrate.conf";
    fs::create_dir_all(&path)?;

    if let Ok(s) = fs::read_to_string(path.to_owned() + "/" + file) {
        *conf().lock()? = serde_json::from_str(&s)?;
    }

    gtk::init()?;
    let application = Application::new(Some("com.github.pkunk.pwrate"), Default::default());
    application.connect_activate(build_ui);
    application.run();

    println!("Hello, world! {:?}", conf());

    let mut file = File::create(path + "/" + file)?;
    file.write_all(&serde_json::to_vec(conf().lock()?.deref())?)?;

    Ok(())
}

fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Pipewire Rate")
        .default_width(350)
        .default_height(70)
        .build();

    let combobox = ComboBoxText::builder()
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    combobox.append(Some("44100"), "44100");
    combobox.append(Some("48000"), "48000");
    combobox.append(Some("88200"), "88200");
    combobox.append(Some("96000"), "96000");
    combobox.append(Some("192000"), "192000");

    combobox.set_active_id(Some(&conf().lock().unwrap().properties.rate.to_string()));

    combobox.connect_changed(clone!(@weak combobox => move |c| {
        if let Some(rate) = c.active_text() {
            if let Ok(rate) = rate.to_string().parse::<u32>() {
                set_rate(rate);
            }
        }
    }));

    window.set_child(Some(&combobox));

    window.show();
}

fn set_rate(rate: u32) {
    conf().lock().unwrap().properties.rate = rate;
    conf().lock().unwrap().properties.allowed_rates = vec![rate];
    //TODO: replace with native implementation
    pw_metadata(
        "clock.allowed-rates",
        "[".to_owned() + &rate.to_string() + "]",
    );
    pw_metadata("clock.rate", rate.to_string());
    pw_metadata("clock.force-rate", rate.to_string());
    pw_metadata("-d", "clock.force-rate");
}

fn pw_metadata<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    let _ = Command::new("pw-metadata")
        .arg("-n")
        .arg("settings")
        .arg("0")
        .arg(key)
        .arg(value)
        .spawn();
}
