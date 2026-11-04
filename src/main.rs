mod eval;

use eval::{evaluate_document, format_number, TokenKind};
use gdk::keys::constants as keys;
use gdk::ModifierType;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
struct Page {
    title: String,
    content: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppState {
    active_page: usize,
    pages: Vec<Page>,
}

fn is_russian_locale() -> bool {
    if let Ok(lang) = std::env::var("SCHET_LANG") {
        return lang.to_lowercase().starts_with("ru");
    }
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return val.to_lowercase().starts_with("ru");
            }
        }
    }
    true
}

fn default_page_title(num: usize) -> String {
    if is_russian_locale() {
        format!("Лист {}", num)
    } else {
        format!("Sheet {}", num)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_page: 0,
            pages: vec![Page {
                title: default_page_title(1),
                content: String::new(),
            }],
        }
    }
}

fn state_file_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = base.join("schet");
    let _ = fs::create_dir_all(&dir);
    dir.join("state.json")
}

fn load_state() -> AppState {
    let ru = is_russian_locale();
    let path = state_file_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(mut st) = serde_json::from_str::<AppState>(&raw) {
            if st.pages.is_empty() {
                st.pages.push(Page {
                    title: default_page_title(1),
                    content: String::new(),
                });
            }
            for p in &mut st.pages {
                if ru {
                    if let Some(num) = p.title.strip_prefix("Page ").or_else(|| p.title.strip_prefix("Sheet ")) {
                        p.title = format!("Лист {}", num);
                    }
                } else if let Some(num) = p.title.strip_prefix("Лист ").or_else(|| p.title.strip_prefix("Page ")) {
                    p.title = format!("Sheet {}", num);
                }
            }
            if st.active_page >= st.pages.len() {
                st.active_page = 0;
            }
            return st;
        }
    }
    AppState::default()
}

fn save_state(state: &AppState) {
    let path = state_file_path();
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

fn detect_system_dark() -> bool {
    if let Ok(force) = std::env::var("SCHET_THEME") {
        if force.eq_ignore_ascii_case("dark") {
            return true;
        }
        if force.eq_ignore_ascii_case("light") {
            return false;
        }
    }

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    let mode_path = Path::new(&runtime_dir).join("theme-mode");
    if let Ok(mode) = fs::read_to_string(mode_path) {
        let trimmed = mode.trim();
        if trimmed.eq_ignore_ascii_case("dark") {
            return true;
        }
        if trimmed.eq_ignore_ascii_case("light") {
            return false;
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        let ini_path = PathBuf::from(home).join(".config/gtk-3.0/settings.ini");
        if let Ok(ini) = fs::read_to_string(ini_path) {
            for line in ini.lines() {
                let l = line.trim();
                if l == "gtk-application-prefer-dark-theme=true" || l == "gtk-application-prefer-dark-theme=1" {
                    return true;
                }
                if l == "gtk-application-prefer-dark-theme=false" || l == "gtk-application-prefer-dark-theme=0" {
                    return false;
                }
            }
        }
    }

    false
}

fn build_css(dark: bool) -> String {
    let (
        editor_bg,
        gutter_bg,
        output_bg,
        fg,
        muted,
        border,
        ans_fg,
        bar_bg,
        brand_bg,
        brand_fg,
        pill_bg,
        pill_fg,
        pill_border,
        pill_hover,
        caret_color,
    ) = if dark {
        // Amber Field — Dark
        (
            "#121210", // editor_bg
            "#171614", // gutter_bg
            "#151412", // output_bg
            "#fafaf9", // fg
            "#78716c", // muted
            "#262320", // border
            "#f59e0b", // ans_fg (phosphor amber)
            "#1c1917", // bar_bg
            "#292524", // brand_bg
            "#f59e0b", // brand_fg
            "#262320", // pill_bg
            "#d6d3d1", // pill_fg
            "#3f3b37", // pill_border
            "#35312d", // pill_hover
            "#f59e0b", // caret_color
        )
    } else {
        // Amber Field — Crisp Pure White Light
        (
            "#ffffff", // editor_bg (pure white)
            "#f8f9fa", // gutter_bg (crisp white-gray rail)
            "#fafafa", // output_bg (clean white panel)
            "#18181b", // fg
            "#a1a1aa", // muted
            "#e4e4e7", // border
            "#d97706", // ans_fg (amber-gold)
            "#f4f4f5", // bar_bg
            "#18181b", // brand_bg
            "#f59e0b", // brand_fg
            "#ffffff", // pill_bg
            "#27272a", // pill_fg
            "#d4d4d8", // pill_border
            "#e4e4e7", // pill_hover
            "#d97706", // caret_color
        )
    };

    format!(
        "
        window, .root-box {{
            background-color: {editor_bg};
            color: {fg};
        }}
        .bottom-bar {{
            background-color: {bar_bg};
            border-top: 1px solid {border};
            padding: 5px 10px;
        }}
        .brand-label {{
            font-family: 'JetBrains Mono', 'Roboto Mono', 'Noto Sans Mono', monospace;
            font-weight: 700;
            font-size: 11px;
            color: {brand_fg};
            background-color: {brand_bg};
            padding: 3px 8px;
            border-radius: 0px;
        }}
        .page-pill {{
            font-family: 'JetBrains Mono', 'Roboto Mono', 'Noto Sans Mono', monospace;
            font-weight: 500;
            font-size: 11px;
            color: {pill_fg};
            background-color: {pill_bg};
            border-top: 1px solid {pill_border};
            border-bottom: 1px solid {pill_border};
            padding: 2px 8px;
        }}
        .status-label {{
            font-family: 'JetBrains Mono', 'Roboto Mono', 'Noto Sans Mono', monospace;
            font-size: 11px;
            color: {muted};
        }}
        .flat-btn {{
            background-image: none;
            background-color: {pill_bg};
            border: 1px solid {pill_border};
            border-radius: 0px;
            padding: 2px 8px;
            min-height: 0px;
            min-width: 0px;
            color: {pill_fg};
            font-family: 'JetBrains Mono', 'Roboto Mono', 'Noto Sans Mono', monospace;
            font-size: 11px;
            box-shadow: none;
        }}
        .flat-btn:hover {{
            background-color: {pill_hover};
        }}
        textview, textview text {{
            background-color: {editor_bg};
            color: {fg};
            caret-color: {caret_color};
            font-family: 'JetBrains Mono', 'Roboto Mono', 'Noto Sans Mono', monospace;
            font-size: 13px;
        }}
        .gutter-view, .gutter-view text {{
            background-color: {gutter_bg};
            color: {muted};
            font-size: 12px;
        }}
        .output-view, .output-view text {{
            background-color: {output_bg};
            color: {ans_fg};
            font-weight: 600;
        }}
        .v-divider {{
            background-color: {border};
            min-width: 1px;
        }}
        "
    )
}

fn configure_tags(table: &gtk::TextTagTable, dark: bool) {
    let add_or_update = |name: &str, color: &str, weight: i32, italic: bool| {
        let tag = table.lookup(name).unwrap_or_else(|| {
            let t = gtk::TextTag::new(Some(name));
            table.add(&t);
            t
        });
        tag.set_foreground(Some(color));
        tag.set_weight(weight);
        tag.set_style(if italic {
            pango::Style::Italic
        } else {
            pango::Style::Normal
        });
    };

    if dark {
        add_or_update("num", "#fafaf9", 400, false);
        add_or_update("ident", "#fde68a", 500, false);
        add_or_update("fn", "#38bdf8", 600, false);
        add_or_update("op", "#f59e0b", 500, false);
        add_or_update("paren", "#78716c", 400, false);
        add_or_update("assign", "#fb923c", 600, false);
        add_or_update("comment", "#57534e", 400, true);
    } else {
        add_or_update("num", "#18181b", 400, false);
        add_or_update("ident", "#b45309", 600, false);
        add_or_update("fn", "#0284c7", 600, false);
        add_or_update("op", "#d97706", 600, false);
        add_or_update("paren", "#71717a", 400, false);
        add_or_update("assign", "#ea580c", 700, false);
        add_or_update("comment", "#a1a1aa", 400, true);
    }
}

fn configure_output_tags(table: &gtk::TextTagTable, dark: bool) {
    let tag = table.lookup("hover_link").unwrap_or_else(|| {
        let t = gtk::TextTag::new(Some("hover_link"));
        table.add(&t);
        t
    });
    tag.set_underline(pango::Underline::None);
    tag.set_weight(700);
    if dark {
        tag.set_foreground(Some("#a3e635")); // Acid lime on hover in Dark theme
    } else {
        tag.set_foreground(Some("#16a34a")); // Crisp emerald green on hover in Light theme
    }
}

fn main() {
    gtk::init().expect("Failed to initialize GTK");

    let mut initial_state = load_state();

    // Check CLI arguments for a `.num` file to import
    if let Some(arg_path) = std::env::args()
        .skip(1)
        .find(|a| a.to_lowercase().ends_with(".num") && Path::new(a).exists())
    {
        if let Ok(content) = fs::read_to_string(&arg_path) {
            let title = Path::new(&arg_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Импорт")
                .to_string();
            if initial_state.pages.len() == 1 && initial_state.pages[0].content.trim().is_empty() {
                initial_state.pages[0].title = title;
                initial_state.pages[0].content = content.trim_end_matches('\n').to_string();
                initial_state.active_page = 0;
            } else if let Some(idx) = initial_state.pages.iter().position(|p| p.title == title) {
                initial_state.pages[idx].content = content.trim_end_matches('\n').to_string();
                initial_state.active_page = idx;
            } else {
                initial_state.pages.push(Page {
                    title,
                    content: content.trim_end_matches('\n').to_string(),
                });
                initial_state.active_page = initial_state.pages.len() - 1;
            }
            save_state(&initial_state);
        }
    }

    let state = Rc::new(RefCell::new(initial_state));
    let current_dark = Rc::new(RefCell::new(detect_system_dark()));
    let eval_cache: Rc<RefCell<Vec<Option<f64>>>> = Rc::new(RefCell::new(Vec::new()));

    let css_provider = gtk::CssProvider::new();
    let screen = gdk::Screen::default().expect("No GDK screen");
    gtk::StyleContext::add_provider_for_screen(
        &screen,
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Счёт");
    window.set_decorated(false);
    window.set_default_size(640, 440);
    window.set_size_request(440, 300);

    let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root_box.style_context().add_class("root-box");

    // --- Main Split Area inside ScrolledWindow ---
    let scrolled = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_vexpand(true);

    let split_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let gutter_view = gtk::TextView::new();
    gutter_view.style_context().add_class("gutter-view");
    gutter_view.set_editable(false);
    gutter_view.set_cursor_visible(false);
    gutter_view.set_can_focus(false);
    gutter_view.set_left_margin(12);
    gutter_view.set_right_margin(12);
    gutter_view.set_top_margin(14);
    gutter_view.set_bottom_margin(14);
    gutter_view.set_pixels_above_lines(5);
    gutter_view.set_pixels_below_lines(5);
    gutter_view.set_size_request(48, -1);

    let gutter_divider = gtk::Box::new(gtk::Orientation::Vertical, 0);
    gutter_divider.style_context().add_class("v-divider");

    let input_view = gtk::TextView::new();
    input_view.set_hexpand(true);
    input_view.set_left_margin(14);
    input_view.set_right_margin(14);
    input_view.set_top_margin(14);
    input_view.set_bottom_margin(14);
    input_view.set_pixels_above_lines(5);
    input_view.set_pixels_below_lines(5);

    let divider = gtk::Box::new(gtk::Orientation::Vertical, 0);
    divider.style_context().add_class("v-divider");

    let output_view = gtk::TextView::new();
    output_view.style_context().add_class("output-view");
    output_view.set_editable(false);
    output_view.set_cursor_visible(false);
    output_view.set_can_focus(false);
    output_view.set_left_margin(18);
    output_view.set_right_margin(18);
    output_view.set_top_margin(14);
    output_view.set_bottom_margin(14);
    output_view.set_pixels_above_lines(5);
    output_view.set_pixels_below_lines(5);
    output_view.set_size_request(185, -1);
    output_view.add_events(
        gdk::EventMask::POINTER_MOTION_MASK
            | gdk::EventMask::LEAVE_NOTIFY_MASK
            | gdk::EventMask::BUTTON_PRESS_MASK,
    );
    output_view.connect_realize(|view| {
        if let Some(text_win) = gtk::prelude::TextViewExt::window(view, gtk::TextWindowType::Text) {
            text_win.set_events(
                text_win.events()
                    | gdk::EventMask::POINTER_MOTION_MASK
                    | gdk::EventMask::LEAVE_NOTIFY_MASK
                    | gdk::EventMask::BUTTON_PRESS_MASK,
            );
        }
    });

    split_box.pack_start(&gutter_view, false, false, 0);
    split_box.pack_start(&gutter_divider, false, false, 0);
    split_box.pack_start(&input_view, true, true, 0);
    split_box.pack_start(&divider, false, false, 0);
    split_box.pack_start(&output_view, false, true, 0);

    scrolled.add(&split_box);
    root_box.pack_start(&scrolled, true, true, 0);

    // --- Single Bottom Hardware Bar (locale-aware, no auto-sum) ---
    let ru = is_russian_locale();
    let bottom_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bottom_bar.style_context().add_class("bottom-bar");

    let brand_label = gtk::Label::new(Some(if ru { "счёт" } else { "schet" }));
    brand_label.style_context().add_class("brand-label");
    bottom_bar.pack_start(&brand_label, false, false, 0);

    let page_nav_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let prev_page_btn = gtk::Button::with_label("‹");
    prev_page_btn.style_context().add_class("flat-btn");
    prev_page_btn.set_tooltip_text(Some(if ru { "Предыдущий лист" } else { "Previous sheet" }));

    let page_title_label = gtk::Label::new(Some(if ru { "Лист 1 · 1/1" } else { "Sheet 1 · 1/1" }));
    page_title_label.style_context().add_class("page-pill");

    let next_page_btn = gtk::Button::with_label("›");
    next_page_btn.style_context().add_class("flat-btn");
    next_page_btn.set_tooltip_text(Some(if ru { "Следующий лист" } else { "Next sheet" }));

    page_nav_box.pack_start(&prev_page_btn, false, false, 0);
    page_nav_box.pack_start(&page_title_label, false, false, 0);
    page_nav_box.pack_start(&next_page_btn, false, false, 0);
    bottom_bar.pack_start(&page_nav_box, false, false, 2);

    let new_page_btn = gtk::Button::with_label("+");
    new_page_btn.style_context().add_class("flat-btn");
    new_page_btn.set_tooltip_text(Some(if ru { "Новый лист (Ctrl+N)" } else { "New sheet (Ctrl+N)" }));
    bottom_bar.pack_start(&new_page_btn, false, false, 0);

    let clear_btn = gtk::Button::with_label(if ru { "Очистить" } else { "Clear" });
    clear_btn.style_context().add_class("flat-btn");
    clear_btn.set_tooltip_text(Some(if ru { "Очистить лист (Ctrl+L)" } else { "Clear sheet (Ctrl+L)" }));
    bottom_bar.pack_start(&clear_btn, false, false, 0);

    let copy_all_btn = gtk::Button::with_label(if ru { "Копировать" } else { "Copy" });
    copy_all_btn.style_context().add_class("flat-btn");
    copy_all_btn.set_tooltip_text(Some(if ru { "Скопировать все ответы" } else { "Copy all results" }));
    bottom_bar.pack_start(&copy_all_btn, false, false, 0);

    let status_right = gtk::Label::new(Some(""));
    status_right.style_context().add_class("status-label");
    bottom_bar.pack_end(&status_right, false, false, 4);

    let bottom_event_box = gtk::EventBox::new();
    bottom_event_box.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    bottom_event_box.add(&bottom_bar);
    {
        let win = window.clone();
        bottom_event_box.connect_button_press_event(move |_, event| {
            if event.button() == 1 {
                let (rx, ry) = event.root();
                win.begin_move_drag(1, rx as i32, ry as i32, event.time());
            }
            glib::Propagation::Proceed
        });
    }

    root_box.pack_start(&bottom_event_box, false, false, 0);

    window.add(&root_box);

    let input_buf = input_view.buffer().expect("No input buffer");
    let gutter_buf = gutter_view.buffer().expect("No gutter buffer");
    let output_buf = output_view.buffer().expect("No output buffer");
    let tag_table = input_buf.tag_table().expect("No tag table");
    let output_tag_table = output_buf.tag_table().expect("No output tag table");

    // Apply initial system theme
    {
        let dark = *current_dark.borrow();
        let css = build_css(dark);
        let _ = css_provider.load_from_data(css.as_bytes());
        configure_tags(&tag_table, dark);
        configure_output_tags(&output_tag_table, dark);
    }

    let is_loading_page = Rc::new(RefCell::new(false));
    let hovered_line: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

    let apply_hover_tag = {
        let output_buf = output_buf.clone();
        let eval_cache = eval_cache.clone();
        let hovered_line = hovered_line.clone();
        Rc::new(move || {
            let (start, end) = output_buf.bounds();
            output_buf.remove_tag_by_name("hover_link", &start, &end);
            if let Some(line) = *hovered_line.borrow() {
                if let Some(Some(_)) = eval_cache.borrow().get(line) {
                    let line_start = output_buf.iter_at_line(line as i32);
                    let mut line_end = line_start;
                    if !line_end.ends_line() {
                        line_end.forward_to_line_end();
                    }
                    output_buf.apply_tag_by_name("hover_link", &line_start, &line_end);
                }
            }
        })
    };

    let recalculate = {
        let input_buf = input_buf.clone();
        let gutter_buf = gutter_buf.clone();
        let output_buf = output_buf.clone();
        let page_title_label = page_title_label.clone();
        let state = state.clone();
        let eval_cache = eval_cache.clone();
        let is_loading_page = is_loading_page.clone();
        let apply_hover_tag = apply_hover_tag.clone();

        Rc::new(move || {
            let (start, end) = input_buf.bounds();
            let text = input_buf
                .text(&start, &end, false)
                .map(|g| g.to_string())
                .unwrap_or_default();

            input_buf.remove_all_tags(&start, &end);

            let evals = evaluate_document(&text);
            let lines: Vec<&str> = text.split('\n').collect();

            let mut gutter_lines = Vec::with_capacity(evals.len());
            let mut out_lines = Vec::with_capacity(evals.len());
            let mut vals = Vec::with_capacity(evals.len());

            let mut line_byte_offset = 0usize;
            for (i, ev) in evals.iter().enumerate() {
                gutter_lines.push(format!("{:02}", i + 1));
                out_lines.push(ev.formatted.clone());
                vals.push(ev.value);

                let line_str = lines.get(i).copied().unwrap_or("");
                for span in &ev.spans {
                    let tag_name = match span.kind {
                        TokenKind::Number => "num",
                        TokenKind::Ident => "ident",
                        TokenKind::Function => "fn",
                        TokenKind::Operator => "op",
                        TokenKind::Paren => "paren",
                        TokenKind::Assign => "assign",
                        TokenKind::Comment => "comment",
                    };
                    let start_chars = text[..line_byte_offset + span.start_byte].chars().count() as i32;
                    let end_chars = text[..line_byte_offset + span.end_byte].chars().count() as i32;
                    let s_iter = input_buf.iter_at_offset(start_chars);
                    let e_iter = input_buf.iter_at_offset(end_chars);
                    input_buf.apply_tag_by_name(tag_name, &s_iter, &e_iter);
                }
                line_byte_offset += line_str.len() + 1;
            }

            gutter_buf.set_text(&gutter_lines.join("\n"));
            output_buf.set_text(&out_lines.join("\n"));
            *eval_cache.borrow_mut() = vals;
            apply_hover_tag();

            if !*is_loading_page.borrow() {
                let mut st = state.borrow_mut();
                let active = st.active_page;
                if let Some(page) = st.pages.get_mut(active) {
                    page.content = text;
                }
                page_title_label.set_text(&format!(
                    "{} · {}/{}",
                    st.pages[active].title,
                    active + 1,
                    st.pages.len()
                ));
                save_state(&st);
            } else {
                let st = state.borrow();
                let active = st.active_page;
                page_title_label.set_text(&format!(
                    "{} · {}/{}",
                    st.pages[active].title,
                    active + 1,
                    st.pages.len()
                ));
            }
        })
    };

    // Live system theme synchronization (checks /run/user/1000/theme-mode & gtk settings every 250ms)
    {
        let current_dark = current_dark.clone();
        let css_provider = css_provider.clone();
        let tag_table = tag_table.clone();
        let output_tag_table = output_tag_table.clone();
        let recalculate = recalculate.clone();
        glib::timeout_add_local(Duration::from_millis(250), move || {
            let sys_dark = detect_system_dark();
            if *current_dark.borrow() != sys_dark {
                *current_dark.borrow_mut() = sys_dark;
                let css = build_css(sys_dark);
                let _ = css_provider.load_from_data(css.as_bytes());
                configure_tags(&tag_table, sys_dark);
                configure_output_tags(&output_tag_table, sys_dark);
                recalculate();
            }
            glib::ControlFlow::Continue
        });
    }

    let load_active_page = {
        let state = state.clone();
        let input_buf = input_buf.clone();
        let is_loading_page = is_loading_page.clone();
        let recalculate = recalculate.clone();

        Rc::new(move || {
            *is_loading_page.borrow_mut() = true;
            let content = {
                let st = state.borrow();
                st.pages[st.active_page].content.clone()
            };
            input_buf.set_text(&content);
            *is_loading_page.borrow_mut() = false;
            recalculate();
        })
    };

    // Initial load
    load_active_page();

    // Recalculate on buffer change
    {
        let recalculate = recalculate.clone();
        let is_loading_page = is_loading_page.clone();
        input_buf.connect_changed(move |_| {
            if !*is_loading_page.borrow() {
                recalculate();
            }
        });
    }

    // Click answer in output_view to copy to clipboard + hover pointer/color highlight
    {
        let eval_cache_click = eval_cache.clone();
        let status_right = status_right.clone();
        output_view.connect_button_press_event(move |view, event| {
            if event.button() == 1 {
                let (wx, wy) = event.position();
                let (_bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Text, wx as i32, wy as i32);
                let (iter, _) = view.line_at_y(by);
                let line = iter.line() as usize;
                if let Some(Some(val)) = eval_cache_click.borrow().get(line) {
                    let formatted = format_number(*val);
                    let clip = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                    clip.set_text(&formatted);
                    if ru {
                        status_right.set_text(&format!("Скопировано: {}", formatted));
                    } else {
                        status_right.set_text(&format!("Copied: {}", formatted));
                    }
                }
            }
            glib::Propagation::Proceed
        });

        let eval_cache_hover = eval_cache.clone();
        let hovered_line_m = hovered_line.clone();
        let apply_hover_tag_m = apply_hover_tag.clone();
        output_view.connect_motion_notify_event(move |view, event| {
            let (wx, wy) = event.position();
            let (_bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Text, wx as i32, wy as i32);
            let (iter, _) = view.line_at_y(by);
            let line = iter.line() as usize;

            let over_answer = matches!(eval_cache_hover.borrow().get(line), Some(Some(_)));
            let new_hover = if over_answer { Some(line) } else { None };
            if *hovered_line_m.borrow() != new_hover {
                *hovered_line_m.borrow_mut() = new_hover;
                apply_hover_tag_m();
            }

            if let Some(text_win) = gtk::prelude::TextViewExt::window(view, gtk::TextWindowType::Text) {
                let display = view.display();
                let cursor = if over_answer {
                    gdk::Cursor::from_name(&display, "pointer")
                } else {
                    gdk::Cursor::from_name(&display, "default")
                };
                text_win.set_cursor(cursor.as_ref());
            }
            glib::Propagation::Proceed
        });

        let hovered_line_l = hovered_line.clone();
        let apply_hover_tag_l = apply_hover_tag.clone();
        output_view.connect_leave_notify_event(move |view, _| {
            if hovered_line_l.borrow().is_some() {
                *hovered_line_l.borrow_mut() = None;
                apply_hover_tag_l();
            }
            if let Some(text_win) = gtk::prelude::TextViewExt::window(view, gtk::TextWindowType::Text) {
                text_win.set_cursor(None);
            }
            glib::Propagation::Proceed
        });
    }

    // Clear button (if already empty and >1 page, removes the empty page)
    {
        let input_buf = input_buf.clone();
        let status_right = status_right.clone();
        let state = state.clone();
        let load_active_page = load_active_page.clone();
        clear_btn.connect_clicked(move |_| {
            let (start, end) = input_buf.bounds();
            let current_text = input_buf
                .text(&start, &end, false)
                .map(|g| g.to_string())
                .unwrap_or_default();
            if current_text.trim().is_empty() {
                let removed = {
                    let mut st = state.borrow_mut();
                    if st.pages.len() > 1 {
                        let active = st.active_page;
                        st.pages.remove(active);
                        for (idx, p) in st.pages.iter_mut().enumerate() {
                            p.title = default_page_title(idx + 1);
                        }
                        if st.active_page >= st.pages.len() {
                            st.active_page = st.pages.len() - 1;
                        }
                        save_state(&st);
                        true
                    } else {
                        false
                    }
                };
                if removed {
                    load_active_page();
                    status_right.set_text(if ru { "Лист удалён" } else { "Sheet deleted" });
                    return;
                }
            }
            input_buf.set_text("");
            status_right.set_text(if ru { "Очищено" } else { "Cleared" });
        });
    }

    // Copy All button
    {
        let eval_cache = eval_cache.clone();
        let status_right = status_right.clone();
        copy_all_btn.connect_clicked(move |_| {
            let lines: Vec<String> = eval_cache
                .borrow()
                .iter()
                .filter_map(|v| v.map(format_number))
                .collect();
            let joined = lines.join("\n");
            let clip = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
            clip.set_text(&joined);
            if ru {
                status_right.set_text(&format!("Скопировано строк: {}", lines.len()));
            } else {
                status_right.set_text(&format!("Copied lines: {}", lines.len()));
            }
        });
    }

    // New Page button
    {
        let state = state.clone();
        let load_active_page = load_active_page.clone();
        new_page_btn.connect_clicked(move |_| {
            {
                let mut st = state.borrow_mut();
                let next_num = st.pages.len() + 1;
                st.pages.push(Page {
                    title: default_page_title(next_num),
                    content: String::new(),
                });
                st.active_page = st.pages.len() - 1;
                save_state(&st);
            }
            load_active_page();
        });
    }

    // Prev / Next Page buttons
    {
        let state = state.clone();
        let load_active_page = load_active_page.clone();
        prev_page_btn.connect_clicked(move |_| {
            let changed = {
                let mut st = state.borrow_mut();
                if st.active_page > 0 {
                    st.active_page -= 1;
                    save_state(&st);
                    true
                } else {
                    false
                }
            };
            if changed {
                load_active_page();
            }
        });
    }
    {
        let state = state.clone();
        let load_active_page = load_active_page.clone();
        next_page_btn.connect_clicked(move |_| {
            let changed = {
                let mut st = state.borrow_mut();
                if st.active_page + 1 < st.pages.len() {
                    st.active_page += 1;
                    save_state(&st);
                    true
                } else {
                    false
                }
            };
            if changed {
                load_active_page();
            }
        });
    }

    // Keyboard shortcuts
    {
        let new_page_btn = new_page_btn.clone();
        let prev_page_btn = prev_page_btn.clone();
        let next_page_btn = next_page_btn.clone();
        let clear_btn = clear_btn.clone();
        window.connect_key_press_event(move |win, event| {
            let state_mod = event.state();
            let ctrl = state_mod.contains(ModifierType::CONTROL_MASK);
            let alt = state_mod.contains(ModifierType::MOD1_MASK);
            let key = event.keyval();
            if ctrl && (key == keys::n || key == keys::N) {
                new_page_btn.clicked();
                return glib::Propagation::Stop;
            }
            if (ctrl && key == keys::Page_Up) || (alt && key == keys::Left) {
                prev_page_btn.clicked();
                return glib::Propagation::Stop;
            }
            if (ctrl && key == keys::Page_Down) || (alt && key == keys::Right) {
                next_page_btn.clicked();
                return glib::Propagation::Stop;
            }
            if ctrl && (key == keys::l || key == keys::L) {
                clear_btn.clicked();
                return glib::Propagation::Stop;
            }
            if ctrl && (key == keys::q || key == keys::Q || key == keys::w || key == keys::W) {
                win.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        glib::Propagation::Proceed
    });

    window.show_all();
    input_view.grab_focus();
    gtk::main();
}
