use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};
pub type Palettes = [crate::theme::Palette; 6];
struct Request {
    hwnd: usize,
    revision: u64,
    light: bool,
}
pub struct Worker {
    tx: Option<mpsc::Sender<Request>>,
    latest: Arc<AtomicU64>,
    job: Option<thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start(publish: impl Fn(usize, u64, Result<Palettes, String>) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let latest = Arc::new(AtomicU64::new(0));
        let active = latest.clone();
        let job = thread::spawn(move || {
            while let Ok(mut r) = rx.recv() {
                while let Ok(next) = rx.try_recv() {
                    r = next;
                }
                if active.load(Ordering::Acquire) != r.revision {
                    continue;
                }
                let result = Ok(crate::color_schemes::all_palettes(r.light));
                if active.load(Ordering::Acquire) == r.revision {
                    publish(r.hwnd, r.revision, result);
                }
            }
        });
        Self {
            tx: Some(tx),
            latest,
            job: Some(job),
        }
    }
    pub fn request(&self, hwnd: usize, v: &mut crate::style_compare_ui::Comparison) {
        v.revision = self.latest.fetch_add(1, Ordering::AcqRel) + 1;
        v.fill_presets();
        let _ = self.tx.as_ref().unwrap().send(Request {
            hwnd,
            revision: v.revision,
            light: v.light,
        });
    }
    pub fn cancel(&self) {
        self.latest.fetch_add(1, Ordering::AcqRel);
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        self.tx.take();
        if let Some(job) = self.job.take() {
            let _ = job.join();
        }
    }
}
