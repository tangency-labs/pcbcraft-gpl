//! Localization (`en` / `he`), BCP 47 tag normalization, and visual RTL helpers for egui 0.36.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Supported UI languages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    English,
    Hebrew,
}

impl Serialize for Lang {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Lang {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let normalized = match raw.trim().to_ascii_lowercase().as_str() {
            "english" => Some(Lang::English),
            "hebrew" => Some(Lang::Hebrew),
            _ => Lang::from_tag(&raw),
        };
        Ok(normalized.unwrap_or_default())
    }
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::English, Lang::Hebrew];

    pub const fn code(self) -> &'static str {
        match self {
            Lang::English => "en",
            Lang::Hebrew => "he",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Lang::English => "English",
            Lang::Hebrew => "עברית",
        }
    }

    pub fn is_rtl(self) -> bool {
        is_rtl_code(self.code())
    }

    /// Parses a BCP 47 / POSIX locale tag (normalizing legacy `iw` to Hebrew and rejecting
    /// private-use `x-*` subtags).
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let clean = strip_posix_suffix(tag)
            .replace('_', "-")
            .to_ascii_lowercase();
        let mut parts = clean.split('-').filter(|s| !s.is_empty());
        let primary = parts.next()?;
        if primary == "x" {
            return None;
        }
        if let Some(script) = script_subtag(&mut parts) {
            if script == "hebr" {
                return Some(Lang::Hebrew);
            }
            if script == "latn" && (primary == "he" || primary == "iw") {
                return None;
            }
        }
        match primary {
            "he" | "iw" => Some(Lang::Hebrew),
            "en" => Some(Lang::English),
            _ => None,
        }
    }
}

fn strip_posix_suffix(tag: &str) -> &str {
    tag.trim().split(['.', '@']).next().unwrap_or_default()
}

fn script_subtag<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut extlangs = 0usize;
    for sub in parts {
        if sub.len() == 1 {
            return None;
        }
        if sub.len() == 4 && sub.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Some(sub);
        }
        if extlangs < 3 && sub.len() == 3 && sub.bytes().all(|b| b.is_ascii_alphabetic()) {
            extlangs += 1;
            continue;
        }
        return None;
    }
    None
}

/// Whether `code` names a right-to-left script or language (accepting legacy `iw`, honoring RFC
/// 5646 script position, and rejecting private-use `x-*` subtags).
pub fn is_rtl_code(code: &str) -> bool {
    let clean = strip_posix_suffix(code)
        .replace('_', "-")
        .to_ascii_lowercase();
    let mut parts = clean.split('-').filter(|s| !s.is_empty());
    let Some(primary) = parts.next() else {
        return false;
    };
    if primary == "x" {
        return false;
    }
    if let Some(script) = script_subtag(&mut parts) {
        return matches!(
            script,
            "arab"
                | "hebr"
                | "syrc"
                | "thaa"
                | "nkoo"
                | "adlm"
                | "mend"
                | "rohg"
                | "samr"
                | "mand"
                | "yezi"
        );
    }
    matches!(
        primary,
        "ar" | "arc"
            | "ckb"
            | "dv"
            | "fa"
            | "he"
            | "iw"
            | "ji"
            | "ps"
            | "sd"
            | "syr"
            | "ug"
            | "ur"
            | "yi"
    )
}

fn is_rtl_script(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x05FF | 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF
    )
}

fn is_ltr_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '+'
                | '-'
                | '.'
                | '/'
                | ':'
                | '~'
                | '\\'
                | '⌘'
                | '⌃'
                | '⇧'
                | '⌥'
                | '&'
                | '='
                | '|'
        )
}

fn mirror_bracket(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        other => other,
    }
}

fn mask_atoms(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let Some(&(start, ch)) = chars.get(i) else {
            break;
        };
        if ch == '\u{2066}' || ch == '\u{2067}' || ch == '\u{2068}' {
            let mut j = i + 1;
            let mut depth = 1usize;
            while j < chars.len() && depth > 0 {
                if let Some(&(_, c)) = chars.get(j) {
                    if c == '\u{2066}' || c == '\u{2067}' || c == '\u{2068}' {
                        depth += 1;
                    } else if c == '\u{2069}' {
                        depth -= 1;
                    }
                }
                j += 1;
            }
            let inner_start = chars.get(i + 1).map_or(line.len(), |(b, _)| *b);
            let inner_end = chars
                .get(j - 1)
                .filter(|(_, c)| *c == '\u{2069}')
                .map_or_else(|| chars.get(j).map_or(line.len(), |(b, _)| *b), |(b, _)| *b);
            if let Some(span) = line.get(inner_start..inner_end) {
                out.push(span.to_string());
            }
            i = j;
            continue;
        }
        if ch.is_whitespace() {
            let mut j = i + 1;
            while j < chars.len() && chars.get(j).is_some_and(|(_, c)| c.is_whitespace()) {
                j += 1;
            }
            let end = chars.get(j).map_or(line.len(), |(b, _)| *b);
            if let Some(seg) = line.get(start..end) {
                out.push(seg.to_string());
            }
            i = j;
            continue;
        }
        if ch.is_ascii_alphanumeric()
            || (matches!(ch, '.' | '/' | '~' | '\\')
                && chars
                    .get(i + 1)
                    .is_some_and(|(_, next)| next.is_ascii_alphanumeric() || *next == '/'))
            || (matches!(ch, '-' | '+')
                && chars
                    .get(i + 1)
                    .is_some_and(|(_, next)| next.is_ascii_digit()))
        {
            let mut j = i + 1;
            while j < chars.len() {
                let Some(&(_, c)) = chars.get(j) else {
                    break;
                };
                if is_ltr_token_char(c)
                    || (c == ' '
                        && chars
                            .get(j + 1)
                            .is_some_and(|(_, next)| next.is_ascii_alphanumeric()))
                {
                    j += 1;
                } else {
                    break;
                }
            }
            while j > i + 1
                && chars
                    .get(j - 1)
                    .is_some_and(|(_, c)| matches!(c, '.' | ':' | '-' | '/'))
            {
                j -= 1;
            }
            let end = chars.get(j).map_or(line.len(), |(b, _)| *b);
            if let Some(seg) = line.get(start..end) {
                out.push(seg.to_string());
            }
            i = j;
            continue;
        }
        if is_rtl_script(ch) {
            let mut j = i + 1;
            while j < chars.len()
                && chars
                    .get(j)
                    .is_some_and(|(_, c)| is_rtl_script(*c) || matches!(c, '\'' | '"' | '״' | '׳'))
            {
                j += 1;
            }
            if chars.get(j).is_some_and(|(_, c)| *c == '-')
                && chars
                    .get(j + 1)
                    .is_some_and(|(_, next)| next.is_ascii_alphanumeric())
            {
                let mut k = j + 1;
                while k < chars.len() && chars.get(k).is_some_and(|(_, c)| is_ltr_token_char(*c)) {
                    k += 1;
                }
                while k > j + 1
                    && chars
                        .get(k - 1)
                        .is_some_and(|(_, c)| matches!(c, '.' | ':' | '-'))
                {
                    k -= 1;
                }
                let word_end = chars.get(j).map_or(line.len(), |(b, _)| *b);
                let suffix_end = chars.get(k).map_or(line.len(), |(b, _)| *b);
                if let (Some(w), Some(s)) =
                    (line.get(start..word_end), line.get(word_end..suffix_end))
                {
                    out.push(w.to_string());
                    out.push(s.to_string());
                }
                i = k;
                continue;
            }
            let end = chars.get(j).map_or(line.len(), |(b, _)| *b);
            if let Some(seg) = line.get(start..end) {
                out.push(seg.to_string());
            }
            i = j;
            continue;
        }
        out.push(mirror_bracket(ch).to_string());
        i += 1;
    }
    out
}

fn push_visual_rtl_line(line: &str, out: &mut String) {
    for tok in mask_atoms(line).into_iter().rev() {
        out.push_str(&tok);
    }
}

/// Converts a logical RTL string to visual left-to-right display order for egui 0.36. Lines with
/// no RTL script characters are left untouched.
pub fn visual_rtl(logical: &str) -> String {
    if !logical.chars().any(is_rtl_script) {
        return logical.to_string();
    }
    let mut out = String::with_capacity(logical.len());
    for (idx, raw_line) in logical.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let (line, cr) = raw_line
            .strip_suffix('\r')
            .map_or((raw_line, false), |l| (l, true));
        if line.chars().any(is_rtl_script) {
            push_visual_rtl_line(line, &mut out);
        } else {
            out.push_str(line);
        }
        if cr {
            out.push('\r');
        }
    }
    out
}

/// Breaks `logical` text at `max_width` first, then applies visual RTL reordering per row.
pub fn wrapped_visual_lines(
    ctx: &egui::Context,
    rtl: bool,
    logical: &str,
    font_id: egui::FontId,
    max_width: f32,
) -> String {
    if !rtl || !logical.chars().any(is_rtl_script) {
        return logical.to_owned();
    }
    let galley = ctx.fonts_mut(|f| {
        f.layout(
            logical.to_owned(),
            font_id,
            Default::default(),
            max_width.max(1.0),
        )
    });
    let mut out = String::with_capacity(logical.len());
    for (idx, placed) in galley.rows.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let row_text = placed.text();
        let trimmed = row_text.trim_end_matches(' ');
        if trimmed.chars().any(is_rtl_script) {
            push_visual_rtl_line(trimmed, &mut out);
        } else {
            out.push_str(trimmed);
        }
    }
    out
}

/// Returns the logical translation of `key` in `lang`.
pub fn t_logical(lang: Lang, key: &'static str) -> &'static str {
    if lang == Lang::English {
        return key;
    }
    hebrew(key).unwrap_or(key)
}

/// Returns the display string for `key` in `lang` (visually reordered when RTL for egui 0.36).
pub fn t(lang: Lang, key: &'static str) -> String {
    let logical = t_logical(lang, key);
    if lang.is_rtl() {
        visual_rtl(logical)
    } else {
        logical.to_string()
    }
}

/// Substitutes `{}` placeholders sequentially in the logical translation, then applies visual RTL
/// reordering once when `lang.is_rtl()`.
pub fn tf(lang: Lang, key: &'static str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut iter = args.iter();
    let mut parts = t_logical(lang, key).split("{}").peekable();
    while let Some(part) = parts.next() {
        out.push_str(part);
        if parts.peek().is_some() {
            out.push_str(iter.next().copied().unwrap_or_default());
        }
    }
    if lang.is_rtl() { visual_rtl(&out) } else { out }
}

/// Renders a wrapping label that breaks logical text before visually reordering each display row.
pub fn wrapped_label(ui: &mut egui::Ui, lang: Lang, key: &'static str) {
    let logical = t_logical(lang, key);
    if !lang.is_rtl() {
        ui.label(logical);
        return;
    }
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let shown = wrapped_visual_lines(ui.ctx(), true, logical, font_id, ui.available_width());
    ui.add(egui::Label::new(shown).extend());
}

fn hebrew(key: &str) -> Option<&'static str> {
    Some(match key {
        "Language" => "שפה",
        "Schematic" => "סכמה",
        "PCB Layout" => "פריסת PCB",
        "Libraries" => "ספריות",
        "Checks" => "בדיקות",
        "3D Board" => "לוח 3D",
        "A real Rust 3D viewer is planned." => "מציג תלת-ממד אמיתי ב-Rust מתוכנן בהמשך.",
        "Appearance" => "מראה",
        "Save  Ctrl+S" => "שמירה  Ctrl+S",
        "Redo" => "ביצוע חוזר",
        "Undo" => "ביטול",
        "Select  Esc" => "בחירה  Esc",
        "Add symbol  A" => "הוסף סמל  A",
        "Wire  W" => "חיווט  W",
        "Sync to PCB >" => "סנכרון ל-PCB >",
        "Route  X" => "ניתוב  X",
        "Sync schematic" => "סנכרן סכמה",
        "Circular cutout" => "חיתוך עגול",
        "Move  M" => "הזזה  M",
        "Rotate  R" => "סיבוב  R",
        "Fit" => "התאמה",
        "Commands  Ctrl+K" => "פקודות  Ctrl+K",
        "PROJECT" => "פרויקט",
        "Add symbol" => "הוסף סמל",
        "Browse KiCad libraries →" => "עיון בספריות KiCad →",
        "Resistor" => "נגד",
        "LED" => "LED",
        "Capacitor" => "קבל",
        "2-pin connector" => "מחבר 2 פינים",
        "COMPONENTS" => "רכיבים",
        "Press A to place your first symbol" => "לחץ A להצבת הסמל הראשון שלך",
        "NETS" => "רשתות",
        "BOARD" => "לוח",
        "{} × {} mm" => "{} × {} מ\"מ",
        "2 copper layers · 1.6 mm" => "2 שכבות נחושת · 1.6 מ\"מ",
        "{} footprints · {} segments" => "{} עקבות · {} מקטעים",
        "One project. One window." => "פרויקט אחד. חלון אחד.",
        "Scroll to zoom · middle-drag to pan" => "גלול לזום · גרירה אמצעית להזזה",
        "Dark" => "כהה",
        "Light" => "בהיר",
        "Show grid" => "הצג רשת",
        "Accent color" => "צבע הדגשה",
        "Appearance is saved automatically." => "המראה נשמר אוטומטית.",
        "PROPERTIES" => "מאפיינים",
        "Unit {}" => "יחידה {}",
        "Stable ID {}" => "מזהה יציב {}",
        "Value" => "ערך",
        "Apply value" => "החל ערך",
        "Position · mm" => "מיקום · מ\"מ",
        "X" => "X",
        "Y" => "Y",
        "Apply position" => "החל מיקום",
        "Rotate 90°  R" => "סובב 90°  R",
        "Footprint: {}" => "עקבה: {}",
        "Footprint: unassigned" => "עקבה: לא משויכת",
        "Footprint: starter package" => "עקבה: חבילת התחלה",
        "Assign footprint in Libraries →" => "שייך עקבה בספריות →",
        "Pin / net" => "פין / רשת",
        "unassigned" => "לא משויך",
        "Selected object" => "אובייקט נבחר",
        "Delete  Del" => "מחיקה  Del",
        "Project settings" => "הגדרות פרויקט",
        "Select an item to inspect and edit it." => "בחר פריט כדי לבדוק ולערוך אותו.",
        "BOARD GEOMETRY" => "גיאומטריית לוח",
        "W" => "רוחב",
        "H" => "גובה",
        " mm" => " מ\"מ",
        "Apply board dimensions" => "החל ממדי לוח",
        "Cutout radius" => "רדיוס חיתוך",
        "CHECKS" => "בדיקות",
        "{} electrical · {} board >" => "{} חשמל · {} לוח >",
        "Initial checks; full ERC/DRC is in development." => {
            "בדיקות ראשוניות; בדיקות ERC/DRC מלאות בפיתוח."
        }
        "Ready · A add · W wire · X route · M move · R rotate · E properties" => {
            "מוכן · A הוסף · W חוט · X נתיב · M הזז · R סובב · E מאפיינים"
        }
        "{} · committed" => "{} · אושר",
        "{} · complete" => "{} · הושלם",
        "operation failed" => "הפעולה נכשלה",
        "Session unavailable" => "הסשן אינו זמין",
        "{} library symbols need a footprint assignment in Libraries" => {
            "{} סמלי ספרייה דורשים שיוך עקבה בספריות"
        }
        "rev {}" => "מהדורה {}",
        "{}, {} mm" => "{}, {} מ\"מ",
        "Project file" => "קובץ פרויקט",
        "Open" => "פתיחה",
        "Save" => "שמירה",
        "Export KiCad board" => "ייצוא לוח KiCad",
        "LED example" => "דוגמת LED",
        "New project" => "פרויקט חדש",
        "ENGINE COMMAND" => "פקודת מנוע",
        "Execute" => "ביצוע",
        "Original starter symbols" => "סמלי פתיחה מקוריים",
        "{} · {} · 2 pins" => "{} · {} · 2 פינים",
        "Place in schematic >" => "הצב בסכמה >",
        "Design checks" => "בדיקות תכנון",
        "Select a finding to inspect its affected object in the editor." => {
            "בחר ממצא כדי לבדוק את האובייקט המושפע בעורך."
        }
        "Electrical" => "חשמל",
        "Board" => "לוח",
        "No findings in the implemented checks" => "לא נמצאו ממצאים בבדיקות המיושמות",
        "finding" => "ממצא",
        "SELECT" => "בחירה",
        "PLACE SYMBOL · CLICK TO COMMIT" => "הצבת סמל · לחץ לאישור",
        "PLACE SYMBOL PACKAGE · CLICK TO COMMIT" => "הצבת חבילת סמל · לחץ לאישור",
        "WIRE · PICK FIRST PIN" => "חיווט · בחר פין ראשון",
        "WIRE · PICK SECOND PIN" => "חיווט · בחר פין שני",
        "ROUTE · PICK FIRST PAD" => "ניתוב · בחר פד ראשון",
        "ROUTE · PICK SECOND PAD" => "ניתוב · בחר פד שני",
        "MOVE · CLICK TO PLACE SELECTION" => "הזזה · לחץ להצבת הבחירה",
        "CUTOUT · CLICK TO PLACE" => "חיתוך · לחץ להצבה",
        "{} × {} mm    F.Cu / B.Cu    clearance {} mm" => {
            "{} × {} מ\"מ    F.Cu / B.Cu    מרווח {} מ\"מ"
        }
        "Start" => "התחלה",
        "End" => "סיום",
        "Stroke" => "עובי קו",
        "Layer" => "שכבה",
        "Fill" => "מילוי",
        "Center" => "מרכז",
        "Radius" => "רדיוס",
        "Through" => "דרך",
        "Vertex {}" => "קודקוד {}",
        "Remaining vertices are preserved." => "הקודקודים הנותרים נשמרים.",
        "Add vertex" => "הוסף קודקוד",
        "Remove last vertex" => "הסר קודקוד אחרון",
        "Position" => "מיקום",
        "Text" => "טקסט",
        "Text size" => "גודל טקסט",
        "Angle" => "זווית",
        "Add drawing:" => "הוסף ציור:",
        "Line" => "קו",
        "Rectangle" => "מלבן",
        "Circle" => "עיגול",
        "Arc" => "קשת",
        "Polyline" => "קו שבור",
        "Installed libraries" => "ספריות מותקנות",
        "Project library" => "ספריית פרויקט",
        "Editor" => "עורך",
        "New symbol" => "סמל חדש",
        "New footprint" => "עקבה חדשה",
        "Editable project library" => "ספריית פרויקט ניתנת לעריכה",
        "Saved definitions live in this project. Placed components retain their embedded copies." => {
            "הגדרות שמורות נשמרות בפרויקט זה. רכיבים שהוצבו שומרים על העותקים המוטמעים שלהם."
        }
        "Collection file" => "קובץ אוסף",
        "Save library" => "שמור ספרייה",
        "Load library" => "טען ספרייה",
        "Symbols" => "סמלים",
        "{} physical pins" => "{} פינים פיזיים",
        "Edit" => "עריכה",
        "Place" => "הצבה",
        "Remove" => "הסרה",
        "Footprints" => "עקבות",
        "{} pads" => "{} פדים",
        "Assign to selection" => "שייך לבחירה",
        "Library editor" => "עורך ספריות",
        "Create a symbol or footprint, or edit a copy from the installed/project library." => {
            "צור סמל או עקבה, או ערוך עותק מהספרייה המותקנת או מספריית הפרויקט."
        }
        "Dimensions are millimetres; angles are counterclockwise degrees. Save commits one undoable edit." => {
            "הממדים הם במילימטרים; הזוויות במעלות נגד כיוון השעון. שמירה יוצרת עריכה אחת הניתנת לביטול."
        }
        "Symbol editor" => "עורך סמלים",
        "Name" => "שם",
        "Reference prefix" => "קידומת ייחוס",
        "Default value" => "ערך ברירת מחדל",
        "Description" => "תיאור",
        "Suggested footprint" => "עקבה מומלצת",
        "Preserved unsupported features: {}" => "תכונות לא נתמכות שנשמרו: {}",
        "Save symbol to project library" => "שמור סמל בספריית הפרויקט",
        "Pins" => "פינים",
        "Duplicate preview unit" => "שכפל יחידת תצוגה מקדימה",
        "Add pin" => "הוסף פין",
        "Pin {} · unit {}" => "פין {} · יחידה {}",
        "Number" => "מספר",
        "Connection" => "חיבור",
        "Length" => "אורך",
        "Unit " => "יחידה ",
        "Unit (0 shared) " => "יחידה (0 משותף) ",
        "Corner ratio " => "יחס פינה ",
        "Delete pin" => "מחק פין",
        "Drawing" => "ציור",
        "Graphic {} · unit {}" => "גרפיקה {} · יחידה {}",
        "Delete graphic" => "מחק גרפיקה",
        "Footprint editor" => "עורך עקבות",
        "Save footprint to project library" => "שמור עקבה בספריית הפרויקט",
        "Pads" => "פדים",
        "Add pad" => "הוסף פד",
        "Pad {} · {}" => "פד {} · {}",
        "Copper size" => "גודל נחושת",
        "Front SMD" => "SMD קדמי",
        "Plated through-hole" => "קדח מצופה",
        "Drill size (oval if unequal)" => "גודל קדח (אובלי אם לא שווה)",
        "Drill offset" => "היסט קדח",
        "Layers: {}" => "שכבות: {}",
        "Delete pad" => "מחק פד",
        "Graphic {}" => "גרפיקה {}",
        "KiCad libraries" => "ספריות KiCad",
        "Browse installed libraries · definitions used by a project are embedded on placement." => {
            "עיין בספריות מותקנות · הגדרות שבשימוש הפרויקט מוטמעות בעת ההצבה."
        }
        "Refresh folders" => "רענן תיקיות",
        "Choose library" => "בחר ספרייה",
        "Library path" => "נתיב ספרייה",
        "Search names, descriptions, keywords" => "חפש שמות, תיאורים, מילות מפתח",
        "Search" => "חיפוש",
        "Reading library…" => "קורא ספרייה…",
        "Installed library browsing is available in the desktop build." => {
            "עיון בספריות מותקנות זמין בגרסת שולחן העבודה."
        }
        "Some library folders exceeded the scan limit; use a specific library path." => {
            "תיקיות ספרייה מסוימות חרגו ממגבלת הסריקה; השתמש בנתיב ספרייה ספציפי."
        }
        "{} matches in scanned entries · showing up to 100" => {
            "{} התאמות בערכים שנסרקו · מציג עד 100"
        }
        "Directory scan reached its limit; more entries may exist." => {
            "סריקת התיקיות הגיעה למגבלה; ייתכן שקיימים ערכים נוספים."
        }
        "Original nodes exceed inline editing limits; the source file is preserved." => {
            "הצמתים המקוריים חורגים ממגבלות העריכה המוטבעת; קובץ המקור נשמר."
        }
        "Edit a copy →" => "ערוך עותק →",
        "{} pads · {} graphics" => "{} פדים · {} רכיבים גרפיים",
        "Pad numbers must match every pin of the selected symbol." => {
            "מספרי הפדים חייבים להתאים לכל פין בסמל הנבחר."
        }
        "Assign to {}" => "שייך אל {}",
        "Select a schematic component before assigning its footprint." => {
            "בחר רכיב סכמה לפני שיוך העקבה שלו."
        }
        "{} pins · {} units" => "{} פינים · {} יחידות",
        "Suggested footprint: {}" => "עקבה מומלצת: {}",
        "Place in schematic →" => "הצב בסכמה →",
        "Inspection available; unsupported pin/graphic styles or repeated common pin numbers cannot be placed yet." => {
            "הבדיקה זמינה; סגנונות פין/גרפיקה לא נתמכים או מספרי פין חוזרים עדיין לא ניתנים להצבה."
        }
        "Choose an entry to preview its real geometry." => {
            "בחר ערך כדי לראות תצוגה מקדימה של הגיאומטריה."
        }
        "KiCad libraries: CC-BY-SA 4.0 + design exception" => {
            "ספריות KiCad: CC-BY-SA 4.0 + חריג עיצוב"
        }
        "Placement: single/multi-unit symbols, matching front SMD or plated through-hole packages. Use Edit a copy to customize a definition." => {
            "הצבה: סמלים של יחידה אחת או מספר יחידות, התאמת חבילות SMD קדמיות או קדח מצופה. השתמש בערוך עותק להתאמה אישית."
        }
        "Preview unit" => "יחידת תצוגה מקדימה",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcp47_and_legacy_iw_normalization_and_rtl_detection() {
        for tag in ["he", "he-IL", "he_IL.UTF-8", "iw", "iw-IL", "en-Hebr"] {
            assert_eq!(Lang::from_tag(tag), Some(Lang::Hebrew), "{tag}");
            assert!(is_rtl_code(tag), "{tag}");
        }
        for tag in [
            "x-he",
            "en-x-he",
            "en-x-arab",
            "ar-Latn",
            "he-Latn",
            "en-US",
        ] {
            assert!(!is_rtl_code(tag), "{tag}");
        }
        assert!(is_rtl_code("zh-Arab-CN"));
        assert!(!is_rtl_code("en-u-ca-hebrew"));
        assert_eq!(Lang::from_tag("x-he"), None);
        assert_eq!(Lang::from_tag("en-x-he"), Some(Lang::English));
    }

    #[test]
    fn every_ui_key_translates_and_visual_rtl_keeps_ltr_tokens() {
        let keys = [
            "Language",
            "Schematic",
            "PCB Layout",
            "Libraries",
            "Checks",
            "3D Board",
            "A real Rust 3D viewer is planned.",
            "Appearance",
            "Save  Ctrl+S",
            "Redo",
            "Undo",
            "Select  Esc",
            "Add symbol  A",
            "Wire  W",
            "Sync to PCB >",
            "Route  X",
            "Sync schematic",
            "Circular cutout",
            "Move  M",
            "Rotate  R",
            "Fit",
            "Commands  Ctrl+K",
            "PROJECT",
            "Add symbol",
            "Browse KiCad libraries →",
            "Resistor",
            "Capacitor",
            "2-pin connector",
            "COMPONENTS",
            "Press A to place your first symbol",
            "NETS",
            "BOARD",
            "{} × {} mm",
            "2 copper layers · 1.6 mm",
            "{} footprints · {} segments",
            "One project. One window.",
            "Scroll to zoom · middle-drag to pan",
            "Dark",
            "Light",
            "Show grid",
            "Accent color",
            "Appearance is saved automatically.",
            "PROPERTIES",
            "Value",
            "Apply value",
            "Position · mm",
            "Apply position",
            "Rotate 90°  R",
            "Footprint: {}",
            "Footprint: unassigned",
            "Footprint: starter package",
            "Assign footprint in Libraries →",
            "Pin / net",
            "unassigned",
            "Selected object",
            "Delete  Del",
            "Project settings",
            "Select an item to inspect and edit it.",
            "BOARD GEOMETRY",
            "Apply board dimensions",
            "Cutout radius",
            "CHECKS",
            "{} electrical · {} board >",
            "Initial checks; full ERC/DRC is in development.",
            "Ready · A add · W wire · X route · M move · R rotate · E properties",
            "Project file",
            "Open",
            "Save",
            "Export KiCad board",
            "LED example",
            "New project",
            "ENGINE COMMAND",
            "Execute",
            "Original starter symbols",
            "Place in schematic >",
            "Design checks",
            "Select a finding to inspect its affected object in the editor.",
            "Electrical",
            "Board",
            "No findings in the implemented checks",
            "PLACE SYMBOL · CLICK TO COMMIT",
            "WIRE · PICK FIRST PIN",
            "ROUTE · PICK FIRST PAD",
            "Installed libraries",
            "Project library",
            "Editor",
            "New symbol",
            "New footprint",
            "Save symbol to project library",
            "Save footprint to project library",
            "KiCad libraries",
            "Symbols",
            "Footprints",
            "Search",
        ];
        for key in keys {
            let tr = t_logical(Lang::Hebrew, key);
            assert_ne!(tr, key, "key {key:?} missing Hebrew translation");
        }
        assert_eq!(t(Lang::Hebrew, "Schematic"), "סכמה");
        assert_eq!(t(Lang::Hebrew, "PCB Layout"), "PCB פריסת");
        assert_eq!(
            tf(Lang::Hebrew, "Footprint: {}", &["/tmp/R_0603.kicad_mod"]),
            "/tmp/R_0603.kicad_mod :עקבה"
        );
        assert_eq!(
            visual_rtl("מיקום: -12.50, +4.00 מ\"מ"),
            "מ\"מ +4.00 ,-12.50 :מיקום"
        );
        assert_eq!(visual_rtl("שינוי - בדיקה"), "בדיקה - שינוי");
        assert_eq!(
            tf(
                Lang::Hebrew,
                "Footprint: {}",
                &["\u{2066}foo - אב - bar\u{2069}"]
            ),
            "foo - אב - bar :עקבה"
        );
        assert_eq!(
            visual_rtl("בדיקה (CSV)\n  cargo test\nשורה שנייה"),
            "(CSV) בדיקה\n  cargo test\nשנייה שורה"
        );

        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let wrapped = wrapped_visual_lines(
            &ctx,
            true,
            t_logical(
                Lang::Hebrew,
                "Saved definitions live in this project. Placed components retain their embedded copies.",
            ),
            egui::FontId::proportional(13.0),
            160.0,
        );
        let first_row = wrapped.lines().next().unwrap_or_default();
        assert!(first_row.contains("הגדרות"), "got {wrapped:?}");
    }
}
