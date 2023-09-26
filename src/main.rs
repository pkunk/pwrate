use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::{env, fs};

use gtk::ffi::GTK_INVALID_LIST_POSITION;
use gtk::prelude::*;
use gtk::{Align, Application, ApplicationWindow, Button, CheckButton, DropDown, Label};
use serde::{Deserialize, Serialize};

static FILE: &str = "/pwrate.conf";
static SERVER_PATH: &str = "/pipewire.conf.d";
static PULSE_PATH: &str = "/pipewire-pulse.conf.d";
static PATH: OnceLock<String> = OnceLock::new();

const DEFAULT_RATE: &str = "48000";
static RATES: &[&str] = &["44100", "48000", "88200", "96000", "192000"];

#[derive(Serialize, Deserialize, Debug)]
struct PwContextProperties {
    #[serde(rename = "default.clock.rate")]
    rate: String,
    #[serde(rename = "default.clock.allowed-rates")]
    allowed_rates: BTreeSet<String>,
}

impl Default for PwContextProperties {
    fn default() -> Self {
        Self {
            rate: DEFAULT_RATE.to_owned(),
            allowed_rates: BTreeSet::from_iter(vec![DEFAULT_RATE.to_owned()]),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PwConfig {
    #[serde(rename = "context.properties")]
    properties: PwContextProperties,
}

#[derive(Serialize, Deserialize, Debug)]
struct PwClientStreamProperties {
    #[serde(rename = "channelmix.mix-lfe")]
    mix_lfe: bool,
    #[serde(rename = "channelmix.upmix")]
    upmix: bool,
    #[serde(rename = "channelmix.upmix-method")]
    upmix_method: String,
}

impl Default for PwClientStreamProperties {
    fn default() -> Self {
        Self {
            mix_lfe: false,
            upmix: true,
            upmix_method: "none".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PwClientConfig {
    #[serde(rename = "stream.properties")]
    properties: PwClientStreamProperties,
}

fn conf() -> &'static Mutex<PwConfig> {
    static INSTANCE: OnceLock<Mutex<PwConfig>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(Default::default()))
}
fn client_conf() -> &'static Mutex<PwClientConfig> {
    static INSTANCE: OnceLock<Mutex<PwClientConfig>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(Default::default()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/pipewire";
    let path = match env::var("XDG_CONFIG_HOME") {
        Ok(xdg) => xdg + path,
        Err(_e) => env::var("HOME")? + "/.config" + path,
    };
    PATH.set(path)?;

    if let Ok(s) = fs::read_to_string(PATH.get().unwrap().to_owned() + SERVER_PATH + FILE) {
        *conf().lock()? = serde_json::from_str(&s)?;
    }

    if let Ok(s) = fs::read_to_string(PATH.get().unwrap().to_owned() + PULSE_PATH + FILE) {
        *client_conf().lock()? = serde_json::from_str(&s)?;
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

    let cnt_rate = gtk::Box::builder()
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

    cnt_rate.append(&lbl_rate);

    let dd_rate = DropDown::builder()
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Center)
        .valign(Align::Center)
        .model(&gtk::StringList::new(RATES))
        .build();

    {
        let active_rate = &conf().lock().unwrap().properties.rate;
        for (i, rate) in RATES.iter().enumerate() {
            if rate == active_rate {
                dd_rate.set_selected(i as u32);
                break;
            }
        }
    }

    dd_rate.connect_selected_notify(move |dd_rate| {
        let selected = dd_rate.selected();
        if selected != GTK_INVALID_LIST_POSITION {
            let rate = RATES[selected as usize].to_string();
            conf().lock().unwrap().properties.rate = rate;
            set_rate();
        }
    });

    cnt_rate.append(&dd_rate);

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

    cnt_rate.append(&btn_save);

    container.append(&cnt_rate);

    let lbl_rates = Label::builder()
        .label("Allowed rates")
        .margin_top(10)
        .margin_bottom(20)
        .margin_start(40)
        .halign(Align::Start)
        .build();
    let rates = &conf().lock().unwrap().properties.allowed_rates;

    container.append(&lbl_rates);

    for rate in RATES {
        let margin = if rate.len() < 6 { "    " } else { "  " }.to_owned();
        let chb = CheckButton::builder()
            .label(&(margin + rate))
            .active(rates.contains(*rate))
            .margin_start(20)
            .build();
        chb.connect_toggled(move |chb| {
            {
                let rates = &mut conf().lock().unwrap().properties.allowed_rates;
                let rate = rate.to_string();
                if chb.is_active() {
                    rates.insert(rate);
                } else {
                    rates.remove(&rate);
                }
            }
            set_rates();
        });
        container.append(&chb);
    }

    let lbl_mix = Label::builder()
        .label("Surround")
        .margin_top(30)
        .margin_start(40)
        .halign(Align::Start)
        .build();
    container.append(&lbl_mix);

    let cnt_mix = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .halign(Align::Start)
        .valign(Align::Center)
        .spacing(10)
        .build();

    let client_properties = &client_conf().lock().unwrap().properties;

    let chb_mix_none = CheckButton::builder()
        .label("None")
        .active(client_properties.upmix_method == "none")
        .margin_start(10)
        .build();

    let chb_mix_simple = CheckButton::builder()
        .label("Simple")
        .active(client_properties.upmix_method == "simple")
        .margin_start(10)
        .build();
    chb_mix_simple.set_group(Some(&chb_mix_none));

    let chb_mix_psd = CheckButton::builder()
        .label("PSD")
        .active(client_properties.upmix_method == "psd")
        .margin_start(10)
        .build();
    chb_mix_psd.set_group(Some(&chb_mix_none));

    chb_mix_none.connect_toggled(move |chb_mix_none| {
        if chb_mix_none.is_active() {
            {
                let client_properties = &mut client_conf().lock().unwrap().properties;
                client_properties.upmix_method = "none".to_owned();
                client_properties.upmix = false;
            }
            let _ = save_client_config();
            restart_pulse();
        }
    });

    chb_mix_simple.connect_toggled(move |chb_mix_simple| {
        if chb_mix_simple.is_active() {
            {
                let client_properties = &mut client_conf().lock().unwrap().properties;
                client_properties.upmix_method = "simple".to_owned();
                client_properties.upmix = true;
            }
            let _ = save_client_config();
            restart_pulse();
        }
    });

    chb_mix_psd.connect_toggled(move |chb_mix_psd| {
        if chb_mix_psd.is_active() {
            {
                let client_properties = &mut client_conf().lock().unwrap().properties;
                client_properties.upmix_method = "psd".to_owned();
                client_properties.upmix = true;
            }
            let _ = save_client_config();
            restart_pulse();
        }
    });

    cnt_mix.append(&chb_mix_none);
    cnt_mix.append(&chb_mix_simple);
    cnt_mix.append(&chb_mix_psd);

    container.append(&cnt_mix);

    let chb_mix_lfe = CheckButton::builder()
        .label("  Mix LFE")
        .active(client_properties.mix_lfe)
        .margin_start(20)
        .build();
    chb_mix_lfe.connect_toggled(move |chb_mix_lfe| {
        {
            client_conf().lock().unwrap().properties.mix_lfe = chb_mix_lfe.is_active();
        }
        let _ = save_client_config();
        restart_pulse();
    });

    container.append(&chb_mix_lfe);

    window.set_child(Some(&container));

    window.show();
}

fn set_rate() {
    let rate = &conf().lock().unwrap().properties.rate;
    pw_metadata("clock.rate", rate);
    pw_metadata("clock.force-rate", rate);
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
        .status();
}

fn save_config() -> Result<(), Box<dyn std::error::Error>> {
    let path = PATH.get().expect("save path should be already set");
    fs::create_dir_all(path.to_owned() + SERVER_PATH)?;

    let mut file = File::create(path.to_owned() + SERVER_PATH + FILE)?;
    file.write_all(&serde_json::to_vec_pretty(conf().lock()?.deref())?)?;
    Ok(())
}

fn save_client_config() -> Result<(), Box<dyn std::error::Error>> {
    let path = PATH.get().expect("save path should be already set");
    fs::create_dir_all(path.to_owned() + PULSE_PATH)?;

    let mut file = File::create(path.to_owned() + PULSE_PATH + FILE)?;
    file.write_all(&serde_json::to_vec_pretty(client_conf().lock()?.deref())?)?;

    fs::create_dir_all(path.to_owned() + "/client.conf.d")?;
    let mut file = File::create(path.to_owned() + "/client.conf.d" + FILE)?;
    file.write_all(&serde_json::to_vec_pretty(client_conf().lock()?.deref())?)?;

    Ok(())
}

fn restart_pulse() {
    let _ = Command::new("systemctl")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .arg("--user")
        .arg("try-reload-or-restart")
        .arg("pipewire-pulse.service")
        .status();
}
