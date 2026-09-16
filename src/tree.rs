use std::ffi::OsStr;
use std::path::PathBuf;

pub const NONE: u32 = u32::MAX;

pub struct Node {
    /// Görüntülenecek ad (UTF-8 değilse kayıplı).
    pub name: Box<str>,
    /// Ad UTF-8 değilse ham bayt hali; yol kurarken bu kullanılır.
    pub raw_name: Option<Box<OsStr>>,
    /// Diskte kapladığı alan (alt ağaç dahil), bayt.
    pub size: u64,
    pub mtime: i64,
    pub parent: u32,
    pub children: Vec<u32>,
    /// Tarama kökünden mutlak derinlik (kök = 0).
    pub depth: u16,
    pub is_dir: bool,
    /// Alt ağaçtaki dosya / klasör sayısı (kendisi dahil).
    pub files: u64,
    pub dirs: u64,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub root_path: PathBuf,
    /// Kökün bulunduğu dosya sisteminin toplam / boş alanı.
    pub fs_total: u64,
    pub fs_free: u64,
    /// Kök bir bağlama noktası mı (boş alan yalnız o zaman anlamlı).
    pub is_mount: bool,
}

impl Tree {
    pub fn path(&self, id: u32) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = id;
        while cur != 0 && cur != NONE {
            parts.push(cur);
            cur = self.nodes[cur as usize].parent;
        }
        let mut p = self.root_path.clone();
        for &n in parts.iter().rev() {
            let node = &self.nodes[n as usize];
            match &node.raw_name {
                Some(raw) => p.push(&**raw),
                None => p.push(&*node.name),
            }
        }
        p
    }

    /// Görüntü için yol; klasörler `/` ile biter.
    pub fn display_path(&self, id: u32) -> String {
        let mut s = self.path(id).to_string_lossy().into_owned();
        if self.nodes[id as usize].is_dir && !s.ends_with('/') {
            s.push('/');
        }
        s
    }

    pub fn is_ancestor_or_self(&self, anc: u32, mut id: u32) -> bool {
        while id != NONE {
            if id == anc {
                return true;
            }
            id = self.nodes[id as usize].parent;
        }
        false
    }

    /// Tarama bittikten sonra: boyut/sayıları yukarı topla, çocukları büyükten küçüğe sırala.
    pub fn finalize(&mut self) {
        // Çocuklar her zaman ebeveynden sonra eklendiği için tersten tek geçiş yeter.
        for i in (1..self.nodes.len()).rev() {
            let (size, files, dirs, parent) = {
                let n = &self.nodes[i];
                (n.size, n.files, n.dirs, n.parent)
            };
            let p = &mut self.nodes[parent as usize];
            p.size += size;
            p.files += files;
            p.dirs += dirs;
        }
        for i in 0..self.nodes.len() {
            self.sort_children(i as u32);
        }
    }

    fn sort_children(&mut self, id: u32) {
        let mut ch = std::mem::take(&mut self.nodes[id as usize].children);
        ch.sort_by(|a, b| self.nodes[*b as usize].size.cmp(&self.nodes[*a as usize].size));
        self.nodes[id as usize].children = ch;
    }

    /// Düğümü ağaçtan ayırır (dosya çöpe taşındıktan sonra).
    pub fn detach(&mut self, id: u32) {
        let (size, files, dirs, parent) = {
            let n = &self.nodes[id as usize];
            (n.size, n.files, n.dirs, n.parent)
        };
        if parent == NONE {
            return;
        }
        self.nodes[parent as usize].children.retain(|&c| c != id);
        self.nodes[id as usize].parent = NONE;
        let mut cur = parent;
        while cur != NONE {
            let n = &mut self.nodes[cur as usize];
            n.size = n.size.saturating_sub(size);
            n.files = n.files.saturating_sub(files);
            n.dirs = n.dirs.saturating_sub(dirs);
            let next = n.parent;
            self.sort_children(cur);
            cur = next;
        }
    }
}
