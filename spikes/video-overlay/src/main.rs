//! Spike 2.1: transparent WebKitGTK webview composed over a libmpv GtkGLArea (Tauri 2).
//!
//! Plan A: `GtkOverlay { GtkGLArea (main child), WebView (overlay child, transparent) }`.
//! Plan B: `GtkStack { webview page | video page }`, keys relayed to mpv, info via `show-text`.

#![allow(deprecated)] // glib channels: still the idiom for the gtk 0.18 / glib 0.18 pair used by Tauri 2

use gtk::prelude::*;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use webkit2gtk::WebViewExt;
use tauri::{WebviewUrl, WebviewWindowBuilder};

// ---------------------------------------------------------------- CLI

#[derive(Clone, Debug)]
struct Cli {
    plan_b: bool,
    measure: bool,
    duration: u64,
    video: String,
    out: Option<String>,
    shot_dir: Option<String>,
    no_overlay: bool,
}

fn parse_cli() -> Cli {
    let mut cli = Cli {
        plan_b: false,
        measure: false,
        duration: 30,
        video: concat!(env!("CARGO_MANIFEST_DIR"), "/media/test-1080p30.mp4").into(),
        out: None,
        shot_dir: None,
        no_overlay: false,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = |i: usize| args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--plan" => {
                cli.plan_b = next(i).eq_ignore_ascii_case("b");
                i += 1;
            }
            "--measure" => cli.measure = true,
            // baseline: keep the webview out of the picture (hidden) to isolate its cost
            "--no-overlay" => cli.no_overlay = true,
            "--duration" => {
                cli.duration = next(i).parse().unwrap_or(30);
                i += 1;
            }
            "--video" => {
                cli.video = next(i);
                i += 1;
            }
            "--out" => {
                cli.out = Some(next(i));
                i += 1;
            }
            "--screenshots" => {
                cli.shot_dir = Some(next(i));
                i += 1;
            }
            other => eprintln!("unknown argument: {other}"),
        }
        i += 1;
    }
    cli
}

// ---------------------------------------------------------------- GL loading

type GetProc = unsafe extern "C" fn(*const c_char) -> *mut c_void;

struct Loader {
    egl: Option<GetProc>,
    glx: Option<GetProc>,
    prefer_glx: bool,
}

unsafe fn sym(lib: &str, name: &str) -> Option<GetProc> {
    let l = CString::new(lib).ok()?;
    let h = libc::dlopen(l.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
    if h.is_null() {
        return None;
    }
    let n = CString::new(name).ok()?;
    let p = libc::dlsym(h, n.as_ptr());
    if p.is_null() {
        None
    } else {
        Some(std::mem::transmute::<*mut c_void, GetProc>(p))
    }
}

thread_local! {
    static LOADER: RefCell<Option<Loader>> = const { RefCell::new(None) };
}

fn init_loader() {
    let prefer_glx = gdk::Display::default()
        .map(|d| d.type_().name().contains("X11"))
        .unwrap_or(false);
    let l = unsafe {
        Loader {
            egl: sym("libEGL.so.1", "eglGetProcAddress"),
            glx: sym("libGL.so.1", "glXGetProcAddressARB"),
            prefer_glx,
        }
    };
    LOADER.with(|c| *c.borrow_mut() = Some(l));
}

/// Resolve a GL entry point through the loader matching the GDK backend, with fallbacks.
fn gl_proc(name: &str) -> *mut c_void {
    let Ok(cname) = CString::new(name) else {
        return std::ptr::null_mut();
    };
    LOADER.with(|c| {
        let b = c.borrow();
        let Some(l) = b.as_ref() else {
            return std::ptr::null_mut();
        };
        let order = if l.prefer_glx { [l.glx, l.egl] } else { [l.egl, l.glx] };
        for f in order.into_iter().flatten() {
            let p = unsafe { f(cname.as_ptr()) };
            if !p.is_null() {
                return p;
            }
        }
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, cname.as_ptr()) }
    })
}

fn get_proc_address(_: &(), name: &str) -> *mut c_void {
    gl_proc(name)
}

type GlGetIntegerv = unsafe extern "C" fn(u32, *mut i32);
type GlGetError = unsafe extern "C" fn() -> u32;
const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;

// ---------------------------------------------------------------- misc helpers

fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

/// utime + stime of this process, in seconds.
fn cpu_seconds() -> f64 {
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|s| {
            let rest = s.rsplit_once(')')?.1.to_string();
            let f: Vec<&str> = rest.split_whitespace().collect();
            // after ')' : state is field index 0, utime index 11, stime index 12
            let u: f64 = f.get(11)?.parse().ok()?;
            let st: f64 = f.get(12)?.parse().ok()?;
            Some((u + st) / ticks)
        })
        .unwrap_or(0.0)
}

fn prop_i64(mpv: &Mpv, name: &str) -> i64 {
    mpv.get_property::<i64>(name).unwrap_or(-1)
}

fn prop_str(mpv: &Mpv, name: &str) -> String {
    mpv.get_property::<String>(name).unwrap_or_else(|_| "n/a".into())
}

/// Plan B info channel: mpv's own OSD (text formatted here, no property expansion needed).
fn osd(mpv: &Mpv, text: &str, ms: u32) {
    let _ = mpv.command("show-text", &[text, &ms.to_string()]);
}

fn position_text(mpv: &Mpv) -> String {
    let t = mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
    let d = mpv.get_property::<f64>("duration").unwrap_or(0.0);
    format!("{t:.1}s / {d:.0}s")
}

fn display_ptr(sym_name: &str) -> *const c_void {
    unsafe {
        let Some(d) = gdk::Display::default() else {
            return std::ptr::null();
        };
        let n = CString::new(sym_name).unwrap();
        let f = libc::dlsym(libc::RTLD_DEFAULT, n.as_ptr());
        if f.is_null() {
            return std::ptr::null();
        }
        let f: unsafe extern "C" fn(*mut gdk::ffi::GdkDisplay) -> *mut c_void = std::mem::transmute(f);
        let p: *mut gdk::ffi::GdkDisplay = glib::translate::ToGlibPtr::to_glib_none(&d).0;
        f(p) as *const c_void
    }
}

// ---------------------------------------------------------------- shared state

struct App {
    mpv: &'static Mpv,
    rc: RefCell<Option<RenderContext<'static>>>,
    pending: RefCell<Option<String>>,
    gl_errors: Cell<u32>,
    renders: Cell<u64>,
    gl_error_codes: RefCell<Vec<u32>>,
}

impl App {
    fn load(&self, path: &str) {
        if self.rc.borrow().is_some() {
            if let Err(e) = self.mpv.command("loadfile", &[path, "replace"]) {
                eprintln!("loadfile failed: {e}");
            } else {
                eprintln!("loadfile {path}");
            }
        } else {
            *self.pending.borrow_mut() = Some(path.to_string());
        }
    }
}

struct PageState(Mutex<Value>);

struct StartTx(Mutex<glib::Sender<()>>);

#[tauri::command]
fn report(state: Value, store: tauri::State<'_, Arc<PageState>>) {
    *store.0.lock().unwrap() = state;
}

#[tauri::command]
fn start_playback(tx: tauri::State<'_, StartTx>) {
    let _ = tx.0.lock().unwrap().send(());
}

// ---------------------------------------------------------------- mpv

fn create_mpv(measure: bool) -> &'static Mpv {
    // GTK's init calls setlocale(LC_ALL, ""); mpv_create fails unless LC_NUMERIC is "C".
    unsafe {
        let c = CString::new("C").unwrap();
        libc::setlocale(libc::LC_NUMERIC, c.as_ptr());
    }
    let mpv = Mpv::with_initializer(|i| {
        // closed configuration (ARCHITECTURE.md 5.1)
        for (k, v) in [
            ("config", "no"),
            ("load-scripts", "no"),
            ("ytdl", "no"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("osc", "no"),
            ("vo", "libmpv"),
            ("hwdec", "auto-safe"),
            ("loop-file", "inf"),
            ("keep-open", "yes"),
        ] {
            i.set_property(k, v)?;
        }
        if measure {
            i.set_property("ao", "null")?;
        }
        Ok(())
    })
    .expect("mpv init (is LC_NUMERIC=C?)");
    Box::leak(Box::new(mpv))
}

fn build_glarea(app: &Rc<App>, sched: glib::Sender<()>, pending_flag: Arc<AtomicBool>) -> gtk::GLArea {
    let area = gtk::GLArea::new();
    area.set_has_depth_buffer(false);
    area.set_has_stencil_buffer(false);
    area.set_auto_render(false);
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_required_version(3, 2);

    let a = app.clone();
    area.connect_realize(move |area| {
        area.make_current();
        if let Some(e) = area.error() {
            eprintln!("GLArea error on realize: {e}");
            return;
        }
        let mut params: Vec<RenderParam<()>> = vec![
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams { get_proc_address, ctx: () }),
        ];
        let is_x11 = gdk::Display::default().map(|d| d.type_().name().contains("X11")).unwrap_or(false);
        let wl = if is_x11 { std::ptr::null() } else { display_ptr("gdk_wayland_display_get_wl_display") };
        let x11 = if is_x11 { display_ptr("gdk_x11_display_get_xdisplay") } else { std::ptr::null() };
        if !wl.is_null() {
            params.push(RenderParam::WaylandDisplay(wl));
        } else if !x11.is_null() {
            params.push(RenderParam::X11Display(x11));
        }
        match a.mpv.create_render_context(params) {
            Ok(mut rc) => {
                let tx = sched.clone();
                let flag = pending_flag.clone();
                // Never touch GL or mpv here: only ask the GTK main thread to redraw.
                rc.set_update_callback(move || {
                    if !flag.swap(true, Ordering::AcqRel) {
                        let _ = tx.send(());
                    }
                });
                *a.rc.borrow_mut() = Some(rc);
                eprintln!("render context created (realize), GL context current");
            }
            Err(e) => {
                eprintln!("render context creation failed: {e}");
                return;
            }
        }
        // Render context exists before loadfile, otherwise mpv would open its own window.
        let p = a.pending.borrow_mut().take();
        if let Some(path) = p {
            a.load(&path);
        }
    });

    let a = app.clone();
    let flag = pending_flag_for_render(&a);
    area.connect_render(move |area, _ctx| {
        flag();
        let rc = a.rc.borrow();
        let Some(rc) = rc.as_ref() else {
            return glib::Propagation::Stop;
        };
        unsafe {
            let (Some(gi), Some(ge)) = (
                std::mem::transmute::<*mut c_void, Option<GlGetIntegerv>>(gl_proc("glGetIntegerv")),
                std::mem::transmute::<*mut c_void, Option<GlGetError>>(gl_proc("glGetError")),
            ) else {
                return glib::Propagation::Stop;
            };
            while ge() != 0 {} // flush stale errors that are not ours
            let mut fbo = 0i32;
            gi(GL_FRAMEBUFFER_BINDING, &mut fbo);
            let s = area.scale_factor();
            let (w, h) = (area.allocated_width() * s, area.allocated_height() * s);
            if let Err(e) = rc.render::<()>(fbo, w, h, true) {
                eprintln!("render error: {e}");
            }
            rc.report_swap();
            let err = ge();
            if err != 0 {
                a.gl_errors.set(a.gl_errors.get() + 1);
                a.gl_error_codes.borrow_mut().push(err);
            }
        }
        a.renders.set(a.renders.get() + 1);
        glib::Propagation::Stop
    });
    area
}

thread_local! {
    static PENDING: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

fn pending_flag_for_render(_a: &Rc<App>) -> impl Fn() {
    || PENDING.with(|p| {
        if let Some(f) = p.borrow().as_ref() {
            f.store(false, Ordering::Release);
        }
    })
}

// ---------------------------------------------------------------- screenshots

/// Draw the widget tree through GTK's own draw path (GLArea texture + webview) into a PNG.
fn shot(widget: &gtk::Widget, path: &str) -> String {
    let a = widget.allocation();
    let Ok(surface) = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, a.width(), a.height()) else {
        return "surface error".into();
    };
    let Ok(cr) = gtk::cairo::Context::new(&surface) else {
        return "cairo error".into();
    };
    cr.set_source_rgb(1.0, 0.0, 1.0); // magenta: shows through only if nothing painted
    let _ = cr.paint();
    widget.draw(&cr);
    drop(cr);
    match std::fs::File::create(path).map(|mut f| surface.write_to_png(&mut f)) {
        Ok(Ok(())) => format!("ok {path}"),
        e => format!("error {e:?}"),
    }
}

fn inject_key(webview: &gtk::Widget, keyval: u32, keycode: u16, ch: &str) -> bool {
    unsafe {
        let Some(win) = webview.window() else {
            return false;
        };
        let ev = gdk::ffi::gdk_event_new(gdk::ffi::GDK_KEY_PRESS);
        let k = &mut (*ev).key;
        let wp: *mut gdk::ffi::GdkWindow = glib::translate::ToGlibPtr::to_glib_none(&win).0;
        k.window = glib::gobject_ffi::g_object_ref(wp as *mut _) as *mut gdk::ffi::GdkWindow;
        k.send_event = 1;
        k.time = 0;
        k.state = 0;
        k.keyval = keyval;
        k.hardware_keycode = keycode;
        k.length = ch.len() as i32;
        let cs = CString::new(ch).unwrap();
        k.string = glib::ffi::g_strdup(cs.as_ptr());
        if let Some(kb) = gdk::Display::default().and_then(|d| d.default_seat()).and_then(|s| s.keyboard()) {
            let dp: *mut gdk::ffi::GdkDevice = glib::translate::ToGlibPtr::to_glib_none(&kb).0;
            gdk::ffi::gdk_event_set_device(ev, dp);
        }
        let event: gdk::Event = glib::translate::from_glib_full(ev);
        let mut event = event;
        gtk::main_do_event(&mut event);
    }
    true
}

// ---------------------------------------------------------------- main

fn main() {
    let cli = parse_cli();
    if !std::path::Path::new(&cli.video).exists() {
        eprintln!("video not found: {} (see README: ffmpeg testsrc2 command)", cli.video);
    }
    let page = Arc::new(PageState(Mutex::new(Value::Null)));
    let (start_tx, start_rx) = glib::MainContext::channel::<()>(glib::Priority::DEFAULT);

    let cli_setup = cli.clone();
    let page_setup = page.clone();
    let result = tauri::Builder::default()
        .manage(page.clone())
        .manage(StartTx(Mutex::new(start_tx)))
        .invoke_handler(tauri::generate_handler![report, start_playback])
        .setup(move |tauri_app| {
            let cli = cli_setup;
            let window = WebviewWindowBuilder::new(tauri_app, "main", WebviewUrl::App("index.html".into()))
                .title("video-overlay-spike")
                .inner_size(1280.0, 720.0)
                .transparent(true)
                .build()?;
            let gtk_window = window.gtk_window()?;
            let vbox = window.default_vbox()?;
            init_loader();
            let mpv = create_mpv(cli.measure);

            // Locate the webview in the vbox and take it out.
            let webview = vbox
                .children()
                .into_iter()
                .find(|c| c.is::<webkit2gtk::WebView>())
                .expect("webview not found in default_vbox");
            let wv = webview.clone().downcast::<webkit2gtk::WebView>().unwrap();
            vbox.remove(&webview);
            wv.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

            let app = Rc::new(App {
                mpv,
                rc: RefCell::new(None),
                pending: RefCell::new(None),
                gl_errors: Cell::new(0),
                renders: Cell::new(0),
                gl_error_codes: RefCell::new(Vec::new()),
            });

            // Redraw scheduling: mpv thread -> flag + channel -> GTK main thread queue_render.
            let (sched_tx, sched_rx) = glib::MainContext::channel::<()>(glib::Priority::DEFAULT);
            let pending_flag = Arc::new(AtomicBool::new(false));
            PENDING.with(|p| *p.borrow_mut() = Some(pending_flag.clone()));
            let area = build_glarea(&app, sched_tx, pending_flag);
            {
                let area = area.clone();
                sched_rx.attach(None, move |_| {
                    area.queue_render();
                    glib::ControlFlow::Continue
                });
            }

            let root: gtk::Widget;
            let stack = gtk::Stack::new();
            if cli.plan_b {
                stack.add_named(&wv, "ui");
                stack.add_named(&area, "video");
                stack.set_visible_child_name("ui");
                vbox.pack_start(&stack, true, true, 0);
                root = stack.clone().upcast();
            } else {
                let overlay = gtk::Overlay::new();
                overlay.add(&area);
                overlay.add_overlay(&wv);
                overlay.set_overlay_pass_through(&wv, false);
                vbox.pack_start(&overlay, true, true, 0);
                root = overlay.upcast();
            }
            gtk_window.show_all();
            wv.set_can_focus(true);
            wv.grab_focus();
            if cli.no_overlay && !cli.plan_b {
                wv.hide();
            }

            // Playback start.
            let start = {
                let app = app.clone();
                let stack = stack.clone();
                let video = cli.video.clone();
                let plan_b = cli.plan_b;
                Rc::new(move || {
                    if plan_b {
                        stack.set_visible_child_name("video");
                    }
                    app.load(&video);
                })
            };
            if cli.plan_b {
                let s = start.clone();
                start_rx.attach(None, move |_| {
                    s();
                    glib::ControlFlow::Continue
                });
            } else {
                start_rx.attach(None, |_| glib::ControlFlow::Continue);
                start(); // Plan A: GLArea is realized by show_all, ctx exists or is pending
            }

            // Plan B: relay GTK key presses to mpv while the video page is visible.
            if cli.plan_b {
                let app = app.clone();
                let stack = stack.clone();
                let gw = gtk_window.clone();
                gtk_window.connect_key_press_event(move |_, ev| {
                    if stack.visible_child_name().as_deref() != Some("video") {
                        return glib::Propagation::Proceed;
                    }
                    use gdk::keys::constants as k;
                    let key = ev.keyval();
                    let m = app.mpv;
                    if key == k::space {
                        let _ = m.command("cycle", &["pause"]);
                        let paused = m.get_property::<bool>("pause").unwrap_or(false);
                        osd(m, &format!("{}  {}", if paused { "Pause" } else { "Play" }, position_text(m)), 1500);
                    } else if key == k::Right {
                        let _ = m.command("seek", &["5"]);
                        osd(m, &format!("+5s  {}", position_text(m)), 1500);
                    } else if key == k::Left {
                        let _ = m.command("seek", &["-5"]);
                        osd(m, &format!("-5s  {}", position_text(m)), 1500);
                    } else if key == k::f {
                        if gw.window().map(|w| w.state().contains(gdk::WindowState::FULLSCREEN)).unwrap_or(false) {
                            gw.unfullscreen();
                        } else {
                            gw.fullscreen();
                        }
                    } else if key == k::Escape {
                        let _ = m.command("stop", &[]);
                        stack.set_visible_child_name("ui");
                    }
                    glib::Propagation::Stop
                });
            }

            if cli.measure {
                run_measure(cli.clone(), app.clone(), gtk_window.clone(), root, wv.clone().upcast(), stack, page_setup.clone());
            }
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("tauri error: {e}");
        std::process::exit(2);
    }
}

// ---------------------------------------------------------------- measurement

#[allow(clippy::too_many_arguments)]
fn run_measure(
    cli: Cli,
    app: Rc<App>,
    win: gtk::ApplicationWindow,
    root: gtk::Widget,
    webview: gtk::Widget,
    stack: gtk::Stack,
    page: Arc<PageState>,
) {
    const TOGGLE_START: u64 = 6; // seconds after playback start
    const CYCLES: u32 = 10; // 10 x (fullscreen, back)
    let t = Rc::new(Cell::new(0u64));
    let ui_phase = Rc::new(Cell::new(if cli.plan_b { 3u64 } else { 0 })); // plan B: webview page first
    let samples: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(Vec::new()));
    let marks: Rc<RefCell<serde_json::Map<String, Value>>> = Rc::new(RefCell::new(Default::default()));
    let last_cpu = Rc::new(Cell::new((cpu_seconds(), std::time::Instant::now())));
    let fs_toggles = Rc::new(Cell::new(0u32));
    let shots: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let started = std::time::Instant::now();
    let injected = Rc::new(Cell::new(0u32));

    // Plan B: show the UI page for a few seconds (and type into it), then switch to the video page.
    if cli.plan_b {
        let inj = injected.clone();
        let wvc = webview.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(1500), move || {
            for (kv, kc, ch) in [(0x61u32, 38u16, "a"), (0x62, 56, "b"), (0x63, 54, "c")] {
                if inject_key(&wvc, kv, kc, ch) {
                    inj.set(inj.get() + 1);
                }
            }
            glib::ControlFlow::Break
        });
        let stack2 = stack.clone();
        let app2 = app.clone();
        let video = cli.video.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
            stack2.set_visible_child_name("video");
            app2.load(&video);
            glib::ControlFlow::Break
        });
    }

    let (t2, samples2, marks2, shots2, inj2) = (t.clone(), samples.clone(), marks.clone(), shots.clone(), injected.clone());
    let (app2, win2, root2, wv2, page2, cli2) = (app.clone(), win.clone(), root.clone(), webview.clone(), page.clone(), cli.clone());
    let ui_phase2 = ui_phase.clone();
    let fs2 = fs_toggles.clone();

    glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
        let now = t2.get() + 1;
        t2.set(now);
        // In plan B the first ui_phase seconds belong to the webview page: no video yet.
        let ui = ui_phase2.get();
        if now <= ui {
            if now == ui - 1 {
                marks2.borrow_mut().insert("ui_page".into(), page2.0.lock().unwrap().clone());
                if let Some(dir) = &cli2.shot_dir {
                    let p = format!("{dir}/ui-page.png");
                    shots2.borrow_mut().push(shot(&root2, &p));
                }
            }
            return glib::ControlFlow::Continue;
        }
        let pt = now - ui; // seconds of playback
        let m = app2.mpv;
        let (cpu_now, inst) = (cpu_seconds(), std::time::Instant::now());
        let (cpu_prev, inst_prev) = last_cpu.get();
        let cpu_pct = (cpu_now - cpu_prev) / inst.duration_since(inst_prev).as_secs_f64().max(1e-6) * 100.0;
        last_cpu.set((cpu_now, inst));
        let s = json!({
            "t": pt,
            "frame_drop": prop_i64(m, "frame-drop-count"),
            "decoder_drop": prop_i64(m, "decoder-frame-drop-count"),
            "frames": prop_i64(m, "estimated-frame-number"),
            "hwdec": prop_str(m, "hwdec-current"),
            "cpu_pct": (cpu_pct * 10.0).round() / 10.0,
            "rss_kb": rss_kb(),
            "fullscreen": win2.window().map(|w| w.state().contains(gdk::WindowState::FULLSCREEN)).unwrap_or(false),
            "gl_errors": app2.gl_errors.get(),
            "renders": app2.renders.get(),
        });
        samples2.borrow_mut().push(s);

        if cli2.plan_b && pt % 2 == 0 {
            let txt = format!("OSD {}  drops {}", position_text(m), prop_i64(m, "frame-drop-count"));
            osd(m, &txt, 900);
        }
        if pt == 2 {
            // keyboard: synthetic key events through the GTK main loop to the focused widget
            if !cli2.plan_b {
                for (kv, kc, ch) in [(0x61u32, 38u16, "a"), (0x62, 56, "b"), (0x63, 54, "c"), (0x64, 40, "d"), (0x65, 26, "e")] {
                    if inject_key(&wv2, kv, kc, ch) {
                        inj2.set(inj2.get() + 1);
                    }
                }
            }
            let focus = win2.focused_widget().map(|w| w.type_().name().to_string());
            marks2.borrow_mut().insert("gtk_focus_widget".into(), json!(focus));
            marks2.borrow_mut().insert("gtk_window_active".into(), json!(win2.is_active()));
            marks2.borrow_mut().insert("webview_is_focus".into(), json!(wv2.is_focus()));
        }
        if pt == 4 {
            marks2.borrow_mut().insert("page_after_keys".into(), page2.0.lock().unwrap().clone());
            if let Some(dir) = &cli2.shot_dir {
                let p = format!("{dir}/playing-windowed.png");
                shots2.borrow_mut().push(shot(&root2, &p));
            }
        }
        if pt == TOGGLE_START - 1 {
            marks2.borrow_mut().insert("rss_kb_before_toggles".into(), json!(rss_kb()));
            marks2.borrow_mut().insert("drops_before_toggles".into(), json!({"frame_drop": prop_i64(m, "frame-drop-count"), "decoder_drop": prop_i64(m, "decoder-frame-drop-count"), "frames": prop_i64(m, "estimated-frame-number")}));
        }
        if pt == TOGGLE_START {
            let (w, root3, cli3, shots3, fs3) = (win2.clone(), root2.clone(), cli2.clone(), shots2.clone(), fs2.clone());
            let go = Rc::new(Cell::new(0u32));
            glib::timeout_add_local(std::time::Duration::from_millis(750), move || {
                let n = go.get();
                if n >= CYCLES * 2 {
                    return glib::ControlFlow::Break;
                }
                if n.is_multiple_of(2) {
                    w.fullscreen();
                } else {
                    w.unfullscreen();
                }
                if n == 3 {
                    if let Some(dir) = &cli3.shot_dir {
                        let p = format!("{dir}/playing-fullscreen.png");
                        shots3.borrow_mut().push(shot(&root3, &p));
                    }
                }
                go.set(n + 1);
                fs3.set(n + 1);
                glib::ControlFlow::Continue
            });
        }
        let toggles_end = TOGGLE_START + (CYCLES as u64 * 2 * 750) / 1000 + 1;
        if pt == toggles_end {
            marks2.borrow_mut().insert("rss_kb_after_toggles".into(), json!(rss_kb()));
        }
        if pt < cli2.duration {
            return glib::ControlFlow::Continue;
        }
        finish(&cli2, &app2, &samples2.borrow(), &marks2.borrow(), &shots2.borrow(), fs2.get(), inj2.get(), &page2, started, toggles_end);
        glib::ControlFlow::Break
    });
}

#[allow(clippy::too_many_arguments)]
fn finish(
    cli: &Cli,
    app: &App,
    samples: &[Value],
    marks: &serde_json::Map<String, Value>,
    shots: &[String],
    toggles: u32,
    injected: u32,
    page: &PageState,
    started: std::time::Instant,
    toggles_end: u64,
) {
    let last = samples.last().cloned().unwrap_or(Value::Null);
    let g = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
    let frames = g(&last, "frames");
    let drops = g(&last, "frame_drop");
    let dec = g(&last, "decoder_drop");
    let pct = |d: i64, f: i64| if f > 0 { (d as f64 / f as f64 * 10000.0).round() / 100.0 } else { -1.0 };
    let cpu: Vec<f64> = samples.iter().skip(1).filter_map(|s| s.get("cpu_pct")?.as_f64()).collect();
    let pre: Vec<f64> = samples.iter().skip(1).take(4).filter_map(|s| s.get("cpu_pct")?.as_f64()).collect();
    let avg = |v: &[f64]| if v.is_empty() { -1.0 } else { (v.iter().sum::<f64>() / v.len() as f64 * 10.0).round() / 10.0 };
    let rss_at = |t: u64| samples.iter().find(|s| g(s, "t") as u64 == t).map(|s| g(s, "rss_kb")).unwrap_or(0);
    let rss_end = g(&last, "rss_kb");
    let hw: Vec<String> = samples.iter().filter_map(|s| s.get("hwdec")?.as_str().map(String::from)).collect();
    let summary = json!({
        "plan": if cli.plan_b { "B" } else { "A" },
        "no_overlay": cli.no_overlay,
        "gdk_backend": gdk::Display::default().map(|d| d.type_().name().to_string()),
        "env": {
            "GDK_BACKEND": std::env::var("GDK_BACKEND").ok(),
            "WEBKIT_DISABLE_DMABUF_RENDERER": std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok(),
            "WEBKIT_DISABLE_COMPOSITING_MODE": std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").ok(),
        },
        "wall_seconds": started.elapsed().as_secs_f64(),
        "playback_seconds": samples.len(),
        "frames": frames,
        "frame_drop": drops,
        "decoder_frame_drop": dec,
        "drop_pct_vo": pct(drops, frames),
        "drop_pct_vo_plus_decoder": pct(drops + dec, frames),
        "hwdec_current": hw.last(),
        "cpu_pct_avg_all": avg(&cpu),
        "cpu_pct_avg_first4s_windowed": avg(&pre),
        "rss_kb_t5": rss_at(5),
        "rss_kb_end": rss_end,
        "rss_drift_kb_t5_to_end": rss_end - rss_at(5),
        "fullscreen_toggles_done": toggles,
        "toggles_end_t": toggles_end,
        "gl_errors": app.gl_errors.get(),
        "gl_error_codes": *app.gl_error_codes.borrow(),
        "renders": app.renders.get(),
        "synthetic_keys_injected": injected,
        "page_state_final": page.0.lock().unwrap().clone(),
        "marks": marks,
        "screenshots": shots,
        "samples": samples,
    });
    let text = serde_json::to_string_pretty(&summary).unwrap();
    if let Some(p) = &cli.out {
        let _ = std::fs::write(p, &text);
    }
    println!("{text}");
    std::process::exit(0);
}
