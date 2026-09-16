use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Tr,
    En,
}

pub struct T {
    pub app: &'static str,
    pub open: &'static str,
    pub reload: &'static str,
    pub zoom_full: &'static str,
    pub zoom_in: &'static str,
    pub zoom_out: &'static str,
    pub free_space: &'static str,
    pub run_open: &'static str,
    pub delete: &'static str,
    pub setup: &'static str,
    pub about: &'static str,
    pub total: &'static str,
    pub free: &'static str,
    pub files_total: &'static str,
    pub folders_total: &'static str,
    pub bytes: &'static str,
    pub files: &'static str,
    pub folders: &'static str,
    pub select_title: &'static str,
    pub path: &'static str,
    pub home: &'static str,
    pub network: &'static str,
    pub one_fs: &'static str,
    pub scan: &'static str,
    pub cancel: &'static str,
    pub scanning: &'static str,
    pub found_files: &'static str,
    pub found_folders: &'static str,
    pub show_in_fm: &'static str,
    pub copy_path: &'static str,
    pub confirm_trash: &'static str,
    pub trash_error: &'static str,
    pub ok: &'static str,
    pub settings: &'static str,
    pub language: &'static str,
    pub layout: &'static str,
    pub density: &'static str,
    pub density_many: &'static str,
    pub density_few: &'static str,
    pub bias: &'static str,
    pub horz: &'static str,
    pub vert: &'static str,
    pub tooltips: &'static str,
    pub info_tips: &'static str,
    pub full_path: &'static str,
    pub date_time: &'static str,
    pub file_size: &'static str,
    pub counts: &'static str,
    pub delay_ms: &'static str,
    pub rollover: &'static str,
    pub misc: &'static str,
    pub animated_zoom: &'static str,
    pub disable_delete: &'static str,
    pub about_text: &'static str,
    pub empty_hint: &'static str,
    pub not_found: &'static str,
    pub map: &'static str,
    pub list: &'static str,
    pub layout_style: &'static str,
    pub squarified: &'static str,
    pub classic: &'static str,
    pub protect_system: &'static str,
    pub protected_msg: &'static str,
    pub col_name: &'static str,
    pub col_share: &'static str,
    pub col_date: &'static str,
}

pub const TR: T = T {
    app: "Disk Haritası",
    open: "Aç",
    reload: "Yenile",
    zoom_full: "Tümü",
    zoom_in: "Yakınlaş",
    zoom_out: "Uzaklaş",
    free_space: "Boş Alan",
    run_open: "Çalıştır/Aç",
    delete: "Sil",
    setup: "Ayarlar",
    about: "Hakkında",
    total: "toplam",
    free: "boş",
    files_total: "Dosya",
    folders_total: "Klasör",
    bytes: "bayt",
    files: "dosya",
    folders: "klasör",
    select_title: "Taranacak yeri seç",
    path: "Yol",
    home: "Ev klasörü",
    network: "ağ",
    one_fs: "Başka dosya sistemlerine geçme",
    scan: "Tara",
    cancel: "İptal",
    scanning: "Taranıyor…",
    found_files: "Bulunan dosya",
    found_folders: "Bulunan klasör",
    show_in_fm: "Dosya yöneticisinde göster",
    copy_path: "Yolu kopyala",
    confirm_trash: "Çöp kutusuna taşınsın mı?",
    trash_error: "Çöp kutusuna taşınamadı",
    ok: "Tamam",
    settings: "Ayarlar",
    language: "Dil",
    layout: "Yerleşim",
    density: "Yoğunluk",
    density_many: "Çok dosya",
    density_few: "Az dosya",
    bias: "Eğilim",
    horz: "Yatay",
    vert: "Dikey",
    tooltips: "İpuçları",
    info_tips: "Bilgi ipucu göster",
    full_path: "Tam yol",
    date_time: "Tarih / saat",
    file_size: "Boyut",
    counts: "Dosya/klasör sayısı",
    delay_ms: "Gecikme (ms)",
    rollover: "Üzerine gelince kutuyu çerçevele",
    misc: "Çeşitli",
    animated_zoom: "Animasyonlu yakınlaş / uzaklaş",
    disable_delete: "Silme komutunu kapat",
    about_text: "Disk kullanımını iç içe kutular (treemap) olarak gösterir.",
    empty_hint: "Taramak için «Aç»",
    not_found: "Bulunamadı",
    map: "Harita",
    list: "Liste",
    layout_style: "Biçim:",
    squarified: "Kareye yakın",
    classic: "Klasik (SpaceMonger 1.4)",
    protect_system: "Sistem klasörlerini silmeye karşı koru",
    protected_msg: "Korumalı konum: sistem klasörü, ev klasörü ya da bağlama noktası silinemez.",
    col_name: "Ad",
    col_share: "Pay",
    col_date: "Değiştirilme",
};

pub const EN: T = T {
    app: "Disk Map",
    open: "Open",
    reload: "Reload",
    zoom_full: "Zoom Full",
    zoom_in: "Zoom In",
    zoom_out: "Zoom Out",
    free_space: "Free Space",
    run_open: "Run or Open",
    delete: "Delete",
    setup: "Setup",
    about: "About",
    total: "total",
    free: "free",
    files_total: "Files",
    folders_total: "Folders",
    bytes: "bytes",
    files: "files",
    folders: "folders",
    select_title: "Select location to view",
    path: "Path",
    home: "Home folder",
    network: "network",
    one_fs: "Stay on one file system",
    scan: "Scan",
    cancel: "Cancel",
    scanning: "Scanning…",
    found_files: "Files found",
    found_folders: "Folders found",
    show_in_fm: "Show in file manager",
    copy_path: "Copy path",
    confirm_trash: "Move to trash?",
    trash_error: "Could not move to trash",
    ok: "OK",
    settings: "Settings",
    language: "Language",
    layout: "Layout",
    density: "Density",
    density_many: "Lots of files",
    density_few: "Few files",
    bias: "Bias",
    horz: "Horizontal",
    vert: "Vertical",
    tooltips: "Tooltips",
    info_tips: "Show info tips",
    full_path: "Full path",
    date_time: "Date / time",
    file_size: "Size",
    counts: "File/folder counts",
    delay_ms: "Delay (ms)",
    rollover: "Outline box under pointer",
    misc: "Miscellaneous",
    animated_zoom: "Animated zoom in / out",
    disable_delete: "Disable delete command",
    about_text: "Shows disk usage as nested boxes (treemap).",
    empty_hint: "Click “Open” to scan",
    not_found: "Not found",
    map: "Map",
    list: "List",
    layout_style: "Style:",
    squarified: "Squarified",
    classic: "Classic (SpaceMonger 1.4)",
    protect_system: "Protect system folders from deletion",
    protected_msg: "Protected location: system folders, the home folder and mount points cannot be deleted.",
    col_name: "Name",
    col_share: "Share",
    col_date: "Modified",
};

pub fn t(lang: Lang) -> &'static T {
    match lang {
        Lang::Tr => &TR,
        Lang::En => &EN,
    }
}

/// 4269932544 → "4.269.932.544" (tr) / "4,269,932,544" (en)
pub fn thousands(n: u64, lang: Lang) -> String {
    let sep = if lang == Lang::Tr { '.' } else { ',' };
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

/// İnsan okuyacağı boyut, ikilik birimlerle.
pub fn human(bytes: u64, lang: Lang) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    let s = if u == 0 { format!("{bytes} B") } else { format!("{v:.1} {}", UNITS[u]) };
    if lang == Lang::Tr { s.replace('.', ",") } else { s }
}

pub fn date(mtime: i64, lang: Lang) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(mtime, 0).single() {
        Some(dt) if lang == Lang::Tr => dt.format("%d.%m.%Y  %H:%M:%S").to_string(),
        Some(dt) => dt.format("%d %b %Y  %H:%M:%S").to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binlik_ayirici() {
        assert_eq!(thousands(4269932544, Lang::En), "4,269,932,544");
        assert_eq!(thousands(999, Lang::Tr), "999");
        assert_eq!(thousands(1000, Lang::Tr), "1.000");
    }
}
