use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::{env, fs};

use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{Align, Application, ApplicationWindow, Button, CheckButton, ComboBoxText, Label};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

static FILE: &str = "/pwrate.conf";
static PATH: OnceCell<String> = OnceCell::new();

#[derive(Serialize, Deserialize, Debug)]
struct PwContextProperties {
    #[serde(rename = "default.clock.rate")]
    rate: u32,
    #[serde(rename = "default.clock.allowed-rates")]
    allowed_rates: BTreeSet<u32>,
}

impl Default for PwContextProperties {
    fn default() -> Self {
        Self {
            rate: 48000,
            allowed_rates: BTreeSet::from_iter(vec![48000].into_iter()),
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
    PATH.set(path)?;

    if let Ok(s) = fs::read_to_string(PATH.get().unwrap().to_owned() + FILE) {
        *conf().lock()? = serde_json::from_str(&s)?;
    }

    gtk::init()?;
    let application = Application::new(Some("com.github.pkunk.pwrate"), Default::default());
    application.connect_activate(build_ui);
    application.run();

    Ok(())
}

fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Pipewire Rate")
        .default_width(350)
        .default_height(70)
        .build();

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(Align::Center)
        .valign(Align::Center)
        .margin_bottom(10)
        .build();

    let container_rate = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Center)
        .valign(Align::Center)
        .spacing(10)
        .build();

    let lbl_rate = Label::builder().label("Default rate:").build();

    container_rate.append(&lbl_rate);

    let cmb_rate = ComboBoxText::builder()
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    cmb_rate.append(Some("44100"), "44100");
    cmb_rate.append(Some("48000"), "48000");
    cmb_rate.append(Some("88200"), "88200");
    cmb_rate.append(Some("96000"), "96000");
    cmb_rate.append(Some("192000"), "192000");

    cmb_rate.set_active_id(Some(&conf().lock().unwrap().properties.rate.to_string()));

    cmb_rate.connect_changed(clone!(@weak cmb_rate => move |c| {
        if let Some(rate) = c.active_text() {
            if let Ok(rate) = rate.to_string().parse::<u32>() {
                conf().lock().unwrap().properties.rate = rate;
                set_rate();
            }
        }
    }));

    container_rate.append(&cmb_rate);

    let btn_save = Button::builder()
        .label("Save Config")
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    btn_save.connect_clicked(|_| {
        save_config().unwrap_or_else(|e| {
            eprintln!("Failed to save config: {}", e);
        });
    });

    container_rate.append(&btn_save);

    container.append(&container_rate);

    let lbl_rates = Label::builder().label("Allowed rates:").build();
    let rates = &conf().lock().unwrap().properties.allowed_rates;

    let chb_44100 = CheckButton::builder()
        .label("  44100")
        .active(rates.contains(&44100))
        .margin_start(10)
        .build();
    chb_44100.connect_toggled(clone!(@weak chb_44100 => move |c| {
        {
            let rates = &mut conf().lock().unwrap().properties.allowed_rates;
            if c.is_active() {
                rates.insert(44100);
            } else {
                rates.remove(&44100);
            }
        }
        set_rates();
    }));

    let chb_48000 = CheckButton::builder()
        .label("  48000")
        .active(rates.contains(&48000))
        .margin_start(10)
        .build();
    chb_48000.connect_toggled(clone!(@weak chb_48000 => move |c| {
        {
            let rates = &mut conf().lock().unwrap().properties.allowed_rates;
            if c.is_active() {
                rates.insert(48000);
            } else {
                rates.remove(&48000);
            }
        }
        set_rates();
    }));

    let chb_88200 = CheckButton::builder()
        .label("  88200")
        .active(rates.contains(&88200))
        .margin_start(10)
        .build();
    chb_88200.connect_toggled(clone!(@weak chb_88200 => move |c| {
        {
            let rates = &mut conf().lock().unwrap().properties.allowed_rates;
            if c.is_active() {
                rates.insert(88200);
            } else {
                rates.remove(&88200);
            }
        }
        set_rates();
    }));

    let chb_96000 = CheckButton::builder()
        .label("  96000")
        .active(rates.contains(&96000))
        .margin_start(10)
        .build();
    chb_96000.connect_toggled(clone!(@weak chb_96000 => move |c| {
        {
            let rates = &mut conf().lock().unwrap().properties.allowed_rates;
            if c.is_active() {
                rates.insert(96000);
            } else {
                rates.remove(&96000);
            }
        }
        set_rates();
    }));

    let chb_192000 = CheckButton::builder()
        .label(" 192000")
        .active(rates.contains(&192000))
        .margin_start(10)
        .build();
    chb_192000.connect_toggled(clone!(@weak chb_192000 => move |c| {
        {
            let rates = &mut conf().lock().unwrap().properties.allowed_rates;
            if c.is_active() {
                rates.insert(192000);
            } else {
                rates.remove(&192000);
            }
        }
        set_rates();
    }));

    container.append(&lbl_rates);
    container.append(&chb_44100);
    container.append(&chb_48000);
    container.append(&chb_88200);
    container.append(&chb_96000);
    container.append(&chb_192000);

    window.set_child(Some(&container));

    window.show();
}

fn set_rate() {
    let rate = conf().lock().unwrap().properties.rate;
    pw_metadata("clock.rate", rate.to_string());
    pw_metadata("clock.force-rate", rate.to_string());
    pw_metadata("-d", "clock.force-rate");
}

fn set_rates() {
    let rates = &conf().lock().unwrap().properties.allowed_rates;
    pw_metadata("clock.allowed-rates", serde_json::to_string(rates).unwrap());
}

fn pw_metadata<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    //TODO: replace with native implementation
    let _ = Command::new("pw-metadata")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .arg("-n")
        .arg("settings")
        .arg("0")
        .arg(key)
        .arg(value)
        .spawn();
}

fn save_config() -> Result<(), Box<dyn std::error::Error>> {
    let path = PATH.get().expect("save path should be already set");
    fs::create_dir_all(path)?;

    let mut file = File::create(path.to_owned() + FILE)?;
    file.write_all(&serde_json::to_vec(conf().lock()?.deref())?)?;
    Ok(())
}
