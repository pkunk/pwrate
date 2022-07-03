use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use pipewire::metadata::Metadata;
use pipewire::prelude::ReadableDict;
use pipewire::proxy::{Listener, ProxyListener, ProxyT};
use pipewire::types::ObjectType;
use pipewire::*;

struct Proxies {
    proxies_t: HashMap<u32, Box<Metadata>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl Proxies {
    fn new() -> Self {
        Self {
            proxies_t: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn add_proxy_t(&mut self, proxy_t: Box<Metadata>, listener: Box<dyn Listener>) {
        let proxy_id = {
            let proxy = proxy_t.upcast_ref();
            proxy.id()
        };

        self.proxies_t.insert(proxy_id, proxy_t);

        let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
        v.push(listener);
    }

    fn add_proxy_listener(&mut self, proxy_id: u32, listener: ProxyListener) {
        let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
        v.push(Box::new(listener));
    }

    fn remove(&mut self, proxy_id: u32) {
        self.proxies_t.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}

fn meta_map() -> &'static Mutex<HashMap<String, String>> {
    static INSTANCE: OnceCell<Mutex<HashMap<String, String>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn test_pw() {
    init();

    roundtrip();

    unsafe { deinit() };
}

fn roundtrip() {
    let mainloop = MainLoop::new().expect("Failed to create main loop");
    let context = Context::new(&mainloop).expect("Failed to create context");
    let core = context.connect(None).expect("Failed to connect to core");

    let _listener_core = core.add_listener_local().register();

    let registry = Arc::new(core.get_registry().expect("Failed to get Registry"));
    let registry_weak = Arc::downgrade(&registry);

    // Proxies and their listeners need to stay alive so store them here
    let proxies = Rc::new(RefCell::new(Proxies::new()));

    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            if let Some(registry) = registry_weak.upgrade() {
                let p = if let ObjectType::Metadata = obj.type_ {
                    let mut p = None;
                    if let Some(props) = &obj.props {
                        if let Some("settings") = props.get("metadata.name") {
                            let metadata: Metadata = registry.bind(obj).unwrap();
                            let obj_listener = metadata
                                .add_listener_local()
                                .property(|subject, key, _type, value| {
                                    if subject == 0 {
                                        if let Some(key) = key {
                                            if let Some(value) = value {
                                                meta_map()
                                                    .lock()
                                                    .unwrap()
                                                    .insert(key.to_owned(), value.to_owned());
                                            } else {
                                                meta_map().lock().unwrap().remove(key);
                                            }
                                        }
                                    }
                                    0
                                })
                                .register();
                            p = Some((Box::new(metadata), Box::new(obj_listener)));
                        }
                    }
                    p
                } else {
                    None
                };

                if let Some((proxy_spe, listener_spe)) = p {
                    let proxy = proxy_spe.upcast_ref();
                    let proxy_id = proxy.id();
                    // Use a weak ref to prevent references cycle between Proxy and proxies:
                    // - ref on proxies in the closure, bound to the Proxy lifetime
                    // - proxies owning a ref on Proxy as well
                    let proxies_weak = Rc::downgrade(&proxies);

                    let listener = proxy
                        .add_listener_local()
                        .removed(move || {
                            if let Some(proxies) = proxies_weak.upgrade() {
                                proxies.borrow_mut().remove(proxy_id);
                            }
                        })
                        .register();

                    proxies.borrow_mut().add_proxy_t(proxy_spe, listener_spe);
                    proxies.borrow_mut().add_proxy_listener(proxy_id, listener);
                }
            };
        })
        .register();

    mainloop.run();
}
