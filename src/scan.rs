use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::tree::{NONE, Node, Tree};

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub cancel: AtomicBool,
    pub current: Mutex<String>,
}

pub struct Scan {
    pub progress: Arc<Progress>,
    pub root: PathBuf,
    /// Kök bir bağlama noktasıysa dosya sisteminin dolu alanı (ilerleme çubuğu için).
    pub expected_bytes: Option<u64>,
    handle: Option<JoinHandle<Option<Tree>>>,
}

impl Scan {
    pub fn start(root: PathBuf, one_fs: bool, ctx: eframe::egui::Context) -> Scan {
        let progress = Arc::new(Progress::default());
        let expected_bytes = if is_mount_point(&root) {
            statvfs(&root).map(|(total, free)| total.saturating_sub(free))
        } else {
            None
        };
        let p = progress.clone();
        let r = root.clone();
        let handle = std::thread::Builder::new()
            .name("tarayici".into())
            // Derin klasör ağaçları özyinelemede yığını taşırmasın.
            .stack_size(512 * 1024 * 1024)
            .spawn(move || {
                let tree = scan(r, one_fs, &p);
                ctx.request_repaint();
                tree
            })
            .expect("tarama iş parçacığı başlatılamadı");
        Scan { progress, root, expected_bytes, handle: Some(handle) }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }

    pub fn take_result(&mut self) -> Option<Tree> {
        self.handle.take().and_then(|h| h.join().ok()).flatten()
    }
}

pub fn statvfs(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let frsize = st.f_frsize as u64;
    Some((st.f_blocks as u64 * frsize, st.f_bavail as u64 * frsize))
}

pub fn is_mount_point(path: &Path) -> bool {
    let Ok(md) = fs::metadata(path) else { return false };
    match path.parent() {
        None => true,
        Some(parent) => fs::metadata(parent).map(|p| p.dev() != md.dev()).unwrap_or(false),
    }
}

struct Scanner<'a> {
    nodes: Vec<Node>,
    progress: &'a Progress,
    root_dev: u64,
    one_fs: bool,
    seen_links: HashSet<(u64, u64)>,
    path: PathBuf,
    last_report: Instant,
}

fn scan(root: PathBuf, one_fs: bool, progress: &Progress) -> Option<Tree> {
    let md = fs::metadata(&root).ok()?;
    let (fs_total, fs_free) = statvfs(&root).unwrap_or((0, 0));
    let mut s = Scanner {
        nodes: Vec::with_capacity(1 << 16),
        progress,
        root_dev: md.dev(),
        one_fs,
        seen_links: HashSet::new(),
        path: root.clone(),
        last_report: Instant::now(),
    };
    s.nodes.push(Node {
        name: root.to_string_lossy().into(),
        raw_name: None,
        size: md.blocks() * 512,
        mtime: md.mtime(),
        parent: NONE,
        children: Vec::new(),
        depth: 0,
        is_dir: md.is_dir(),
        files: 0,
        dirs: 0,
    });
    if md.is_dir() {
        s.scan_dir(0, 0);
    }
    if progress.cancel.load(Ordering::Relaxed) {
        return None;
    }
    let is_mount = is_mount_point(&root);
    let mut tree = Tree { nodes: s.nodes, root_path: root, fs_total, fs_free, is_mount };
    tree.finalize();
    Some(tree)
}

impl Scanner<'_> {
    fn scan_dir(&mut self, id: u32, depth: u16) {
        let Ok(rd) = fs::read_dir(&self.path) else { return };
        for ent in rd.flatten() {
            if self.progress.cancel.load(Ordering::Relaxed) {
                return;
            }
            let name: OsString = ent.file_name();
            // DirEntry::metadata sembolik bağları izlemez (lstat).
            let Ok(md) = ent.metadata() else { continue };
            let ft = md.file_type();
            let mut size = md.blocks() * 512;
            if !ft.is_dir() && md.nlink() > 1 && !self.seen_links.insert((md.dev(), md.ino())) {
                size = 0; // sert bağ: yalnız ilk görüldüğünde say
            }
            let is_dir = ft.is_dir();
            let (display, raw) = match name.to_str() {
                Some(s) => (Box::<str>::from(s), None),
                None => (name.to_string_lossy().into(), Some(name.clone().into_boxed_os_str())),
            };
            let child = self.nodes.len() as u32;
            self.nodes.push(Node {
                name: display,
                raw_name: raw,
                size,
                mtime: md.mtime(),
                parent: id,
                children: Vec::new(),
                depth: depth + 1,
                is_dir,
                files: u64::from(!is_dir),
                dirs: u64::from(is_dir),
            });
            self.nodes[id as usize].children.push(child);
            self.progress.bytes.fetch_add(size, Ordering::Relaxed);

            if is_dir {
                self.progress.dirs.fetch_add(1, Ordering::Relaxed);
                if !(self.one_fs && md.dev() != self.root_dev) {
                    self.path.push(&name);
                    self.report();
                    self.scan_dir(child, depth + 1);
                    self.path.pop();
                }
            } else {
                self.progress.files.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn report(&mut self) {
        if self.last_report.elapsed() >= Duration::from_millis(50) {
            self.last_report = Instant::now();
            if let Ok(mut c) = self.progress.current.lock() {
                *c = self.path.to_string_lossy().into_owned();
            }
        }
    }
}

pub struct MountEntry {
    pub path: PathBuf,
    pub fstype: String,
    /// Ağ bağlarında (NAS kapalıyken donmasın diye) sorgulanmaz.
    pub space: Option<(u64, u64)>,
}

/// /proc/self/mounts içinden gerçek dosya sistemleri.
pub fn mounts() -> Vec<MountEntry> {
    const LOCAL: &[&str] = &[
        "ext2", "ext3", "ext4", "btrfs", "xfs", "f2fs", "vfat", "exfat", "ntfs", "ntfs3",
        "fuseblk", "zfs", "jfs", "reiserfs",
    ];
    const NETWORK: &[&str] = &["cifs", "smb3", "nfs", "nfs4", "sshfs", "fuse.sshfs"];
    let Ok(text) = fs::read_to_string("/proc/self/mounts") else { return Vec::new() };
    let mut out: Vec<MountEntry> = Vec::new();
    for line in text.lines() {
        let mut f = line.split(' ');
        let (Some(_dev), Some(mnt), Some(fstype)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let local = LOCAL.contains(&fstype);
        let network = NETWORK.contains(&fstype);
        if !local && !network {
            continue;
        }
        let path = PathBuf::from(unescape_mount(mnt));
        if path.starts_with("/snap") || out.iter().any(|m| m.path == path) {
            continue;
        }
        let space = if local { statvfs(&path) } else { None };
        out.push(MountEntry { path, fstype: fstype.to_string(), space });
    }
    out
}

/// /proc/mounts boşluk vb. karakterleri \040 gibi sekizlik kaçışla yazar.
fn unescape_mount(s: &str) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
            out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    OsString::from_vec(out)
}
