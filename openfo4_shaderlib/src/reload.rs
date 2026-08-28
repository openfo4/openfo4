use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};

use game_render::shader::{Error, Shader};
use hashbrown::HashMap;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use notify::{RecursiveMode, Watcher};

pub type BoxError = Box<dyn std::error::Error>;

#[derive(Debug)]
pub struct ReloadableShaderSource {
    path: PathBuf,
    cell: Arc<AtomicBool>,
    sources: Mutex<Vec<PathBuf>>,
}

impl ReloadableShaderSource {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cell: Arc::new(AtomicBool::new(false)),
            sources: Mutex::new(Vec::new()),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.cell.swap(false, Ordering::SeqCst)
    }

    pub fn reload_sources(&self) -> Result<(), Error> {
        let mut sources = self.sources.lock().unwrap();
        for path in sources.drain(..) {
            FileWatcher::unregister(path, self.cell.clone());
        }

        let new_sources = slangc::load_imported_files(&self.path).map_err(Error::Slang)?;
        for source in &new_sources {
            FileWatcher::register(source.clone(), self.cell.clone());
        }
        *sources = new_sources;

        Ok(())
    }

    pub fn compile(&self) -> Result<Shader, BoxError> {
        let bytes = compile(&self.path)?;
        Ok(Shader::from_spirv(&bytes)?)
    }
}

enum WatchEvent {
    Register(PathBuf, Arc<AtomicBool>),
    Unregister(PathBuf, Arc<AtomicBool>),
    Changed(PathBuf),
}

struct FileWatcher;

impl FileWatcher {
    fn register(path: PathBuf, cell: Arc<AtomicBool>) {
        Self::get().send(WatchEvent::Register(path, cell)).ok();
    }

    fn unregister(path: PathBuf, cell: Arc<AtomicBool>) {
        Self::get().send(WatchEvent::Unregister(path, cell)).ok();
    }

    fn get() -> &'static mpsc::Sender<WatchEvent> {
        static WATCHER: OnceLock<mpsc::Sender<WatchEvent>> = OnceLock::new();

        WATCHER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();

            let event_handler = {
                let tx = tx.clone();
                move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res
                        && (event.need_rescan()
                            || event.kind.is_create()
                            || event.kind.is_modify()
                            || event.kind.is_remove())
                    {
                        for path in event.paths {
                            tx.send(WatchEvent::Changed(path)).ok();
                        }
                    }
                }
            };

            let mut watcher = notify::recommended_watcher(event_handler).unwrap();

            let mut paths = HashMap::<_, Vec<_>>::new();

            std::thread::spawn(move || {
                while let Ok(event) = rx.recv() {
                    match event {
                        WatchEvent::Register(path, cell) => {
                            let add_to_watcher = !paths.contains_key(&path);
                            paths.entry(path.clone()).or_default().push(cell);

                            if add_to_watcher {
                                watcher.watch(&path, RecursiveMode::NonRecursive).ok();
                            }
                        }
                        WatchEvent::Unregister(path, cell) => {
                            let mut remove_from_watcher = false;
                            if let Some(cells) = paths.get_mut(&path) {
                                cells.retain(|c| !Arc::ptr_eq(c, &cell));
                                if cells.is_empty() {
                                    paths.remove(&path);
                                    remove_from_watcher = true;
                                }
                            }

                            if remove_from_watcher {
                                watcher.unwatch(&path).ok();
                            }
                        }
                        WatchEvent::Changed(path) => {
                            if let Some(cells) = paths.get(&path) {
                                for cell in cells {
                                    cell.store(true, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }
            });

            tx
        })
    }
}

fn compile(path: &Path) -> Result<Vec<u8>, BoxError> {
    if path.extension().is_some_and(|ext| ext == "slang") {
        compile_slang(path)
    } else if path.extension().is_some_and(|ext| ext == "wgsl") {
        compile_wgsl(path)
    } else {
        panic!("unknown shader extension: {:?}", path.extension())
    }
}

fn compile_slang(path: &Path) -> Result<Vec<u8>, BoxError> {
    let bytes = slangc::compile(&path, &slangc::Config::default())?;
    Ok(bytes)
}

fn compile_wgsl(path: &Path) -> Result<Vec<u8>, BoxError> {
    let data = std::fs::read_to_string(path)?;
    let module = naga::front::wgsl::parse_str(&data)?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::default()).validate(&module)?;
    let words = naga::back::spv::write_vec(&module, &info, &Default::default(), None)?;
    Ok(words
        .into_iter()
        .map(|word| word.to_le_bytes())
        .flatten()
        .collect())
}
