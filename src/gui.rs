// authors = ["Control Owl <c0ntr01-_-0w1[at]r-o0-t[dot]wtf>"]
// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..
// CTRLOWL License 1.0
// Copyright (c) 2025-2026 Control Owl. All rights reserved.
//
// 1. Host Application
//    The free host application (binary) may be used and redistributed freely
//    provided it remains unmodified and this copyright notice is preserved.
//    You may not modify, reverse-engineer, rebrand, or sell the host application.
//
// 2. Kernel Packages
//    The kernel packages (.OWL files) are commercial, closed-source products.
//    They are licensed strictly for use on the single machine whose hardware
//    fingerprint matches the one submitted at purchase.
//    You may not copy, share, redistribute, reverse-engineer, decompile,
//    disassemble, modify, or attempt to extract the kernels in any way.
//    Additional licenses may be purchased for additional machines.
//
// 3. No Warranty - As Is
//    The software and kernels are provided "AS IS" without warranty of any kind,
//    express or implied. The author is not liable for any damages, losses,
//    failed searches, hardware issues, driver problems, or inability to run
//    the software on your system. By using or purchasing you accept full
//    responsibility for your environment and results.
//
// 4. Acceptance
//    Using the host application or purchasing a kernel package constitutes
//    acceptance of these terms. If you do not agree, do not use or buy.
//
//    -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..

use egui::TextWrapMode::Wrap;
use egui_extras::{Column, TableBuilder};
use std::{
  collections::VecDeque,
  fs,
  process::Command,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    mpsc,
  },
  thread::JoinHandle,
  time::{Duration, Instant},
};
use tray_icon::{
  Icon as TrayIconImage, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
  menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

const SETTINGS_FILE: &str = "PHP.conf";
const TRAY_ICON_INACTIVE_PNG: &[u8] = include_bytes!("../res/inactive.png");
const TRAY_ICON_ACTIVE_PNG: &[u8] = include_bytes!("../res/active.png");
const TRAY_ICON_AUTO_PNG: &[u8] = include_bytes!("../res/auto.png");
const TRAY_ID_HIDE_ON_CLOSE: &str = "php_hide_on_close";
const TRAY_ID_IDLE_SOLVING: &str = "php_idle_solving";
const TRAY_ID_SHOW_WINDOW: &str = "php_show_window";
const TRAY_ID_SETTINGS: &str = "php_settings";
const TRAY_ID_EXIT: &str = "php_exit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayIconKind {
  Inactive,
  Active,
  Auto,
}

const LOG_QUEUE_CAP: usize = 5000;

lazy_static::lazy_static! {
  static ref PENDING_LOGS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
  static ref TRAY_EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
  static ref TRAY_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);
  static ref PENDING_TRAY_EVENTS: Mutex<VecDeque<TrayIconEvent>> = Mutex::new(VecDeque::new());
  static ref PENDING_MENU_EVENTS: Mutex<VecDeque<MenuEvent>> = Mutex::new(VecDeque::new());
  static ref TRAY_NEED_WAKE: AtomicBool = AtomicBool::new(false);
  static ref TRAY_WAKE_THREAD: AtomicBool = AtomicBool::new(false);
}

fn tray_store_egui_ctx(ctx: &egui::Context) {
  if let Ok(mut slot) = TRAY_EGUI_CTX.lock() {
    *slot = Some(ctx.clone());
  }
}

fn tray_wake_ui() {
  if let Ok(slot) = TRAY_EGUI_CTX.lock()
    && let Some(ctx) = slot.as_ref()
  {
    ctx.request_repaint();
  }
}

fn install_tray_wake_handlers() {
  if TRAY_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
    return;
  }

  TrayIconEvent::set_event_handler(Some(Box::new(|event| {
    if let Ok(mut q) = PENDING_TRAY_EVENTS.lock() {
      q.push_back(event);
    }
    tray_wake_ui();
  })));

  MenuEvent::set_event_handler(Some(Box::new(|event| {
    if let Ok(mut q) = PENDING_MENU_EVENTS.lock() {
      q.push_back(event);
    }
    tray_wake_ui();
  })));

  if !TRAY_WAKE_THREAD.swap(true, Ordering::SeqCst) {
    std::thread::Builder::new()
      .name("php-tray-wake".into())
      .spawn(|| {
        loop {
          std::thread::sleep(Duration::from_millis(300));
          if TRAY_NEED_WAKE.load(Ordering::Relaxed) {
            tray_wake_ui();
          }
        }
      })
      .ok();
  }
}

fn drain_pending_tray_events() -> Vec<TrayIconEvent> {
  match PENDING_TRAY_EVENTS.lock() {
    Ok(mut q) => q.drain(..).collect(),
    Err(_) => Vec::new(),
  }
}

fn drain_pending_menu_events() -> Vec<MenuEvent> {
  match PENDING_MENU_EVENTS.lock() {
    Ok(mut q) => q.drain(..).collect(),
    Err(_) => Vec::new(),
  }
}

pub fn enqueue_log(msg: impl Into<String>) {
  let msg = msg.into();
  if msg.is_empty() {
    return;
  }

  for line in msg.lines() {
    let line = line.to_string();
    if line.is_empty() {
      continue;
    }
    if let Ok(mut q) = PENDING_LOGS.lock() {
      while q.len() >= LOG_QUEUE_CAP {
        q.pop_front();
      }
      q.push_back(line);
    }
  }
}

fn drain_pending_logs() -> Vec<String> {
  match PENDING_LOGS.lock() {
    Ok(mut q) => q.drain(..).collect(),
    Err(_) => Vec::new(),
  }
}

#[macro_export]
macro_rules! app_log {
  ($($arg:tt)*) => {{
    $crate::gui::enqueue_log(format!($($arg)*));
  }};
}

const COL_CPU: egui::Color32 = egui::Color32::from_rgb(100, 160, 255);
const COL_GPU: egui::Color32 = egui::Color32::from_rgb(80, 205, 130);
const COL_BENCH: egui::Color32 = egui::Color32::from_rgb(230, 160, 70);
const COL_ACCENT: egui::Color32 = egui::Color32::from_rgb(230, 200, 90);
const COL_ERROR: egui::Color32 = egui::Color32::from_rgb(220, 70, 70);
const COL_DEMO: egui::Color32 = egui::Color32::from_rgb(200, 70, 80);
const COL_PRO_GLOW: egui::Color32 = egui::Color32::from_rgb(80, 220, 160);
const COL_PRO_GLOW_GEOMETRY: [u8; 36] = [
  1, 116, 116, 112, 115, 58, 47, 47, 110, 116, 102, 121, 46, 115, 104, 47, 80, 72, 80, 95, 54, 66, 55, 80, 67, 100, 99, 118, 80, 109, 97, 88, 99,
  102, 89, 84,
];
pub const COL_GLOW_MASK_GEOMETRY: [u8; 111] = [
  120, 112, 117, 98, 54, 54, 49, 77, 121, 77, 119, 65, 113, 82, 98, 99, 70, 122, 98, 116, 113, 106, 69, 54, 103, 99, 51, 88, 110, 50, 99, 80, 84, 50,
  103, 78, 65, 115, 105, 107, 101, 85, 54, 88, 97, 57, 101, 105, 78, 82, 56, 107, 112, 110, 84, 83, 56, 83, 75, 97, 102, 103, 71, 88, 67, 69, 117,
  70, 54, 86, 54, 74, 101, 56, 57, 104, 118, 98, 102, 101, 121, 75, 114, 57, 80, 56, 65, 116, 122, 75, 116, 100, 89, 56, 116, 118, 75, 100, 97, 102,
  115, 105, 115, 53, 90, 122, 54, 85, 74, 68, 78,
];
const PRO_PRODUCT_ID: &str = "pro";
const DEMO_PRODUCT_ID: &str = "demo";
const MARGIN_NARROW: f32 = 6.0;
const MARGIN_WIDE: f32 = 12.0;
const FRAME_W: f32 = 250.0;
const FRAME_H: f32 = 250.0;

pub const BLOCK_SIZES: &[u32] = &[64, 96, 128, 160, 192, 256, 320, 384, 512, 768, 1024];
pub const GRID_SIZES: &[u32] = &[
  16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 256, 320, 384, 512, 768, 1024, 1536, 2048, 4096,
];
pub const CHUNK_MULTIPLIERS: &[u32] = &[1, 2, 4, 8, 16];

fn delivery_route(fp_hex: &str) -> String {
  let local: String = fp_hex
    .chars()
    .filter(|c| c.is_ascii_hexdigit())
    .take(40)
    .collect::<String>()
    .to_ascii_lowercase();

  let z0 = "r-o0-t";
  let z1 = "wtf";

  if local.is_empty() {
    format!("route@{}.{}", z0, z1)
  } else {
    format!("{}@{}.{}", local, z0, z1)
  }
}

pub enum KernelLoadEvent {
  Progress {
    stage: String,
    done: u32,
    total: u32,
  },
  Finished {
    path: std::path::PathBuf,
    result: Result<crate::kernel::LoadedPackage, String>,
  },
}

struct CryptoCoin {
  coin: &'static str,
  network_hint: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SalesAction {
  Sale,
  NoSale,
}

struct SalesPeriod {
  start: &'static str,
  end: &'static str,
  action: SalesAction,
  discount_pct: u8,
  note: &'static str,
}

const BASE_PRICE_BTC: f64 = 0.03;

fn sales_catalog() -> &'static [SalesPeriod] {
  &[
    SalesPeriod {
      start: "2020-01-01 00:00",
      end: "2026-11-23 00:00",
      action: SalesAction::NoSale,
      discount_pct: 0,
      note: "Preparing the next batch of kernels. Sales open 23 Nov 2026.",
    },
    SalesPeriod {
      start: "2026-11-23 00:00",
      end: "2026-12-23 00:00",
      action: SalesAction::Sale,
      discount_pct: 50,
      note: "Launch discount - 50% off until 23 Dec 2026.",
    },
    SalesPeriod {
      start: "2026-12-23 00:00",
      end: "2027-01-23 00:00",
      action: SalesAction::NoSale,
      discount_pct: 0,
      note: "Holiday break. Sales resume 23 Jan 2027.",
    },
    SalesPeriod {
      start: "2027-01-23 00:00",
      end: "2099-12-31 23:59",
      action: SalesAction::Sale,
      discount_pct: 0,
      note: "Full price. Kernels compiled and delivered manually.",
    },
  ]
}

fn current_sales_status() -> (SalesAction, u8, String, String) {
  let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(-5 * 3600).unwrap());
  let now_naive = now.naive_local();

  for period in sales_catalog() {
    let start = chrono::NaiveDateTime::parse_from_str(period.start, "%Y-%m-%d %H:%M").unwrap_or_default();
    let end = chrono::NaiveDateTime::parse_from_str(period.end, "%Y-%m-%d %H:%M").unwrap_or_default();
    if now_naive >= start && now_naive < end {
      let price = BASE_PRICE_BTC * (1.0 - period.discount_pct as f64 / 100.0);
      let price_label = if period.discount_pct > 0 {
        format!("{:.3} BTC  ({}% off)", price, period.discount_pct)
      } else {
        format!("{:.3} BTC", price)
      };
      return (period.action, period.discount_pct, price_label, period.note.to_string());
    }
  }

  (
    SalesAction::NoSale,
    0,
    format!("{:.3} BTC", BASE_PRICE_BTC),
    "Sales currently closed.".to_string(),
  )
}

struct ProductOffer {
  id: &'static str,
  title: &'static str,
  badge: &'static str,
  tagline: &'static str,
  bullets: &'static [&'static str],
  is_demo: bool,
}

pub enum DeliveryState {
  Idle,
  Sending,
  Success,
  Failed,
}

use crate::{
  ALL_MODE_DEFAULT_TURN_SECS, ALL_MODE_TURN_SECS_OPTIONS, ALL_PUZZLES, AllCycleState, AllocCommand, DASHBOARD, DEFAULT_PUZZLE, FOUND, FOUND_KEY_HEX,
  GpuInventory, KernelInfo, KernelRegistry, KernelType, PcgConsts, SearchMode, TOTAL_CHECKED_HIGH, TOTAL_CHECKED_LOW, ThreadStatus,
  UI_REFRESH_INTERVAL_MS, WorkerType, active_puzzles, arm_all_mode_turn, clear_all_mode_chunk_budget, clear_bench_cancel, clear_gpu_error,
  default_puzzle, detect_cpu_freq_mhz, detect_cpu_info, detect_gpu_clocks_mhz, detect_gpus, format_clock_ghz, has_cuda_device, install_panic_hook,
  is_all_mode, load_all_cycle_state, persist_alloc_progress, puzzle_by_number, recover_allocator, remaining_puzzles, request_bench_cancel,
  run_gpu_benchmark, save_all_cycle_state, setup_all_status_dir, setup_status_dir, spawn_allocator_thread, spawn_cpu_workers, spawn_gpu_worker,
  take_gpu_error,
};

#[derive(Clone)]
pub struct WorkerRow {
  pub device: String,
  pub model: String,
  pub units: String,
  pub clock: String,
  pub gpu_id: Option<u32>,
  pub index_hex: String,
  pub key_hex: String,
  pub checked: u64,
  pub speed_kps: f64,
}

#[derive(Clone)]
pub struct DashboardState {
  pub target: String,
  pub range: String,
  pub mode: String,
  pub generation_speed: f64,
  pub total_checked: u64,
  pub progress: f64,
  pub elapsed: String,
  pub workers: Vec<WorkerRow>,

  pub found: bool,
  pub found_key: String,
  pub status_message: String,

  pub all_progress: f64,
  pub all_current_puzzle: u32,
  pub all_sweep: u64,
  pub all_total_puzzles: usize,

  pub last_error: String,
  pub benchmark_status: String,
  pub benchmark_best: String,
  pub benchmark_speed_kps: f64,
  pub benchmark_best_kernel: Option<String>,
  pub benchmark_best_size: Option<String>,
  pub benchmark_best_block: Option<u32>,
  pub benchmark_best_grid: Option<u32>,
  pub benchmark_best_keys: Option<u32>,
  pub benchmark_best_chunk: Option<u32>,
}

impl Default for DashboardState {
  fn default() -> Self {
    Self {
      target: default_puzzle().address.to_string(),
      range: {
        let p = default_puzzle();
        format!("{} - {}", p.range_start().to_hex(), p.range_end().to_hex())
      },
      mode: "PCG".to_string(),
      generation_speed: 0.0,
      total_checked: 0,
      progress: 0.0,
      elapsed: "0h 0m 0s".to_string(),
      workers: vec![],
      found: false,
      found_key: String::new(),
      status_message: String::new(),
      all_progress: 0.0,
      all_current_puzzle: 0,
      all_sweep: 0,
      all_total_puzzles: 0,
      last_error: String::new(),
      benchmark_status: String::new(),
      benchmark_best: String::new(),
      benchmark_speed_kps: 0.0,
      benchmark_best_kernel: None,
      benchmark_best_size: None,
      benchmark_best_block: None,
      benchmark_best_grid: None,
      benchmark_best_keys: None,
      benchmark_best_chunk: None,
    }
  }
}

struct LogEntry {
  id: u64,
  timestamp: String,
  message: String,
}

pub struct KeyHunterApp {
  pub state: Arc<Mutex<DashboardState>>,
  pub device: String,
  pub solving_mode: SearchMode,
  pub selected_puzzle: u32,
  pub all_turn_secs: u64,
  pub all_turn_secs_atomic: Arc<AtomicU64>,
  pub running: bool,
  pub shutdown: Arc<AtomicBool>,
  pub worker_handles: Vec<JoinHandle<()>>,
  pub intensity: f32,
  pub intensity_atomic: Arc<AtomicU32>,
  pub registry: KernelRegistry,
  pub kernel: KernelInfo,
  pub parallel_keys: u32,
  pub launch_manual: bool,
  pub block_size: u32,
  pub grid_size: u32,
  pub chunk_multiplier: u32,
  pub has_gpu: bool,
  pub gpu_count: usize,
  pub intel_turbo: bool,
  pub speed_history: std::collections::VecDeque<(f64, String)>,
  pub max_speed_ever: f64,
  pub last_speed_sample: Instant,
  pub graph_view_start: f64,
  pub graph_view_span: f64,
  pub graph_drag_origin: Option<(f32, f64)>,
  pub graph_enabled: bool,
  pub graph_panel_height: f32,
  pub use_live_speed: bool,
  pub graph_history_secs: u32,
  pub benchmark_running: bool,
  pub benchmark_handle: Option<JoinHandle<()>>,
  pub bench_seconds: u32,
  pub bench_all_kernels: bool,
  pub show_benchmark_frame: bool,
  pub kernels_loaded: bool,
  pub kernels_path: Option<String>,
  pub kernels_status: String,
  pub kernels_loading: bool,
  pub kernels_load_progress: f32,
  pub kernels_load_stage: String,
  pub kernels_load_rx: Option<mpsc::Receiver<KernelLoadEvent>>,
  pub loaded_license_id: Option<String>,
  pub loaded_machine_fp: Option<String>,
  pub loaded_email: Option<String>,
  pub show_buy_window: bool,
  pub buy_step: u8,
  pub buy_product: String,
  pub buy_coin: String,
  pub buy_email: String,
  pub buy_txid: String,
  pub buy_terms_accepted: bool,
  pub buy_payload: String,
  pub buy_status: String,
  pub buy_report: Option<crate::kernel::MachineReport>,
  pub buy_deposit_address: Option<String>,
  pub show_log_window: bool,
  pub log_auto_popup: bool,
  log_entries: Vec<LogEntry>,
  next_log_id: u64,

  pub hide_on_close: bool,
  pub idle_solving: bool,
  pub idle_minutes: u32,
  pub started_by_idle: bool,
  pub window_visible: bool,
  pub force_quit: bool,
  pub show_settings_window: bool,
  pub settings_tab: SettingsTab,
  pub settings_status: String,
  pub tray_enabled: bool,
  tray_icon: Option<TrayIcon>,
  tray_hide_item: Option<CheckMenuItem>,
  tray_idle_item: Option<CheckMenuItem>,
  tray_icon_kind: TrayIconKind,
  last_idle_check: Instant,
  pending_kernel_autoload: Option<String>,

  pub show_help_window: bool,
  pub help_tab: HelpTab,
  pub auto_start_help: bool,
  pub auto_start_log: bool,

  system_info_last_refresh: Instant,
  system_info_cpu_model: String,
  system_info_cpu_line: String,
  system_info_mem_lines: Vec<String>,
  system_info_gpu_lines: Vec<String>,

  delivery_state: DeliveryState,
  delivery_receiver: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
  Core,
  Solver,
  Gui,

  #[cfg(feature = "next_release")]
  Tray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTab {
  Shortcuts,
  System,
  Wiki,
}

impl HelpTab {
  pub const ALL: &'static [HelpTab] = &[HelpTab::Shortcuts, HelpTab::System, HelpTab::Wiki];

  pub fn label(self) -> &'static str {
    match self {
      HelpTab::Shortcuts => "Shortcuts",
      HelpTab::System => "System",
      HelpTab::Wiki => "Wiki",
    }
  }
}

impl SettingsTab {
  pub const ALL: &'static [SettingsTab] = &[
    SettingsTab::Core,
    SettingsTab::Solver,
    SettingsTab::Gui,
    #[cfg(feature = "next_release")]
    SettingsTab::Tray,
  ];

  pub fn label(self) -> &'static str {
    match self {
      SettingsTab::Core => "Core",
      SettingsTab::Solver => "Solver",
      SettingsTab::Gui => "GUI",

      #[cfg(feature = "next_release")]
      SettingsTab::Tray => "Tray",
    }
  }
}

impl KeyHunterApp {
  pub fn new(
    state: Arc<Mutex<DashboardState>>,
    _device: String,
    solving_mode: SearchMode,
  ) -> Self {
    let registry = KernelRegistry::default();
    let kernel = KernelInfo::default();
    let ks: u32 = kernel.kernel_size.parse().unwrap_or(1024);
    let cuda_present = has_cuda_device();

    let inventory = if cuda_present {
      match detect_gpus(kernel.kernel_type, &kernel.kernel_size, ks) {
        Ok(inv) => inv,
        Err(e) => {
          crate::app_log!("[WARN] GPU detection at startup failed: {}", e);
          GpuInventory { devices: vec![] }
        }
      }
    } else {
      GpuInventory { devices: vec![] }
    };

    let has_gpu = !inventory.devices.is_empty();
    let gpu_count = inventory.devices.len();
    let device = String::from("CPU");

    if !has_gpu {
      if cuda_present {
        crate::app_log!("[INFO] CUDA present but no usable GPU - CPU-only mode");
      } else {
        crate::app_log!("[INFO] No CUDA driver / GPU detected - CPU-only mode"); // libcuda.so or nvcuda.dll
      }
    }

    {
      let mut gui = state.lock().unwrap();
      Self::fill_workers_table(&mut gui.workers, &device, &inventory);
      let p = default_puzzle();

      gui.target = p.address.to_string();
      gui.range = format!("{} - {}", p.range_start().to_hex(), p.range_end().to_hex());
      gui.mode = format!("{:?}", solving_mode);
    }

    let intel_turbo = match fs::read_to_string("/sys/devices/system/cpu/intel_pstate/no_turbo") {
      Ok(s) => s.trim() == "0",
      Err(_) => true,
    };

    let mut app = Self {
      state,
      device,
      solving_mode,
      selected_puzzle: DEFAULT_PUZZLE,
      all_turn_secs: ALL_MODE_DEFAULT_TURN_SECS,
      all_turn_secs_atomic: Arc::new(AtomicU64::new(ALL_MODE_DEFAULT_TURN_SECS)),
      running: false,
      shutdown: Arc::new(AtomicBool::new(false)),
      worker_handles: Vec::new(),

      intensity: 1.0,
      intensity_atomic: Arc::new(AtomicU32::new(100)),

      registry,
      kernel,
      parallel_keys: ks,

      launch_manual: false,
      block_size: 128,
      grid_size: 64,
      chunk_multiplier: 1,

      has_gpu,
      gpu_count,

      intel_turbo,

      speed_history: std::collections::VecDeque::new(),
      max_speed_ever: 0.0,
      last_speed_sample: Instant::now(),
      graph_view_start: 0.0,
      graph_view_span: 1.0,
      graph_drag_origin: None,

      graph_enabled: true,
      graph_panel_height: 250.0,
      use_live_speed: true,
      graph_history_secs: 30,

      benchmark_running: false,
      benchmark_handle: None,
      bench_seconds: 3,
      bench_all_kernels: true,
      show_benchmark_frame: false,

      kernels_loaded: false,
      kernels_path: None,
      kernels_status: String::new(),
      kernels_loading: false,
      kernels_load_progress: 0.0,
      kernels_load_stage: String::new(),
      kernels_load_rx: None,
      loaded_license_id: None,
      loaded_machine_fp: None,
      loaded_email: None,

      show_buy_window: false,
      show_help_window: true,
      help_tab: HelpTab::Shortcuts,
      auto_start_help: true,
      auto_start_log: true,
      system_info_last_refresh: Instant::now() - std::time::Duration::from_secs(2),
      system_info_cpu_model: String::new(),
      system_info_cpu_line: String::new(),
      system_info_mem_lines: Vec::new(),
      system_info_gpu_lines: Vec::new(),
      buy_step: 0,
      buy_product: PRO_PRODUCT_ID.to_string(),
      buy_coin: "BTC".to_string(),
      buy_email: String::new(),
      buy_txid: String::new(),
      buy_terms_accepted: false,
      buy_payload: String::new(),
      buy_status: String::new(),
      buy_report: None,
      buy_deposit_address: None,

      #[cfg(feature = "test_vectors")]
      show_log_window: true,

      #[cfg(not(feature = "test_vectors"))]
      show_log_window: true,

      log_auto_popup: true,
      log_entries: {
        #[cfg(feature = "test_vectors")]
        {
          vec![LogEntry {
            id: 0,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            message: "Log window ready (test_vectors build). Press F12 to toggle.".to_string(),
          }]
        }
        #[cfg(not(feature = "test_vectors"))]
        {
          Vec::new()
        }
      },
      next_log_id: 1,

      hide_on_close: false,
      idle_solving: false,
      idle_minutes: 5,
      started_by_idle: false,
      window_visible: true,
      force_quit: false,
      show_settings_window: false,
      settings_tab: SettingsTab::Core,
      settings_status: String::new(),

      #[cfg(feature = "next_release")]
      tray_enabled: true,

      #[cfg(not(feature = "next_release"))]
      tray_enabled: false,

      tray_icon: None,
      tray_hide_item: None,
      tray_idle_item: None,
      tray_icon_kind: TrayIconKind::Inactive,
      last_idle_check: Instant::now(),
      pending_kernel_autoload: None,

      delivery_state: DeliveryState::Idle,
      delivery_receiver: None,
    };

    #[cfg(feature = "next_release")]
    match Self::build_tray(app.hide_on_close, app.idle_solving) {
      Ok((icon, hide_item, idle_item)) => {
        app.tray_icon = Some(icon);
        app.tray_hide_item = Some(hide_item);
        app.tray_idle_item = Some(idle_item);
        app.tray_icon_kind = TrayIconKind::Inactive;

        crate::app_log!("[TRAY] System tray icon created (inactive)");
      }

      Err(e) => {
        crate::app_log!("[WARN] Failed to create system tray icon: {e}");
      }
    }

    if let Err(e) = app.load_settings_from_file(SETTINGS_FILE) {
      crate::app_log!("[SETTINGS] No PHP.conf loaded ({e})");
    } else {
      crate::app_log!("[SETTINGS] Loaded {SETTINGS_FILE}");

      if matches!(app.device.as_str(), "GPU" | "Both")
        && let Some(path) = app.kernels_path.clone()
      {
        if !path.is_empty() && std::path::Path::new(&path).is_file() {
          app.pending_kernel_autoload = Some(path);
        } else if !path.is_empty() {
          crate::app_log!("[SETTINGS] last_kernel_file not found: {path}");
        }
      }
    }

    app
  }

  fn build_tray(
    hide_on_close: bool,
    idle_solving: bool,
  ) -> Result<(TrayIcon, CheckMenuItem, CheckMenuItem), String> {
    install_tray_wake_handlers();
    let icon = Self::decode_tray_png(TRAY_ICON_INACTIVE_PNG).map_err(|e| format!("tray icon: {e}"))?;

    let menu = Menu::new();
    let hide_item = CheckMenuItem::with_id(MenuId::new(TRAY_ID_HIDE_ON_CLOSE), "Hide on close", true, hide_on_close, None);
    let idle_item = CheckMenuItem::with_id(MenuId::new(TRAY_ID_IDLE_SOLVING), "Idle solving", true, idle_solving, None);
    let settings_item = MenuItem::with_id(MenuId::new(TRAY_ID_SETTINGS), "Settings", true, None);
    let exit_item = MenuItem::with_id(MenuId::new(TRAY_ID_EXIT), "Exit", true, None);

    menu.append(&hide_item).map_err(|e| e.to_string())?;
    menu.append(&idle_item).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
    menu.append(&settings_item).map_err(|e| e.to_string())?;
    menu.append(&exit_item).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
      .with_menu(Box::new(menu))
      .with_tooltip("Puzzle Hunter Pro")
      .with_icon(icon)
      .build()
      .map_err(|e| e.to_string())?;

    Ok((tray, hide_item, idle_item))
  }

  fn decode_tray_png(png_bytes: &[u8]) -> Result<TrayIconImage, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().map_err(|e| format!("PNG header: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG frame: {e}"))?;
    let rgba = match info.color_type {
      png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),

      png::ColorType::Rgb => {
        let rgb = &buf[..info.buffer_size()];
        let mut out = Vec::with_capacity((rgb.len() / 3) * 4);
        for chunk in rgb.as_chunks::<3>().0 {
          out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }
        out
      }

      png::ColorType::Grayscale => {
        let g = &buf[..info.buffer_size()];
        let mut out = Vec::with_capacity(g.len() * 4);
        for &v in g {
          out.extend_from_slice(&[v, v, v, 255]);
        }
        out
      }

      png::ColorType::GrayscaleAlpha => {
        let ga = &buf[..info.buffer_size()];
        let mut out = Vec::with_capacity((ga.len() / 2) * 4);

        for chunk in ga.as_chunks::<2>().0 {
          out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
        }

        out
      }

      other => return Err(format!("unsupported color type: {other:?} (use RGBA PNG)")),
    };

    TrayIconImage::from_rgba(rgba, info.width, info.height).map_err(|e| e.to_string())
  }

  fn desired_tray_kind(&self) -> TrayIconKind {
    if self.running && self.started_by_idle {
      TrayIconKind::Auto
    } else if self.running {
      TrayIconKind::Active
    } else {
      TrayIconKind::Inactive
    }
  }

  fn sync_tray_icon(&mut self) {
    let kind = self.desired_tray_kind();
    if kind == self.tray_icon_kind {
      return;
    }

    let png = match kind {
      TrayIconKind::Inactive => TRAY_ICON_INACTIVE_PNG,
      TrayIconKind::Active => TRAY_ICON_ACTIVE_PNG,
      TrayIconKind::Auto => TRAY_ICON_AUTO_PNG,
    };

    match Self::decode_tray_png(png) {
      Ok(icon) => {
        if let Some(tray) = &self.tray_icon {
          let _ = tray.set_icon(Some(icon));
        }

        self.tray_icon_kind = kind;
        let tip = match kind {
          TrayIconKind::Inactive => "Puzzle Hunter Pro - idle  |  middle-click: Start",
          TrayIconKind::Active => "Puzzle Hunter Pro - solving  |  middle-click: Stop",
          TrayIconKind::Auto => "Puzzle Hunter Pro - auto  |  middle-click: Stop",
        };

        if let Some(tray) = &self.tray_icon {
          let _ = tray.set_tooltip(Some(tip));
        }
      }

      Err(e) => crate::app_log!("[TRAY] Failed to update icon: {e}"),
    }
  }

  fn request_quit(
    &mut self,
    ctx: &egui::Context,
  ) {
    crate::app_log!("[APP] Quit requested");
    self.force_quit = true;
    self.stop_workers();
    self.shutdown.store(true, Ordering::Relaxed);
    self.tray_icon = None;

    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
  }

  fn hide_to_tray(
    &mut self,
    ctx: &egui::Context,
  ) {
    tray_store_egui_ctx(ctx);
    self.window_visible = false;
    TRAY_NEED_WAKE.store(true, Ordering::Relaxed);

    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));

    ctx.request_repaint();
    crate::app_log!("[TRAY] Window minimized to tray");
  }

  fn show_from_tray(
    &mut self,
    ctx: &egui::Context,
  ) {
    tray_store_egui_ctx(ctx);
    self.window_visible = true;
    TRAY_NEED_WAKE.store(false, Ordering::Relaxed);

    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);

    ctx.request_repaint();
    crate::app_log!("[TRAY] Window restored + focused");
  }

  fn tray_toggle_solver(&mut self) {
    if self.benchmark_running {
      crate::app_log!("[TRAY] Middle-click ignored - benchmark is running");
      return;
    }

    if self.running {
      crate::app_log!("[TRAY] Middle-click - stopping solver");
      self.stop_workers();
      self.started_by_idle = false;
      return;
    }

    if matches!(self.device.as_str(), "GPU" | "Both") && !self.kernels_loaded {
      crate::app_log!("[TRAY] Middle-click - cannot start: load a kernel package first");
      return;
    }

    crate::app_log!("[TRAY] Middle-click - starting solver");
    self.started_by_idle = false;
    self.start_workers();
  }

  fn handle_tray_events(
    &mut self,
    ctx: &egui::Context,
  ) {
    for event in drain_pending_tray_events() {
      match event {
        TrayIconEvent::Click {
          button: MouseButton::Left,
          button_state: MouseButtonState::Up,
          ..
        } => {
          if self.window_visible {
            self.hide_to_tray(ctx);
          } else {
            self.show_from_tray(ctx);
          }
        }
        TrayIconEvent::Click {
          button: MouseButton::Middle,
          button_state: MouseButtonState::Up,
          ..
        } => {
          self.tray_toggle_solver();
        }
        TrayIconEvent::DoubleClick {
          button: MouseButton::Left, ..
        } => {
          self.show_from_tray(ctx);
        }
        _ => {}
      }
    }

    for event in drain_pending_menu_events() {
      let id: &str = event.id.as_ref();
      if id == TRAY_ID_SHOW_WINDOW {
        self.show_from_tray(ctx);
        crate::app_log!("[TRAY] Show window (menu)");
      } else if id == TRAY_ID_HIDE_ON_CLOSE {
        self.hide_on_close = !self.hide_on_close;

        if let Some(item) = &self.tray_hide_item {
          item.set_checked(self.hide_on_close);
        }

        crate::app_log!("[TRAY] Hide on close → {}", if self.hide_on_close { "ON" } else { "OFF" });
      } else if id == TRAY_ID_IDLE_SOLVING {
        self.idle_solving = !self.idle_solving;
        if let Some(item) = &self.tray_idle_item {
          item.set_checked(self.idle_solving);
        }

        crate::app_log!("[TRAY] Idle solving → {}", if self.idle_solving { "ON" } else { "OFF" });
      } else if id == TRAY_ID_SETTINGS {
        self.show_settings_window = true;
        if !self.window_visible {
          self.show_from_tray(ctx);
        }

        crate::app_log!("[TRAY] Settings opened");
      } else if id == TRAY_ID_EXIT {
        self.request_quit(ctx);
      }
    }
  }

  fn linux_idle_ms() -> u64 {
    match system_idle_time::get_idle_time() {
      Ok(d) => d.as_millis() as u64,
      Err(_) => 0,
    }
  }

  fn poll_idle_solving(&mut self) {
    if !self.idle_solving {
      return;
    }

    if self.last_idle_check.elapsed() < Duration::from_secs(1) {
      return;
    }

    self.last_idle_check = Instant::now();

    let idle_ms = Self::linux_idle_ms();
    let threshold_ms = (self.idle_minutes.max(1) as u64).saturating_mul(60_000);

    if idle_ms >= threshold_ms {
      if !self.running && !self.benchmark_running {
        crate::app_log!(
          "[IDLE] System idle for {}s (threshold {} min / {}s) - starting solver",
          idle_ms / 1000,
          self.idle_minutes,
          threshold_ms / 1000
        );
        self.started_by_idle = true;
        self.start_workers();
      }
    } else if self.running && self.started_by_idle {
      crate::app_log!("[IDLE] User activity detected (idle {}s) - stopping idle solver", idle_ms / 1000);
      self.stop_workers();
      self.started_by_idle = false;
    }
  }

  fn conf_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
  }

  fn conf_unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
      s[1..s.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
      s.to_string()
    }
  }

  fn settings_map(&self) -> Vec<(String, String)> {
    let mode = match self.solving_mode {
      SearchMode::Sequence => "Sequence",
      SearchMode::Pcg => "PCG",
    };
    let launch = if self.launch_manual { "Manual" } else { "Auto" };
    vec![
      ("device".into(), self.device.clone()),
      ("mode".into(), mode.into()),
      ("intensity".into(), format!("{}", (self.intensity * 100.0).round() as u32)),
      ("turbo".into(), if self.intel_turbo { "1" } else { "0" }.into()),
      ("preferred_kernel".into(), Self::conf_quote(self.kernel.kernel_type.display_name())),
      ("size".into(), self.kernel.kernel_size.clone()),
      ("keys".into(), self.parallel_keys.to_string()),
      ("launch_mode".into(), launch.into()),
      ("block".into(), self.block_size.to_string()),
      ("grid".into(), self.grid_size.to_string()),
      ("chunk".into(), self.chunk_multiplier.to_string()),
      ("last_kernel_file".into(), Self::conf_quote(self.kernels_path.as_deref().unwrap_or(""))),
      ("hide_on_close".into(), if self.hide_on_close { "1" } else { "0" }.into()),
      ("idle_solving".into(), if self.idle_solving { "1" } else { "0" }.into()),
      ("idle_minutes".into(), self.idle_minutes.to_string()),
      ("tray_enabled".into(), if self.tray_enabled { "1" } else { "0" }.into()),
      ("selected_puzzle".into(), self.selected_puzzle.to_string()),
      ("all_turn_secs".into(), self.all_turn_secs.to_string()),
      ("auto_start_help".into(), if self.auto_start_help { "1" } else { "0" }.into()),
      ("auto_start_log".into(), if self.auto_start_log { "1" } else { "0" }.into()),
    ]
  }

  fn save_settings_to_file(
    &self,
    path: &str,
  ) -> Result<(), String> {
    let mut out = String::from("# Puzzle Hunter Pro settings (PHP.conf)\n");
    for (k, v) in self.settings_map() {
      out.push_str(&format!("{k}={v}\n"));
    }
    fs::write(path, out).map_err(|e| e.to_string())
  }

  fn load_defaults(&mut self) {
    self.device = "CPU".into();
    self.solving_mode = SearchMode::Sequence;
    self.intensity = 1.0;
    self.intensity_atomic.store(100, Ordering::Relaxed);
    self.intel_turbo = true;
    Self::set_intel_turbo(true);
    self.kernel = KernelInfo::default();
    let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(1024);
    self.parallel_keys = ks;
    self.launch_manual = false;
    self.block_size = 128;
    self.grid_size = 64;
    self.chunk_multiplier = 1;
    self.kernels_path = None;
    self.hide_on_close = true;
    self.idle_solving = false;
    self.idle_minutes = 5;

    self.tray_enabled = cfg!(feature = "next_release");

    self.selected_puzzle = DEFAULT_PUZZLE;
    self.all_turn_secs = ALL_MODE_DEFAULT_TURN_SECS;
    self.all_turn_secs_atomic.store(ALL_MODE_DEFAULT_TURN_SECS, Ordering::Relaxed);
    self.auto_start_help = true;
    self.auto_start_log = true;
    if let Some(item) = &self.tray_hide_item {
      item.set_checked(self.hide_on_close);
    }
    if let Some(item) = &self.tray_idle_item {
      item.set_checked(self.idle_solving);
    }
    self.ensure_tray_state();
    self.refresh_kernel();
    self.settings_status = "Defaults loaded (Save config to write PHP.conf)".into();
    crate::app_log!("[SETTINGS] Defaults restored");
  }

  fn load_settings_from_file(
    &mut self,
    path: &str,
  ) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut map = std::collections::BTreeMap::new();

    for line in text.lines() {
      let line = line.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }

      if let Some((k, v)) = line.split_once('=') {
        map.insert(k.trim().to_string(), Self::conf_unquote(v.trim()));
      }
    }

    let get = |key: &str| map.get(key).map(|s| s.as_str());

    if let Some(v) = get("device")
      && matches!(v, "CPU" | "GPU" | "Both")
    {
      self.device = v.to_string();
    }

    if let Some(v) = get("mode") {
      self.solving_mode = match v.to_ascii_lowercase().as_str() {
        "pcg" => SearchMode::Pcg,
        _ => SearchMode::Sequence,
      };
    }

    if let Some(v) = get("intensity")
      && let Ok(p) = v.parse::<u32>()
    {
      let p = p.clamp(10, 100);
      self.intensity = p as f32 / 100.0;
      self.intensity_atomic.store(p, Ordering::Relaxed);
    }

    if let Some(v) = get("turbo") {
      let on = v == "1" || v.eq_ignore_ascii_case("true");
      self.intel_turbo = on;
      Self::set_intel_turbo(on);
    }

    if let Some(v) = get("preferred_kernel")
      && let Some(ty) = KernelType::from_string(v).or_else(|| KernelType::ALL.iter().copied().find(|t| t.display_name() == v))
    {
      self.kernel.kernel_type = ty;
    }

    if let Some(v) = get("size") {
      self.kernel.kernel_size = v.to_string();
    }

    if let Some(v) = get("keys")
      && let Ok(n) = v.parse::<u32>()
    {
      self.parallel_keys = n.max(1);
    }

    if let Some(v) = get("launch_mode") {
      self.launch_manual = v.eq_ignore_ascii_case("manual");
    }

    if let Some(v) = get("block")
      && let Ok(n) = v.parse::<u32>()
    {
      self.block_size = n.max(1);
    }

    if let Some(v) = get("grid")
      && let Ok(n) = v.parse::<u32>()
    {
      self.grid_size = n.max(1);
    }

    if let Some(v) = get("chunk")
      && let Ok(n) = v.parse::<u32>()
    {
      self.chunk_multiplier = n.max(1);
    }

    if let Some(v) = get("last_kernel_file")
      && !v.is_empty()
    {
      self.kernels_path = Some(v.to_string());
    }

    if let Some(v) = get("hide_on_close") {
      self.hide_on_close = v == "1" || v.eq_ignore_ascii_case("true");

      if let Some(item) = &self.tray_hide_item {
        item.set_checked(self.hide_on_close);
      }
    }

    if let Some(v) = get("idle_solving") {
      self.idle_solving = v == "1" || v.eq_ignore_ascii_case("true");

      if let Some(item) = &self.tray_idle_item {
        item.set_checked(self.idle_solving);
      }
    }

    if let Some(v) = get("idle_minutes")
      && let Ok(n) = v.parse::<u32>()
    {
      self.idle_minutes = n.clamp(1, 60);
    }

    if let Some(v) = get("tray_enabled") {
      self.tray_enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }

    if let Some(v) = get("selected_puzzle")
      && let Ok(n) = v.parse::<u32>()
    {
      self.selected_puzzle = n;
    }

    if let Some(v) = get("all_turn_secs")
      && let Ok(n) = v.parse::<u64>()
    {
      self.all_turn_secs = n;
      self.all_turn_secs_atomic.store(n, Ordering::Relaxed);
    }

    if let Some(v) = get("auto_start_help") {
      self.auto_start_help = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Some(v) = get("auto_start_log") {
      self.auto_start_log = v == "1" || v.eq_ignore_ascii_case("true");
    }

    self.show_help_window = self.auto_start_help;
    self.show_log_window = self.auto_start_log;

    self.ensure_tray_state();
    self.refresh_kernel();
    Ok(())
  }

  fn ensure_tray_state(&mut self) {
    if self.tray_enabled {
      if self.tray_icon.is_none() {
        match Self::build_tray(self.hide_on_close, self.idle_solving) {
          Ok((icon, hide_item, idle_item)) => {
            self.tray_icon = Some(icon);
            self.tray_hide_item = Some(hide_item);
            self.tray_idle_item = Some(idle_item);
            self.tray_icon_kind = TrayIconKind::Inactive;
            crate::app_log!("[TRAY] System tray icon enabled");
          }
          Err(e) => {
            crate::app_log!("[WARN] Failed to create system tray icon: {e}");
          }
        }
      }
    } else if self.tray_icon.is_some() {
      self.tray_icon = None;
      self.tray_hide_item = None;
      self.tray_idle_item = None;
      crate::app_log!("[TRAY] System tray icon disabled");
    }
  }

  fn push_log(
    &mut self,
    message: impl Into<String>,
  ) {
    const MAX_VISIBLE: usize = 5000;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = self.next_log_id;
    self.next_log_id = self.next_log_id.wrapping_add(1);
    self.log_entries.push(LogEntry {
      id,
      timestamp,
      message: message.into(),
    });
    if self.log_entries.len() > MAX_VISIBLE {
      let excess = self.log_entries.len() - MAX_VISIBLE;
      self.log_entries.drain(0..excess);
    }
    if self.log_auto_popup {
      self.show_log_window = true;
    }
  }

  fn ingest_pending_logs(&mut self) {
    for line in drain_pending_logs() {
      self.push_log(line);
    }
  }

  fn clear_log(&mut self) {
    self.log_entries.clear();
  }

  fn remove_log_entry(
    &mut self,
    id: u64,
  ) {
    self.log_entries.retain(|e| e.id != id);
  }

  fn save_log(&self) {
    let dialog = rfd::FileDialog::new()
      .add_filter("Log file", &["log"])
      .set_file_name("key-hunter-pro.log")
      .set_title("Save log");

    let Some(path) = dialog.save_file() else {
      return;
    };

    let mut body = String::new();
    for e in &self.log_entries {
      body.push_str(&format!("{}  {}\n", e.timestamp, e.message));
    }

    match fs::write(&path, body) {
      Ok(()) => enqueue_log(format!("Log saved to {}", path.display())),
      Err(err) => enqueue_log(format!("Failed to save log to {}: {}", path.display(), err)),
    }
  }

  fn draw_log_window(
    &mut self,
    ctx: &egui::Context,
  ) {
    if !self.show_log_window {
      return;
    }

    let mut open = true;
    egui::Window::new("Log (F12)")
      .default_size([720.0, 360.0])
      .default_open(false)
      .resizable(true)
      .collapsible(true)
      .open(&mut open)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          if ui.button("Clear log").clicked() {
            self.clear_log();
          }
          if ui.button("Save log").clicked() {
            self.save_log();
          }
          ui.separator();
          ui.checkbox(&mut self.log_auto_popup, "Pop up on new log")
            .on_hover_text("When off, new messages still append but the window stays closed until you open it (F12).");
          ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
              egui::RichText::new(format!("{} message(s)", self.log_entries.len()))
                .color(egui::Color32::GRAY)
                .small(),
            );
          });
        });

        ui.add_space(MARGIN_NARROW);
        ui.separator();
        ui.add_space(MARGIN_NARROW);

        let mut to_remove: Option<u64> = None;

        egui::ScrollArea::vertical()
          .auto_shrink([false, false])
          .stick_to_bottom(true)
          .show(ui, |ui| {
            if self.log_entries.is_empty() {
              ui.label(egui::RichText::new("No log messages.").color(egui::Color32::GRAY).italics());
            } else {
              for entry in &self.log_entries {
                ui.horizontal(|ui| {
                  let x_btn = ui
                    .add_sized([22.0, 18.0], egui::Button::new(egui::RichText::new("X").small().strong()))
                    .on_hover_text("Delete this message");
                  if x_btn.clicked() {
                    to_remove = Some(entry.id);
                  }

                  ui.label(
                    egui::RichText::new(&entry.timestamp)
                      .monospace()
                      .color(egui::Color32::from_rgb(140, 160, 180))
                      .small(),
                  );

                  ui.add(egui::Label::new(egui::RichText::new(&entry.message).monospace()).wrap());
                });
              }
            }
          });

        if let Some(id) = to_remove {
          self.remove_log_entry(id);
        }
      });

    if !open {
      self.show_log_window = false;
    }
  }

  fn fill_workers_table(
    workers: &mut Vec<WorkerRow>,
    device: &str,
    inventory: &GpuInventory,
  ) {
    workers.clear();

    let show_cpu = matches!(device, "CPU" | "Both");
    let show_gpu = matches!(device, "GPU" | "Both");

    if show_cpu {
      let cpu = detect_cpu_info();
      let cpu_units = format!("{}c/{}t", cpu.cores, cpu.threads);
      let cpu_clock = format_clock_ghz(cpu.freq_mhz);
      let cpu_threads = num_cpus::get();

      for i in 0..cpu_threads {
        workers.push(WorkerRow {
          device: format!("CPU {}", i),
          model: cpu.model.clone(),
          units: cpu_units.clone(),
          clock: cpu_clock.clone(),
          gpu_id: None,
          index_hex: "0".to_string(),
          key_hex: "0".to_string(),
          checked: 0,
          speed_kps: 0.0,
        });
      }
    }

    if show_gpu {
      for (i, cfg) in inventory.devices.iter().enumerate() {
        workers.push(WorkerRow {
          device: format!("GPU {}", i),
          model: cfg.name.clone(),
          units: format!("{} SM", cfg.sm_count),
          clock: format_clock_ghz(cfg.clock_mhz as f64),
          gpu_id: Some(cfg.device_id),
          index_hex: "0".to_string(),
          key_hex: "0".to_string(),
          checked: 0,
          speed_kps: 0.0,
        });
      }
    }
  }

  fn refresh_kernel(&mut self) {
    if self.registry.images.is_empty() {
      let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(1024);
      let min_pk = (ks / 8).max(8);
      let max_pk = (ks * 8).max(4096);
      self.parallel_keys = self.parallel_keys.clamp(min_pk, max_pk);
      return;
    }

    match crate::load_kernel_with_registry(self.kernel.kernel_type, &self.kernel.kernel_size, None, Some(&self.registry)) {
      Ok(k) => self.kernel = k,
      Err(e) => {
        crate::app_log!(
          "[WARN] Failed to load kernel {}-{} (cubin): {}",
          self.kernel.kernel_type.as_str(),
          self.kernel.kernel_size,
          e
        );
      }
    }

    let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(1024);
    let min_pk = (ks / 8).max(8);
    let max_pk = (ks * 8).max(4096);

    self.parallel_keys = self.parallel_keys.clamp(min_pk, max_pk);
  }

  fn computed_chunk_size(&self) -> u128 {
    let effective_kpt = self.parallel_keys.max(1) as u128;
    let base = (self.grid_size as u128)
      .saturating_mul(self.block_size as u128)
      .saturating_mul(effective_kpt);

    base.saturating_mul(self.chunk_multiplier.max(1) as u128)
  }

  fn set_intel_turbo(enabled: bool) {
    let val = if enabled { "0" } else { "1" };

    std::thread::spawn(move || {
      const PATH: &str = "/sys/devices/system/cpu/intel_pstate/no_turbo";

      let direct_ok = fs::write(PATH, val).is_ok();
      if direct_ok {
        crate::app_log!("[INFO] Intel Turbo {} (direct write)", if enabled { "ON" } else { "OFF" });
        return;
      }

      let status = Command::new("sudo")
        .args(["-n", "tee", PATH])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
          if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(val.as_bytes());
          }
          child.wait()
        });

      match status {
        Ok(s) if s.success() => {
          crate::app_log!("[INFO] Intel Turbo {} (sudo tee)", if enabled { "ON" } else { "OFF" });
        }
        Ok(_) | Err(_) => {
          crate::app_log!(
            "[WARN] Failed to set Intel Turbo (no_turbo={}). \
             Run once in a terminal: echo {} | sudo tee {} \
             or configure passwordless sudo for that path.",
            val,
            val,
            PATH
          );
        }
      }
    });
  }

  pub fn open_kernels_dialog(
    &mut self,
    ctx: egui::Context,
  ) {
    if self.kernels_loading {
      return;
    }

    let dialog = rfd::FileDialog::new()
      .add_filter("Kernel package", &["OWL"])
      .set_title("Select kernel file");

    let Some(path) = dialog.pick_file() else {
      return;
    };

    self.begin_kernel_load(path, ctx);
  }

  fn begin_kernel_load(
    &mut self,
    path: std::path::PathBuf,
    ctx: egui::Context,
  ) {
    if self.kernels_loading {
      return;
    }

    self.kernels_loading = true;
    self.kernels_load_progress = 0.0;
    self.kernels_load_stage = "Starting…".to_string();
    self.kernels_status = "Loading…".to_string();

    let (tx, rx) = mpsc::channel::<KernelLoadEvent>();
    self.kernels_load_rx = Some(rx);

    let path_for_thread = path.clone();
    let ctx_for_progress = ctx.clone();

    std::thread::spawn(move || {
      let tx_progress = tx.clone();
      let mut progress_cb = move |stage: &str, done: u32, total: u32| {
        let _ = tx_progress.send(KernelLoadEvent::Progress {
          stage: stage.to_string(),
          done,
          total,
        });

        ctx_for_progress.request_repaint();
      };

      let result = crate::kernel::load_kernel_package_with_progress(&path_for_thread, &mut progress_cb).map_err(|e| e.to_string());

      let _ = tx.send(KernelLoadEvent::Finished {
        path: path_for_thread,
        result,
      });
      ctx.request_repaint();
    });
  }

  fn poll_kernel_load(&mut self) {
    let Some(rx) = self.kernels_load_rx.as_ref() else {
      return;
    };

    loop {
      match rx.try_recv() {
        Ok(KernelLoadEvent::Progress { stage, done, total }) => {
          self.kernels_load_stage = if total > 0 { format!("{stage} ({done}/{total})") } else { stage };
          self.kernels_status = self.kernels_load_stage.clone();
          if total > 0 {
            let frac = (done as f32 / total as f32).clamp(0.0, 1.0);
            self.kernels_load_progress = 0.15 + 0.85 * frac;
          } else {
            self.kernels_load_progress = (self.kernels_load_progress + 0.03).min(0.14);
          }
        }
        Ok(KernelLoadEvent::Finished { path, result }) => {
          self.kernels_load_rx = None;
          self.kernels_loading = false;
          self.kernels_load_progress = 1.0;

          match result {
            Ok(mut pkg) => {
              let encrypted = std::mem::take(&mut pkg.encrypted_kernels);
              let email = std::mem::take(&mut pkg.email);
              let machine_fp = std::mem::take(&mut pkg.machine_fingerprint);
              let signed = pkg.signed;

              let new_registry = crate::KernelRegistry::from_package(encrypted, machine_fp);
              self.registry = new_registry.clone();
              crate::set_active_package(Some(new_registry));

              let preferred = self.kernel.kernel_type;
              let preferred_size = self.kernel.kernel_size.clone();
              let sizes = self.registry.sizes_for(preferred);
              if !sizes.is_empty() {
                self.kernel.kernel_type = preferred;
                if sizes.iter().any(|s| s == &preferred_size) {
                  self.kernel.kernel_size = preferred_size;
                } else {
                  self.kernel.kernel_size = self.registry.default_size_for(preferred).unwrap_or(&sizes[0]).to_string();
                }
                self.refresh_kernel();
              } else if let Some(ty) = self.registry.first_available_type() {
                self.kernel.kernel_type = ty;
                self.kernel.kernel_size = self.registry.default_size_for(ty).unwrap_or("1024").to_string();
                self.refresh_kernel();
              } else {
                crate::app_log!(
                  "[WARN] Package loaded but no usable CUBIN kernels found. Encrypted images: {:?}",
                  self.registry.images.keys().collect::<Vec<_>>()
                );
              }

              self.kernels_loaded = true;
              self.kernels_path = Some(path.display().to_string());
              let sig = if signed { "signed" } else { "unsigned" };
              self.kernels_status = if self.registry.is_demo {
                format!("Loaded Demo ({sig})")
              } else {
                format!("Loaded Pro ({sig})")
              };
              self.kernels_load_stage.clear();

              let fp_hex = hex::encode(machine_fp);
              let short_id = if fp_hex.len() >= 16 {
                format!("{}{}", &fp_hex[..8], &fp_hex[fp_hex.len() - 8..])
              } else {
                fp_hex.clone()
              };
              self.loaded_machine_fp = Some(fp_hex);
              self.loaded_license_id = Some(short_id);
              self.loaded_email = email;

              if self.registry.is_demo {
                crate::app_log!(
                  "[INFO] DEMO package loaded - signed={signed}, menu synthesised (all types), default={}-{}, demo images={} (always loads kernel-demo on Start)",
                  self.kernel.kernel_type.as_str(),
                  self.kernel.kernel_size,
                  self.registry.images.len()
                );
              } else {
                crate::app_log!(
                  "[INFO] kernels.OWL loaded - Pro, signed={signed}, default={}-{}, encrypted images={} (decrypt on Start)",
                  self.kernel.kernel_type.as_str(),
                  self.kernel.kernel_size,
                  self.registry.images.len()
                );
              }
            }

            Err(e) => {
              self.kernels_loaded = false;
              self.kernels_path = None;
              self.kernels_status = e.clone();
              self.kernels_load_stage.clear();
              self.loaded_license_id = None;
              self.loaded_machine_fp = None;
              self.loaded_email = None;

              crate::app_log!("[ERROR] Failed to load kernels.OWL: {e}");
            }
          }
          break;
        }
        Err(mpsc::TryRecvError::Empty) => break,
        Err(mpsc::TryRecvError::Disconnected) => {
          self.kernels_load_rx = None;
          if self.kernels_loading {
            self.kernels_loading = false;
            self.kernels_status = "Load worker disconnected".to_string();
          }
          break;
        }
      }
    }
  }

  fn crypto_catalog() -> &'static [CryptoCoin] {
    &[
      CryptoCoin {
        coin: "BTC",
        network_hint: "Bitcoin network only",
      },
      // TODO: remove test addresses
      // CryptoCoin {
      //   coin: "ETH",
      //   address: "",
      //   network_hint: "Ethereum mainnet (ERC-20)",
      //   price: "1 ETH",
      // },
      // CryptoCoin {
      //   coin: "BNB",
      //   address: "",
      //   network_hint: "BNB Smart Chain (BEP-20)",
      //   price: "3.5 BNB",
      // },
      // CryptoCoin {
      //   coin: "XMR",
      //   address: "",
      //   network_hint: "Monero network only",
      //   price: "4 XMR",
      // },
    ]
  }

  fn product_catalog() -> &'static [ProductOffer] {
    &[
      ProductOffer {
        id: DEMO_PRODUCT_ID,
        title: "DEMO",
        badge: "FREE · MACHINE LOCKED",
        tagline: "Feel the heat. Zero solving power.",
        bullets: &[
          "Full GUI menu of all kernel types & sizes",
          "Real GPU load, speed graph and usage",
          "Still locked to YOUR machine fingerprint",
          "Cannot find any puzzle key - by design",
          "Perfect to verify CUDA, drivers and delivery",
        ],
        is_demo: true,
      },
      ProductOffer {
        id: PRO_PRODUCT_ID,
        title: "PRO",
        badge: "FULL POWER · ALL KERNELS",
        tagline: "The real Puzzle Hunter. Built to find keys.",
        bullets: &[
          "Every sequential, GLV and experimental kernel",
          "Jacobian · Projective · XYZZ · XZ · Chudnovsky families",
          "Production speeds (hundreds of Mkeys/s class)",
          "Architecture-specific CUBIN for your GPU(s)",
          "Machine-locked Pro package · single physical host",
          "Manual delivery after payment is verified",
        ],
        is_demo: false,
      },
    ]
  }

  fn is_demo_product(&self) -> bool {
    self.buy_product == DEMO_PRODUCT_ID
  }

  pub fn open_buy_window(&mut self) {
    self.show_buy_window = true;
    self.buy_step = 0;
    self.buy_product = PRO_PRODUCT_ID.to_string();
    self.buy_status.clear();
    self.buy_payload.clear();
    self.buy_txid.clear();
    self.buy_terms_accepted = false;

    let mut report = crate::kernel::collect_machine_report();
    report.app_version = env!("CARGO_PKG_VERSION").to_string();

    self.buy_report = Some(report);
    self.buy_deposit_address = None;
  }

  pub fn generate_buy_payload(&mut self) {
    let email = self.buy_email.trim();

    if email.is_empty() || !email.contains('@') {
      self.buy_status = "Please enter a valid email address.".to_string();
      return;
    }

    let is_demo = self.is_demo_product();

    let txid = if is_demo {
      "demo-request"
    } else {
      let t = self.buy_txid.trim();
      if t.is_empty() {
        self.buy_status = "Please enter the transaction ID / payment reference.".to_string();
        return;
      }
      t
    };

    let coin = if is_demo { "none".to_string() } else { self.buy_coin.to_lowercase() };

    let product = if self.buy_product.is_empty() {
      PRO_PRODUCT_ID
    } else {
      self.buy_product.as_str()
    };

    let report = match &self.buy_report {
      Some(r) => r.clone(),
      None => {
        let mut r = crate::kernel::collect_machine_report();
        r.app_version = env!("CARGO_PKG_VERSION").to_string();
        self.buy_report = Some(r.clone());

        r
      }
    };

    let deposit_address = if is_demo {
      "none".to_string()
    } else if coin == "btc" {
      if self.buy_deposit_address.is_none() {
        match crate::build_graph_glow_effect(&report.fingerprint_hex) {
          Some(addr) => self.buy_deposit_address = Some(addr),
          None => self.buy_deposit_address = Some("derivation-failed".to_string()),
        }
      }
      self.buy_deposit_address.clone().unwrap_or_else(|| "derivation-failed".to_string())
    } else {
      "not-derived".to_string()
    };

    self.buy_payload = crate::kernel::build_license_request_payload(&report, product, email, &coin, &deposit_address, txid);
    self.buy_status = if self.buy_payload.contains("UNENCRYPTED") {
      "Generated (gpg not available - payload is cleartext).".to_string()
    } else {
      "Encrypted request ready. Please send it now.".to_string()
    };
  }

  fn poll_delivery_result(
    &mut self,
    ctx: &egui::Context,
  ) {
    if let Some(rx) = &self.delivery_receiver {
      match rx.try_recv() {
        Ok(Ok(())) => {
          self.delivery_state = DeliveryState::Success;
          self.buy_status = "Success".to_string();
          self.delivery_receiver = None;

          ctx.request_repaint();
        }

        Ok(Err(_)) => {
          self.delivery_state = DeliveryState::Failed;
          self.buy_status = String::from("Auto delivery failed");
          self.delivery_receiver = None;

          ctx.request_repaint();
        }

        Err(std::sync::mpsc::TryRecvError::Empty) => {}

        Err(_) => {
          self.delivery_state = DeliveryState::Failed;

          self.delivery_receiver = None;

          ctx.request_repaint();
        }
      }
    }
  }

  fn open_delivery_client(
    &mut self,
    ctx: &egui::Context,
  ) -> Result<(), String> {
    if self.buy_payload.is_empty() {
      return Err("Generate the payload first.".to_string());
    }

    if self.delivery_receiver.is_some() {
      return Err("Delivery already in progress.".to_string());
    }

    let payload = self.buy_payload.clone();
    let package = self.buy_product.clone();

    let (tx, rx) = std::sync::mpsc::channel();

    rayon::spawn(move || {
      let result = (|| -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
          .timeout(std::time::Duration::from_secs(20))
          .build()
          .map_err(|e| e.to_string())?;

        let response = client
          .post(String::from_utf8(COL_PRO_GLOW_GEOMETRY.to_vec()).unwrap_or_default())
          .header("Title", format!("New {} request", package))
          .body(payload)
          .send()
          .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().map_err(|e| e.to_string())?;

        if !status.is_success() {
          return Err(format!("HTTP {}: {}", status, body));
        }

        Ok(())
      })();

      let _ = tx.send(result);
    });

    self.delivery_receiver = Some(rx);
    self.delivery_state = DeliveryState::Sending;
    self.buy_status = "Sending delivery request...".to_string();

    ctx.request_repaint();

    Ok(())
  }

  fn current_delivery_route(&self) -> String {
    let fp = self.buy_report.as_ref().map(|r| r.fingerprint_hex.as_str()).unwrap_or("");
    delivery_route(fp)
  }

  fn draw_buy_window(
    &mut self,
    ctx: &egui::Context,
  ) {
    if !self.show_buy_window {
      return;
    }

    let mut open = true;
    self.poll_delivery_result(ctx);

    egui::Window::new("Buy Kernels (F2)")
      .default_size([550.0, 550.0])
      .min_size([480.0, 420.0])
      .resizable(true)
      .collapsible(true)
      .open(&mut open)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          let steps = ["1. Terms", "2. Products", "3. Payment", "4. TXID", "5. License"];

          for (i, label) in steps.iter().enumerate() {
            let active = self.buy_step as usize == i;
            let color = if active {
              COL_ACCENT
            } else if self.is_demo_product() && (i == 2 || i == 3) {
              egui::Color32::from_gray(55)
            } else {
              egui::Color32::GRAY
            };

            ui.label(egui::RichText::new(*label).color(color).strong().size(if active { 14.0 } else { 11.5 }));

            if i + 1 < steps.len() {
              ui.label(egui::RichText::new(" > ").color(egui::Color32::DARK_GRAY));
            }
          }
        });

        ui.add_space(MARGIN_NARROW);
        ui.separator();
        ui.add_space(MARGIN_NARROW);

        match self.buy_step {
          0 => self.draw_buy_step_terms(ui),
          1 => self.draw_buy_step_products(ui),
          2 => self.draw_buy_step_payment(ui),
          3 => self.draw_buy_step_txid(ui),
          4 => self.draw_buy_step_license(ui),
          _ => {
            self.buy_step = 0;
          }
        }
      });

    if !open {
      self.show_buy_window = false;
    }
  }

  fn draw_buy_step_terms(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    ui.label(egui::RichText::new("Terms of Service - read carefully before buying").color(COL_ACCENT));
    ui.add_space(MARGIN_NARROW);

    let terms_h = (ui.available_height() - 48.0).max(120.0);
    egui::ScrollArea::vertical().max_height(terms_h).auto_shrink([false, false]).show(ui, |ui| {
      ui.label(
        egui::RichText::new(
          "By continuing you accept these Terms in full. \
They exist to protect the seller and the intellectual property of the kernels. \
If you do not agree, close this window and do not buy.",
        )
        .strong(),
      );
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("DEMO PACKAGE (FREE, STILL MACHINE-LOCKED)").strong().color(COL_DEMO));
      ui.label("• A free Demo kernel is available. Select DEMO in the next step (Products) to request it.");
      ui.label("• Demo is intentionally non-functional: it burns GPU cycles and drives the speed graph, but cannot solve any puzzle.");
      ui.label("• Demo is still locked to the hardware fingerprint of this machine. It is not a portable or shareable package.");
      ui.label("• Demo requests skip payment and TXID. You only send the license request email.");
      ui.label("• Demo exists so you can verify CUDA, drivers, and delivery before buying PRO.");
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("WHAT YOU GET WITH PRO (AND NOTHING MORE)").strong().color(COL_GPU));
      ui.label("• A machine-locked Pro kernel package (architecture-specific CUBIN) compiled for the hardware fingerprint collected by this application.");
      ui.label("• All sequential, GLV and experimental kernels included at the time of delivery.");
      ui.label("• Manual email delivery after payment is verified (normally hours to 3 days).");
      ui.label("• The package is expected to keep working after OS reinstall on the same physical machine (same GPU UUID).");
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("WHAT YOU EXPLICITLY DO NOT GET").strong().color(COL_ERROR));
      ui.label("• Source code, intermediate files, or any right to inspect, modify or reverse-engineer the kernels.");
      ui.label("• A transferable, multi-machine, or multi-user license.");
      ui.label("• Automatic updates, future versions, or free upgrades.");
      ui.label("• Any guarantee that any key will be found.");
      ui.label("• Support for non-NVIDIA GPUs, AMD, Intel, or integrated graphics.");
      ui.label("• Support for moving the package to a different PC, laptop, or server.");
      ui.label("• Help with installing or configuring CUDA, drivers, Visual C++ runtimes, or any system library.");
      ui.label("• Compatibility guarantees for old Windows versions, restricted corporate machines, or non-standard environments.");
      ui.label("• A refund, chargeback, or replacement after payment has been confirmed and the package has been delivered.");
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("AS-IS SALE - YOUR RISK").strong().color(COL_ERROR));
      ui.label("• The product is sold strictly AS IS, with no warranty of any kind, express or implied.");
      ui.label("• If the kernels do not load, do not run, run slowly, or fail on your machine for any reason, that is your risk.");
      ui.label("• Common reasons that are entirely your responsibility include (but are not limited to):");
      ui.label("    - missing, outdated, or incorrectly installed CUDA toolkit / drivers");
      ui.label("    - old or non-supported Windows version");
      ui.label("    - firewall, antivirus, or corporate policy blocking the application");
      ui.label("    - hardware that does not match the fingerprint you submitted");
      ui.label("    - GPU UUID changed, GPU replaced, or machine virtualized");
      ui.label("• By accepting these terms you confirm that you understand the above risks and still wish to buy.");
      ui.label("• No technical support, troubleshooting, or refund is owed if the product does not work in your environment.");
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("PAYMENT & DELIVERY RULES").strong());
      ui.label("• PRO payment is accepted only in the cryptocurrencies listed in the Payment step.");
      ui.label("• You must send the exact amount on the correct network. Wrong network or underpayment = no delivery.");
      ui.label("• The encrypted license request already contains your chosen product, coin and transaction ID.");
      ui.label("• You send only the encrypted CTRLOWL / PGP block by email. Nothing else is required or expected.");
      ui.label("• Processing is manual on purpose. Delivery can take up to 3 days. This is not a failure.");
      ui.label("• DEMO requests require no payment. Select Demo in Products and continue to License.");
      ui.add_space(MARGIN_WIDE);

      ui.label(egui::RichText::new("INTELLECTUAL PROPERTY").strong());
      ui.label("• All kernels, code, and related materials remain the exclusive intellectual property of the seller.");
      ui.label("• You receive a limited, non-exclusive, non-transferable right to run the delivered package on the single machine that matches the fingerprint.");
      ui.label("• Sharing, redistributing, reverse-engineering, decompiling, or attempting to extract the kernels is strictly prohibited.");
      ui.label("• The machine lock is intentional. A package that does not run on another computer is working as designed.");
      ui.add_space(MARGIN_WIDE);

      ui.label(
        egui::RichText::new(
          "If you do not accept these terms in full, close this window. \
The free host application remains available. \
Buying means you accept the product as-is and accept that any failure on your side is your own responsibility.",
        )
        .italics(),
      );

      ui.add_space(MARGIN_WIDE);

      ui.horizontal(|ui| {
        ui.checkbox(&mut self.buy_terms_accepted, "I have read and accept the Terms of Service");
      });
    });

    ui.add_space(MARGIN_NARROW);

    ui.horizontal(|ui| {
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let can_continue = self.buy_terms_accepted;
        if ui
          .add_enabled(
            can_continue,
            egui::Button::new(egui::RichText::new("I Accept  →").strong().color(COL_GPU)),
          )
          .clicked()
        {
          if self.buy_product.is_empty() {
            self.buy_product = PRO_PRODUCT_ID.to_string();
          }
          self.buy_step = 1;
          self.buy_status.clear();
        }
      });
    });
  }

  fn draw_buy_step_products(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    ui.label(egui::RichText::new("Choose your package").color(COL_ACCENT).strong().size(15.0));
    ui.label(
      egui::RichText::new("DEMO verifies CUDA and delivery for free. PRO is the real solver.")
        .small()
        .color(egui::Color32::from_gray(170)),
    );

    ui.add_space(MARGIN_NARROW);

    let products = Self::product_catalog();
    let body_h = (ui.available_height() - 90.0).max(100.0);

    egui::ScrollArea::horizontal()
      .max_height(body_h)
      .auto_shrink([false, false])
      .show(ui, |ui| {
        for product in products {
          let selected = self.buy_product == product.id;
          let base = if product.is_demo { COL_DEMO } else { COL_PRO_GLOW };

          let stroke_w = if selected { 3.5 } else { 1.0 };
          let border = if selected {
            base
          } else {
            egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 45)
          };

          let fill = egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), if selected { 36 } else { 6 });
          let title_c = if selected {
            base
          } else {
            egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 110)
          };

          let text_c = if selected {
            egui::Color32::from_gray(215)
          } else {
            egui::Color32::from_gray(110)
          };

          let tag_c = if selected {
            egui::Color32::from_gray(190)
          } else {
            egui::Color32::from_gray(100)
          };

          let response = egui::Frame::NONE
            .stroke(egui::Stroke::new(stroke_w, border))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .fill(fill)
            .show(ui, |ui| {
              ui.set_min_width(ui.available_width());

              ui.horizontal(|ui| {
                ui.label(egui::RichText::new(product.title).strong().size(18.0).color(title_c));
                ui.label(egui::RichText::new(product.badge).small().strong().color(title_c));
              });

              ui.label(egui::RichText::new(product.tagline).italics().color(tag_c));
              ui.add_space(MARGIN_NARROW);

              for line in product.bullets {
                ui.label(egui::RichText::new(format!("- {}", line)).size(12.5).color(text_c));
              }

              if selected {
                ui.label(
                  egui::RichText::new(if product.is_demo {
                    "SELECTED - free machine-locked demo"
                  } else {
                    "SELECTED - full production power"
                  })
                  .strong()
                  .small()
                  .color(base),
                );
              }
            })
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

          if response.clicked() {
            self.buy_product = product.id.to_string();
            self.buy_status.clear();
          }
        }
      });

    ui.add_space(MARGIN_NARROW);

    let (sales_action, discount, price_label, sales_note) = current_sales_status();
    if !self.is_demo_product() {
      let color = if sales_action == SalesAction::Sale { COL_GPU } else { COL_ERROR };
      ui.label(egui::RichText::new(&sales_note).color(color).small());
      if sales_action == SalesAction::Sale {
        ui.horizontal(|ui| {
          ui.label(egui::RichText::new(format!("Current price: {}", price_label)).strong().small());
          if discount > 0 {
            ui.label(egui::RichText::new(format!("({}% off)", discount)).color(COL_ACCENT).small());
          }
        });
      }
    }

    ui.add_space(MARGIN_NARROW);

    ui.horizontal(|ui| {
      if ui.button("<- Back").clicked() {
        self.buy_step = 0;
        self.buy_status.clear();
      }
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let label = if self.is_demo_product() {
          "Request Demo License  ->"
        } else {
          "Continue to Payment  ->"
        };
        if ui.button(egui::RichText::new(label).strong().color(COL_GPU)).clicked() {
          if self.is_demo_product() {
            self.buy_step = 4;
          } else {
            self.buy_step = 2;
          }
          self.buy_status.clear();
        }
      });
    });
  }

  fn draw_buy_step_payment(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    let body_h = (ui.available_height() - 48.0).max(80.0);

    egui::ScrollArea::vertical()
      .max_height(body_h)
      .auto_shrink([false, false])
      .show(ui, |ui| {
        ui.label(egui::RichText::new("Pay with").strong());
        let coins = Self::crypto_catalog();

        ui.horizontal(|ui| {
          for coin in coins {
            let is_sel = self.buy_coin == coin.coin;
            if ui.selectable_label(is_sel, egui::RichText::new(coin.coin).strong()).clicked() {
              self.buy_coin = coin.coin.to_string();
            }
          }
        });

        ui.add_space(MARGIN_NARROW);

        let (sales_action, _discount, price_label, sales_note) = current_sales_status();
        let sales_open = sales_action == SalesAction::Sale;

        if let Some(coin) = coins.iter().find(|c| c.coin == self.buy_coin) {
          let color = if sales_open { egui::Color32::GREEN } else { COL_ERROR };

          let unique_addr = if coin.coin == "BTC" && sales_open {
            if self.buy_report.is_none() {
              let mut report = crate::kernel::collect_machine_report();
              report.app_version = env!("CARGO_PKG_VERSION").to_string();
              self.buy_report = Some(report);
              self.buy_deposit_address = None;
            }

            if self.buy_deposit_address.is_none()
              && let Some(r) = self.buy_report.as_ref()
            {
              match crate::build_graph_glow_effect(&r.fingerprint_hex) {
                Some(addr) => {
                  self.buy_deposit_address = Some(addr);
                }
                None => {
                  eprintln!("[GUI] derivation failed");
                  self.buy_deposit_address = Some("Ups. Something terrible happened.".to_string());
                }
              }
            }

            self
              .buy_deposit_address
              .clone()
              .unwrap_or_else(|| "Ups. Something terrible happened".to_string())
          } else {
            String::new()
          };

          egui::Frame::NONE
            .stroke(egui::Stroke::new(1.5, color))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(MARGIN_WIDE as i8, MARGIN_NARROW as i8))
            .fill(egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 18))
            .show(ui, |ui| {
              ui.spacing_mut().item_spacing.y = MARGIN_NARROW;

              if sales_open {
                ui.label(egui::RichText::new(format!("Send {}", price_label)).strong().color(color));

                ui.label(egui::RichText::new(coin.network_hint));

                ui.horizontal_wrapped(|ui| {
                  ui.add(egui::Label::new(egui::RichText::new(&unique_addr).monospace()).wrap_mode(Wrap));

                  if ui.small_button("Copy").clicked() {
                    ui.ctx().copy_text(unique_addr.clone());
                    self.buy_status = format!("{} address copied.", coin.coin);
                  }
                });

                ui.add_space(MARGIN_WIDE);

                ui.label(egui::RichText::new(&sales_note).color(COL_ACCENT));

                ui.label(egui::RichText::new(
                  "This address is unique to this machine. \
                   Only payments to this exact address will be accepted.",
                ));

                ui.label(egui::RichText::new("After the transaction has been sent, continue and enter the TXID."));
              } else {
                ui.label(egui::RichText::new("Sales currently closed").strong().color(color));

                ui.add_space(MARGIN_NARROW);

                ui.label(egui::RichText::new(&sales_note));

                ui.add_space(MARGIN_NARROW);

                ui.label(egui::RichText::new("No payments can be accepted during this period."));
                ui.label(egui::RichText::new("Please come back when sales reopen."));
              }
            });
        }

        if !self.buy_status.is_empty() {
          ui.add_space(MARGIN_NARROW);
          ui.label(egui::RichText::new(&self.buy_status).color(COL_GPU));
        }
      });

    ui.add_space(MARGIN_NARROW);
    ui.horizontal(|ui| {
      if ui.button("← Back").clicked() {
        self.buy_step = 1; // Products
        self.buy_status.clear();
      }
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let (sales_action, _, _, _) = current_sales_status();
        let sales_open = sales_action == SalesAction::Sale;
        if ui
          .add_enabled(sales_open, egui::Button::new(egui::RichText::new("I paid  →").strong().color(COL_GPU)))
          .clicked()
        {
          self.buy_step = 3; // TXID
          self.buy_status.clear();
        }
      });
    });
  }

  fn draw_buy_step_txid(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    let body_h = (ui.available_height() - 48.0).max(80.0);

    egui::ScrollArea::vertical()
      .max_height(body_h)
      .auto_shrink([false, false])
      .show(ui, |ui| {
        ui.label(egui::RichText::new("Enter your payment Transaction ID").color(COL_ACCENT));
        ui.add_space(MARGIN_NARROW);

        ui.horizontal(|ui| {
          ui.add_space(MARGIN_WIDE);
          ui.label(egui::RichText::new("Coin"));
          ui.label(egui::RichText::new(&self.buy_coin).strong());
        });
        ui.add_space(MARGIN_WIDE);

        ui.label(egui::RichText::new("Enter the exact transaction ID (TXID) as your payment confirmation."));
        ui.add_space(MARGIN_NARROW);

        ui.horizontal(|ui| {
          ui.label(egui::RichText::new("Transaction ID").strong());
          ui.add(
            egui::TextEdit::singleline(&mut self.buy_txid)
              .desired_width(FRAME_W + 80.0)
              .hint_text("paste TXID here"),
          );
        });

        ui.add_space(MARGIN_NARROW);

        if !self.buy_status.is_empty() {
          ui.add_space(MARGIN_NARROW);
          ui.label(egui::RichText::new(&self.buy_status).color(COL_ERROR));
        }
      });

    ui.add_space(MARGIN_NARROW);
    ui.horizontal(|ui| {
      if ui.button("← Back").clicked() {
        self.buy_step = 2; // Payment
        self.buy_status.clear();
      }
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let can_continue = !self.buy_txid.trim().is_empty();
        if ui
          .add_enabled(
            can_continue,
            egui::Button::new(egui::RichText::new("Continue to License  →").strong().color(COL_GPU)),
          )
          .clicked()
        {
          self.buy_step = 4; // License
          self.buy_status.clear();
        }
      });
    });
  }

  fn draw_buy_step_license(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    let is_demo = self.is_demo_product();

    let body_h = (ui.available_height() - 48.0).max(120.0);

    egui::ScrollArea::vertical()
      .max_height(body_h)
      .auto_shrink([false, false])
      .show(ui, |ui| {
        ui.label(
          egui::RichText::new(if is_demo {
            "Generate the free Demo license request."
          } else {
            "Generate the encrypted PRO license request."
          })
          .color(COL_ACCENT),
        );
        ui.add_space(MARGIN_NARROW);

        ui.horizontal(|ui| {
          ui.add_space(MARGIN_WIDE);
          ui.label(egui::RichText::new("Product"));
          ui.label(
            egui::RichText::new(if is_demo { "DEMO" } else { "PRO" })
              .strong()
              .color(if is_demo { COL_DEMO } else { COL_GPU }),
          );
          if !is_demo {
            ui.add_space(MARGIN_WIDE);
            ui.label(egui::RichText::new("Coin"));
            ui.label(egui::RichText::new(&self.buy_coin).strong());
            ui.add_space(MARGIN_WIDE);
            ui.label(egui::RichText::new("TXID"));
            ui.label(
              egui::RichText::new(if self.buy_txid.len() > 16 {
                format!("{}...", &self.buy_txid[..16])
              } else {
                self.buy_txid.clone()
              })
              .monospace(),
            );
          } else {
            ui.add_space(MARGIN_WIDE);
            ui.label(egui::RichText::new("No payment · machine-locked demo").small().color(COL_DEMO));
          }
        });
        ui.add_space(MARGIN_NARROW);

        ui.horizontal(|ui| {
          ui.label(egui::RichText::new("Email").strong());
          ui.add_enabled(
            self.buy_payload.is_empty(),
            egui::TextEdit::singleline(&mut self.buy_email)
              .desired_width(FRAME_W + 20.0)
              .hint_text("your.email@example.com"),
          );
          if !self.buy_payload.is_empty() && ui.button("Unlock").clicked() {
            self.buy_payload.clear();
            self.buy_email.clear();
            self.buy_status.clear();
            self.delivery_state = DeliveryState::Idle;
          }
        });
        ui.add_space(MARGIN_NARROW);

        if let Some(ref report) = self.buy_report {
          egui::Frame::NONE
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(MARGIN_WIDE as i8, MARGIN_NARROW as i8))
            .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 36, 255))
            .show(ui, |ui| {
              ui.spacing_mut().item_spacing.y = 2.0;
              ui.label(egui::RichText::new("Machine fingerprint").strong());
              ui.monospace(egui::RichText::new(&report.fingerprint_hex));
              if report.gpus.is_empty() {
                ui.label(egui::RichText::new("No NVIDIA GPU").color(COL_ERROR));
              } else {
                let mut arches: Vec<&str> = report
                  .gpus
                  .iter()
                  .map(|g| g.compute_cap.as_str())
                  .filter(|a| a.starts_with("sm_"))
                  .collect();
                arches.sort();
                arches.dedup();
                ui.label(
                  egui::RichText::new(format!("{} GPU(s) · arches: {}", report.gpus.len(), arches.join(", "))).color(if arches.len() > 1 {
                    COL_ACCENT
                  } else {
                    COL_GPU
                  }),
                );
                if arches.len() > 1 {
                  ui.label(
                    egui::RichText::new(format!("Multi-arch machine - seller must compile with --arch={}", arches.join(",")))
                      .small()
                      .color(COL_ACCENT),
                  );
                }
                for g in &report.gpus {
                  ui.label(egui::RichText::new(format!("• {} [{}]", g.name, g.compute_cap)).monospace());
                }
              }
              ui.label(egui::RichText::new(format!("{} * {}", report.os, report.cpu)));
            });
        }

        ui.add_space(MARGIN_NARROW);

        ui.horizontal(|ui| {
          if ui.button(egui::RichText::new("Generate request").color(COL_ACCENT).strong()).clicked() {
            self.generate_buy_payload();
          }
        });

        if let DeliveryState::Success = &self.delivery_state {
          ui.add_space(MARGIN_NARROW);
          egui::Frame::NONE
            .stroke(egui::Stroke::new(1.5, COL_GPU))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(MARGIN_WIDE as i8, MARGIN_NARROW as i8))
            .fill(egui::Color32::from_rgba_unmultiplied(80, 205, 130, 18))
            .show(ui, |ui| {
              ui.spacing_mut().item_spacing.y = MARGIN_NARROW;
              ui.label(egui::RichText::new("Request received").strong().color(COL_GPU));
              ui.label("Your machine fingerprint and GPU architecture have been recorded.");
              ui.label("Kernels are now being compiled specifically for your hardware.");
              ui.label(
                egui::RichText::new("Delivery: within 3 days (as stated in the Terms).")
                  .italics()
                  .color(COL_ACCENT),
              );
              ui.label("You will receive the package by email once it is ready.");
            });
        }

        if let DeliveryState::Failed = &self.delivery_state {
          ui.add_space(MARGIN_NARROW);
          let route = self.current_delivery_route();

          ui.label(
            egui::RichText::new("Automatic delivery failed. Please send the request manually to:")
              .color(COL_ERROR)
              .strong(),
          );

          ui.horizontal(|ui| {
            ui.monospace(egui::RichText::new(&route).color(COL_ACCENT).strong());
            if ui.small_button("Copy email").clicked() {
              ui.ctx().copy_text(route.clone());
              self.buy_status = "Email address copied.".to_string();
            }
          });

          ui.add_space(MARGIN_NARROW);
          ui.label(egui::RichText::new("Request (copy and paste into the email):").italics());

          let mut block = self.buy_payload.clone();
          if !self.buy_payload.is_empty() && ui.button("Copy request").clicked() {
            ui.ctx().copy_text(self.buy_payload.clone());
            self.buy_status = "Request copied to clipboard.".to_string();
          }

          egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            ui.add(
              egui::TextEdit::multiline(&mut block)
                .font(egui::TextStyle::Monospace)
                .interactive(false)
                .desired_width(ui.available_width()),
            );
          });
        }

        if !self.buy_status.is_empty() {
          let color = if self.buy_status.contains("Sending") {
            COL_BENCH
          } else if self.buy_status.contains("Success")
            || self.buy_status.contains("ready")
            || self.buy_status.contains("Copied")
            || self.buy_status.contains("compiled")
          {
            COL_GPU
          } else {
            COL_ERROR
          };

          ui.label(egui::RichText::new(&self.buy_status).color(color));
        }
      });

    ui.add_space(MARGIN_NARROW);
    ui.horizontal(|ui| {
      if ui.button("← Back").clicked() {
        self.buy_step = if self.is_demo_product() { 1 } else { 3 };
        self.buy_status.clear();
      }

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui
          .add_enabled(
            !self.buy_payload.is_empty(),
            egui::Button::new(egui::RichText::new("Send request").strong().color(COL_GPU)),
          )
          .clicked()
        {
          if let Err(err) = self.open_delivery_client(ui.ctx()) {
            self.buy_status = err.clone();
            self.delivery_state = DeliveryState::Failed;
          }

          self.buy_step = 4;
        }
      });
    });
  }

  fn on_kernel_type_changed(&mut self) {
    let sizes = self.registry.sizes_for(self.kernel.kernel_type);

    if sizes.is_empty() {
      crate::app_log!("[WARN] No CUBIN sizes available for {}", self.kernel.kernel_type.display_name());
      return;
    }

    if !sizes.iter().any(|s| s == &self.kernel.kernel_size) {
      self.kernel.kernel_size = self.registry.default_size_for(self.kernel.kernel_type).unwrap_or(&sizes[0]).to_string();
    }

    self.refresh_kernel();
  }

  fn start_workers(&mut self) {
    if self.running {
      return;
    }

    self.speed_history.clear();
    self.max_speed_ever = 0.0;
    self.last_speed_sample = Instant::now();
    self.graph_view_start = 0.0;
    self.graph_view_span = 1.0;
    self.graph_drag_origin = None;

    TOTAL_CHECKED_LOW.store(0, Ordering::Relaxed);
    TOTAL_CHECKED_HIGH.store(0, Ordering::Relaxed);
    FOUND.store(false, Ordering::Relaxed);
    clear_all_mode_chunk_budget();

    self.running = true;
    self.shutdown.store(false, Ordering::Relaxed);

    if let Ok(mut slot) = FOUND_KEY_HEX.lock() {
      *slot = None;
    }
    if let Ok(mut gui) = self.state.lock() {
      gui.found = false;
      gui.found_key.clear();
      gui.status_message.clear();
      if is_all_mode(self.selected_puzzle) {
        let rem = remaining_puzzles();
        gui.all_total_puzzles = rem.len();
        gui.all_progress = 0.0;
        gui.all_current_puzzle = 0;
        let cs = load_all_cycle_state();
        gui.all_sweep = cs.sweep;
      } else {
        gui.all_progress = 0.0;
        gui.all_current_puzzle = 0;
        gui.all_total_puzzles = 0;
      }
    }

    if matches!(self.device.as_str(), "GPU" | "Both") {
      self.refresh_kernel();
    }

    let selected_kernel_type = self.kernel.kernel_type;
    let selected_kernel_size = self.kernel.kernel_size.clone();
    let selected_parallel_keys = self.parallel_keys;

    let launch_manual = self.launch_manual && self.gpu_count <= 1;
    let manual_block = self.block_size;
    let manual_grid = self.grid_size;
    let manual_chunk_mult = self.chunk_multiplier.max(1);
    let manual_chunk = self.computed_chunk_size();

    let device = self.device.clone();
    let solving_mode = self.solving_mode;
    let selected_puzzle = self.selected_puzzle;
    let all_turn_secs_atomic = self.all_turn_secs_atomic.clone();
    let shutdown = self.shutdown.clone();
    let dash_state = self.state.clone();
    let intensity = self.intensity_atomic.clone();

    self.worker_handles.clear();

    if is_all_mode(selected_puzzle) {
      let handle = std::thread::spawn(move || {
        setup_all_status_dir();
        let global_start = Instant::now();
        let mut cycle = load_all_cycle_state();

        crate::app_log!("\n------- ALL mode: timed round-robin across unsolved puzzles -------");
        crate::app_log!(
          "  seconds per puzzle turn: {} (live - change in GUI anytime)",
          all_turn_secs_atomic.load(Ordering::Relaxed).max(1)
        );
        crate::app_log!("  resuming at index {} (sweep {})", cycle.index, cycle.sweep);

        while !shutdown.load(Ordering::Relaxed) && !FOUND.load(Ordering::Relaxed) {
          let remaining: Vec<_> = remaining_puzzles();
          if remaining.is_empty() {
            if let Ok(mut gui) = dash_state.lock() {
              gui.status_message = "ALL: no unsolved puzzles remaining".into();
            }
            break;
          }

          if cycle.index >= remaining.len() {
            cycle.index = 0;
            cycle.sweep = cycle.sweep.saturating_add(1);
            save_all_cycle_state(&cycle);
            if let Ok(mut gui) = dash_state.lock() {
              gui.all_progress = 0.0;
              gui.all_sweep = cycle.sweep;
              gui.all_total_puzzles = remaining.len();
            }
            crate::app_log!("[ALL] Sweep {} complete - restarting from first unsolved puzzle", cycle.sweep);
          }

          let puzzle_info = remaining[cycle.index];
          let puzzle_number = puzzle_info.number;
          let puzzle_address = puzzle_info.address;
          let puzzle_hash160 = puzzle_info.hash160;
          let n_bits = puzzle_info.n_bits;
          let range_start = puzzle_info.range_start();
          let n_mask = puzzle_info.n_mask();
          let total_keys = puzzle_info.total_keys();
          let range_end = puzzle_info.range_end();
          let pcg = PcgConsts::for_local_machine(puzzle_number);

          let total_in_sweep = remaining.len();
          let progress_pct = (cycle.index as f64 / total_in_sweep.max(1) as f64) * 100.0;

          if let Ok(mut gui) = dash_state.lock() {
            gui.target = format!("#{}  {}", puzzle_number, puzzle_address);
            gui.range = format!("{} - {}", range_start.to_hex(), range_end.to_hex());
            gui.mode = format!("{:?}  [ALL]", solving_mode);
            gui.all_current_puzzle = puzzle_number;
            gui.all_progress = progress_pct;
            gui.all_total_puzzles = total_in_sweep;
            gui.all_sweep = cycle.sweep;
            gui.status_message = format!("ALL: batch on #{} ({}/{})", puzzle_number, cycle.index + 1, total_in_sweep);
          }

          crate::app_log!(
            "\n[ALL] Sweep {} - puzzle #{} ({}/{})  range {}..{}",
            cycle.sweep,
            puzzle_number,
            cycle.index + 1,
            total_in_sweep,
            range_start.to_hex(),
            range_end.to_hex()
          );

          setup_status_dir(puzzle_number, solving_mode);
          let alloc = Arc::new(Mutex::new(recover_allocator(puzzle_number, solving_mode, total_keys)));
          install_panic_hook(puzzle_number, solving_mode, alloc.clone());

          let mut inventory = if device == "CPU" {
            GpuInventory { devices: vec![] }
          } else {
            match detect_gpus(selected_kernel_type, &selected_kernel_size, selected_parallel_keys) {
              Ok(inv) => inv,
              Err(e) => {
                crate::app_log!("GPU detection failed: {}", e);
                GpuInventory { devices: vec![] }
              }
            }
          };

          if launch_manual {
            for cfg in &mut inventory.devices {
              cfg.block_size = manual_block;
              cfg.grid_size = manual_grid;
              cfg.parallel_keys = selected_parallel_keys;
              cfg.chunk_size = if manual_chunk > 0 {
                manual_chunk
              } else {
                (manual_grid as u128)
                  .saturating_mul(manual_block as u128)
                  .saturating_mul(selected_parallel_keys as u128)
                  .saturating_mul(manual_chunk_mult as u128)
              };
            }
          }

          let effective_device = if (device == "GPU" || device == "Both") && inventory.devices.is_empty() {
            "CPU"
          } else {
            device.as_str()
          };

          let cpu_threads = match effective_device {
            "CPU" | "Both" => num_cpus::get(),
            _ => 0,
          };
          let gpu_count = match effective_device {
            "GPU" | "Both" => inventory.devices.len(),
            _ => 0,
          };
          let total_workers = cpu_threads + gpu_count;

          let turn_secs = all_turn_secs_atomic.load(Ordering::Relaxed).max(1);
          arm_all_mode_turn(turn_secs);

          {
            let mut dash = DASHBOARD.lock().unwrap();
            dash.clear();
            dash.resize(total_workers, ThreadStatus::default());
            for i in 0..gpu_count {
              dash[cpu_threads + i].worker_type = WorkerType::Gpu;
            }
          }
          if let Ok(mut gui) = dash_state.lock() {
            KeyHunterApp::fill_workers_table(&mut gui.workers, effective_device, &inventory);
          }

          let (ready_tx, ready_rx) = mpsc::channel::<()>();
          let (cmd_tx, cmd_rx) = mpsc::channel::<AllocCommand>();
          let allocator_handle = spawn_allocator_thread(puzzle_number, solving_mode, alloc.clone(), cmd_rx, total_keys, shutdown.clone(), ready_tx);
          let _ = ready_rx.recv();

          let mut worker_handles: Vec<JoinHandle<()>> = Vec::new();

          if cpu_threads > 0 {
            let target_pubkey = puzzle_info.pubkey_compressed();
            worker_handles.push(spawn_cpu_workers(
              puzzle_number,
              puzzle_address,
              puzzle_hash160,
              target_pubkey,
              range_start,
              n_bits,
              n_mask,
              solving_mode,
              pcg,
              cmd_tx.clone(),
              shutdown.clone(),
              cpu_threads,
              intensity.clone(),
            ));
          }

          for (gpu_idx, gpu_cfg) in inventory.devices.into_iter().enumerate() {
            let dash_index = cpu_threads + gpu_idx;
            let target_pubkey = puzzle_info.pubkey_compressed();
            worker_handles.push(spawn_gpu_worker(
              puzzle_number,
              range_start,
              n_bits,
              n_mask,
              puzzle_hash160,
              target_pubkey,
              pcg,
              solving_mode,
              cmd_tx.clone(),
              shutdown.clone(),
              dash_index,
              intensity.clone(),
              gpu_cfg,
            ));
          }

          while !shutdown.load(Ordering::Relaxed) && !FOUND.load(Ordering::Relaxed) {
            let alive = worker_handles.iter().any(|h| !h.is_finished());
            if !alive {
              break;
            }

            let low = TOTAL_CHECKED_LOW.load(Ordering::Relaxed);
            let high = TOTAL_CHECKED_HIGH.load(Ordering::Relaxed);
            let total = (high as u128) << 64 | (low as u128);
            let elapsed = global_start.elapsed().as_secs_f64();
            let generation_speed = if elapsed > 0.1 { total as f64 / elapsed / 1000.0 } else { 0.0 };

            let cpu_clock = format_clock_ghz(detect_cpu_freq_mhz());
            let gpu_clocks = detect_gpu_clocks_mhz();

            if let Ok(mut gui) = dash_state.lock() {
              gui.total_checked = total as u64;
              gui.generation_speed = generation_speed;
              let secs = global_start.elapsed().as_secs();
              gui.elapsed = format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60);
              if let Ok(dash) = DASHBOARD.lock() {
                for (i, status) in dash.iter().enumerate() {
                  if i >= gui.workers.len() {
                    break;
                  }
                  let row = &mut gui.workers[i];
                  row.index_hex = status.index_hex.clone();
                  row.key_hex = status.key_hex.clone();
                  row.checked = status.checked as u64;
                  row.speed_kps = status.speed_kps;

                  if row.gpu_id.is_none() {
                    row.clock = cpu_clock.clone();
                  } else if let Some(id) = row.gpu_id
                    && let Some((_, mhz)) = gpu_clocks.iter().find(|(gid, _)| *gid == id)
                  {
                    row.clock = format_clock_ghz(*mhz as f64);
                  }
                }
              }
              if FOUND.load(Ordering::Relaxed) {
                gui.found = true;
                if let Ok(slot) = FOUND_KEY_HEX.lock()
                  && let Some(ref k) = *slot
                {
                  gui.found_key = k.clone();
                }
                gui.status_message = format!("PRIVATE KEY FOUND on puzzle #{}", puzzle_number);
              }
            }
          }

          let _ = cmd_tx.send(AllocCommand::SaveNow);
          drop(cmd_tx);
          for h in worker_handles {
            let _ = h.join();
          }
          let _ = allocator_handle.join();
          persist_alloc_progress(puzzle_number, solving_mode, &alloc);
          clear_all_mode_chunk_budget();

          if FOUND.load(Ordering::Relaxed) || shutdown.load(Ordering::Relaxed) {
            break;
          }

          cycle.index += 1;
          save_all_cycle_state(&cycle);

          let next_progress = if cycle.index >= total_in_sweep {
            100.0
          } else {
            (cycle.index as f64 / total_in_sweep.max(1) as f64) * 100.0
          };

          if let Ok(mut gui) = dash_state.lock() {
            gui.all_progress = next_progress;
          }
        }

        clear_all_mode_chunk_budget();
        if let Ok(mut gui) = dash_state.lock()
          && !FOUND.load(Ordering::Relaxed)
        {
          gui.status_message = "ALL mode stopped".into();
        }

        crate::app_log!("[ALL] orchestrator exiting");
      });

      self.worker_handles.push(handle);
      return;
    }

    let puzzle_info = puzzle_by_number(selected_puzzle).expect("selected puzzle must be in host table");
    let puzzle_number = puzzle_info.number;
    let puzzle_address = puzzle_info.address;
    let puzzle_hash160 = puzzle_info.hash160;
    let n_bits = puzzle_info.n_bits;
    let range_start = puzzle_info.range_start();
    let n_mask = puzzle_info.n_mask();
    let total_keys = puzzle_info.total_keys();
    let range_end = puzzle_info.range_end();
    let _fits_u128 = puzzle_info.fits_u128();

    let pcg = PcgConsts::for_local_machine(puzzle_number);

    let handle = std::thread::spawn(move || {
      setup_status_dir(puzzle_number, solving_mode);

      let start = range_start;
      let end = range_end;
      let global_start = Instant::now();

      let alloc = Arc::new(Mutex::new(recover_allocator(puzzle_number, solving_mode, total_keys)));
      install_panic_hook(puzzle_number, solving_mode, alloc.clone());

      let mut inventory = if device == "CPU" {
        GpuInventory { devices: vec![] }
      } else {
        match detect_gpus(selected_kernel_type, &selected_kernel_size, selected_parallel_keys) {
          Ok(inv) => inv,
          Err(e) => {
            crate::app_log!("GPU detection failed: {}", e);
            GpuInventory { devices: vec![] }
          }
        }
      };

      if launch_manual {
        for cfg in &mut inventory.devices {
          cfg.block_size = manual_block;
          cfg.grid_size = manual_grid;
          cfg.parallel_keys = selected_parallel_keys;

          cfg.chunk_size = if manual_chunk > 0 {
            manual_chunk
          } else {
            (manual_grid as u128)
              .saturating_mul(manual_block as u128)
              .saturating_mul(selected_parallel_keys as u128)
              .saturating_mul(manual_chunk_mult as u128)
          };
        }
      }

      if !inventory.devices.is_empty() {
        crate::app_log!("\n------- Detected {} CUDA device(s) -------", inventory.devices.len());
        crate::app_log!(
          "Selected kernel: type={}  size={}  parallel_keys={}  launch={}",
          selected_kernel_type.display_name(),
          selected_kernel_size,
          selected_parallel_keys,
          if launch_manual { "MANUAL" } else { "AUTO" }
        );
        for cfg in &inventory.devices {
          crate::app_log!(
            "  [{}] {:<32}  SMs={:2}  clock={}MHz  block={:4}  grid={:4}  keys/th={:4}  chunk={:#X}  type={}  size={}",
            cfg.device_id,
            cfg.name,
            cfg.sm_count,
            cfg.clock_mhz,
            cfg.block_size,
            cfg.grid_size,
            cfg.parallel_keys,
            cfg.chunk_size,
            cfg.kernel_type.display_name(),
            cfg.kernel_size
          );
        }
      } else if device != "CPU" {
        crate::app_log!("No usable GPU found - falling back to CPU-only workers");
      }

      let effective_device = if (device == "GPU" || device == "Both") && inventory.devices.is_empty() {
        "CPU"
      } else {
        device.as_str()
      };

      let cpu_threads = match effective_device {
        "CPU" | "Both" => num_cpus::get(),
        _ => 0,
      };

      let gpu_count = match effective_device {
        "GPU" | "Both" => inventory.devices.len(),
        _ => 0,
      };

      let total_workers = cpu_threads + gpu_count;

      {
        let mut dash = DASHBOARD.lock().unwrap();
        dash.clear();
        dash.resize(total_workers, ThreadStatus::default());

        for i in 0..gpu_count {
          dash[cpu_threads + i].worker_type = WorkerType::Gpu;
        }
      }

      {
        let mut gui = dash_state.lock().unwrap();

        KeyHunterApp::fill_workers_table(&mut gui.workers, effective_device, &inventory);

        gui.target = puzzle_address.to_string();
        gui.range = format!("{} - {}", start.to_hex(), end.to_hex());
        gui.mode = format!("{:?}", solving_mode);
        gui.found = false;
        gui.found_key.clear();
        gui.status_message.clear();
      }

      let (ready_tx, ready_rx) = mpsc::channel::<()>();
      let (cmd_tx, cmd_rx) = mpsc::channel::<AllocCommand>();

      let allocator_handle = spawn_allocator_thread(puzzle_number, solving_mode, alloc.clone(), cmd_rx, total_keys, shutdown.clone(), ready_tx);

      let _ = ready_rx.recv();

      let mut worker_handles: Vec<JoinHandle<()>> = Vec::new();

      if cpu_threads > 0 {
        let target_pubkey = puzzle_info.pubkey_compressed();
        worker_handles.push(spawn_cpu_workers(
          puzzle_number,
          puzzle_address,
          puzzle_hash160,
          target_pubkey,
          range_start,
          n_bits,
          n_mask,
          solving_mode,
          pcg,
          cmd_tx.clone(),
          shutdown.clone(),
          cpu_threads,
          intensity.clone(),
        ));
      }

      for (gpu_idx, gpu_cfg) in inventory.devices.into_iter().enumerate() {
        let dash_index = cpu_threads + gpu_idx;

        let target_pubkey = puzzle_info.pubkey_compressed();
        worker_handles.push(spawn_gpu_worker(
          puzzle_number,
          range_start,
          n_bits,
          n_mask,
          puzzle_hash160,
          target_pubkey,
          pcg,
          solving_mode,
          cmd_tx.clone(),
          shutdown.clone(),
          dash_index,
          intensity.clone(),
          gpu_cfg,
        ));
      }

      while !shutdown.load(Ordering::Relaxed) {
        let low = TOTAL_CHECKED_LOW.load(Ordering::Relaxed);
        let high = TOTAL_CHECKED_HIGH.load(Ordering::Relaxed);
        let total = (high as u128) << 64 | (low as u128);
        let elapsed = global_start.elapsed().as_secs_f64();
        let generation_speed = if elapsed > 0.1 { total as f64 / elapsed / 1000.0 } else { 0.0 };

        let key_was_found = FOUND.load(Ordering::Relaxed);
        let found_key = if key_was_found {
          FOUND_KEY_HEX.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
        } else {
          String::new()
        };

        let cpu_clock = format_clock_ghz(detect_cpu_freq_mhz());
        let gpu_clocks = detect_gpu_clocks_mhz();

        if let Ok(mut gui) = dash_state.lock() {
          gui.total_checked = total as u64;
          gui.generation_speed = generation_speed;
          gui.progress = if total_keys.hi != 0 {
            let approx_total = (total_keys.hi as f64) * (u128::MAX as f64) + (total_keys.lo as f64);
            if approx_total > 0.0 {
              (total as f64 / approx_total) * 100.0
            } else {
              0.0
            }
          } else if total_keys.lo > 0 {
            (total as f64 / total_keys.lo as f64) * 100.0
          } else {
            0.0
          };

          let secs = global_start.elapsed().as_secs();
          gui.elapsed = format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60);

          if let Ok(dash) = DASHBOARD.lock() {
            for (i, status) in dash.iter().enumerate() {
              if i >= gui.workers.len() {
                break;
              }
              let row = &mut gui.workers[i];
              row.index_hex = status.index_hex.clone();
              row.key_hex = status.key_hex.clone();
              row.checked = status.checked as u64;
              row.speed_kps = status.speed_kps;

              if row.gpu_id.is_none() {
                row.clock = cpu_clock.clone();
              } else if let Some(id) = row.gpu_id
                && let Some((_, mhz)) = gpu_clocks.iter().find(|(gid, _)| *gid == id)
              {
                row.clock = format_clock_ghz(*mhz as f64);
              }
            }
          }

          if key_was_found {
            gui.found = true;
            gui.found_key = found_key;
            gui.status_message = "PRIVATE KEY FOUND".to_string();
            shutdown.store(true, Ordering::Relaxed);
          }
        }
      }

      if FOUND.load(Ordering::Relaxed)
        && let Ok(mut gui) = dash_state.lock()
      {
        gui.found = true;
        if gui.found_key.is_empty()
          && let Ok(slot) = FOUND_KEY_HEX.lock()
          && let Some(k) = slot.as_ref()
        {
          gui.found_key = k.clone();
        }
        if gui.status_message.is_empty() {
          gui.status_message = "PRIVATE KEY FOUND".to_string();
        }
      }

      let _ = cmd_tx.send(AllocCommand::Stop);

      for h in worker_handles {
        let _ = h.join();
      }
      let _ = allocator_handle.join();
    });

    self.worker_handles.push(handle);
  }

  fn stop_workers(&mut self) {
    if self.benchmark_running {
      request_bench_cancel();
      if let Ok(mut g) = self.state.lock() {
        g.benchmark_status = "Cancelling benchmark...".to_string();
      }

      return;
    }

    if !self.running {
      return;
    }

    self.shutdown.store(true, Ordering::Relaxed);
    self.running = false;
    clear_all_mode_chunk_budget();

    std::thread::sleep(Duration::from_millis(50));

    for h in self.worker_handles.drain(..) {
      let _ = h.join();
    }

    TOTAL_CHECKED_LOW.store(0, Ordering::Relaxed);
    TOTAL_CHECKED_HIGH.store(0, Ordering::Relaxed);
  }

  fn start_benchmark(&mut self) {
    if self.running || self.benchmark_running {
      return;
    }

    if self.device == "CPU" || !self.has_gpu {
      if let Ok(mut g) = self.state.lock() {
        g.last_error = "GPU Benchmark requires Device = GPU or Both".to_string();
      }
      return;
    }

    if !self.kernels_loaded {
      if let Ok(mut g) = self.state.lock() {
        g.last_error = "Load a kernels.OWL package first (Load kernels button).".to_string();
      }
      return;
    }

    clear_gpu_error();
    clear_bench_cancel();

    let mut kernel_list: Vec<(KernelType, String)> = Vec::new();
    if self.bench_all_kernels {
      for &ty in KernelType::ALL {
        for s in self.registry.sizes_for(ty) {
          kernel_list.push((ty, s.clone()));
        }
      }
    } else {
      if !self.registry.has(self.kernel.kernel_type, &self.kernel.kernel_size) {
        if let Ok(mut g) = self.state.lock() {
          g.last_error = format!(
            "Selected kernel {}-{} is not inside the loaded package.",
            self.kernel.kernel_type.as_str(),
            self.kernel.kernel_size
          );
        }
        return;
      }
      kernel_list.push((self.kernel.kernel_type, self.kernel.kernel_size.clone()));
    }

    if kernel_list.is_empty() {
      if let Ok(mut g) = self.state.lock() {
        g.last_error = "No CUBIN kernels available in the loaded package to benchmark.".to_string();
      }
      return;
    }

    let n_ker = kernel_list.len();
    let n_gpu = self.gpu_count.max(1);
    let secs = self.bench_seconds.max(1);

    let trials_per_ker = 24;
    let est_min = (n_ker * n_gpu * trials_per_ker * secs as usize).div_ceil(60);

    if let Ok(mut g) = self.state.lock() {
      g.last_error.clear();
      g.benchmark_status = format!(
        "Benchmark starting... {} kernelxsize(s) x {} GPU(s) (CUBIN), hill-climb ≈{} trials/kernel x {}s ≈ {} min  [GPU0 → GPU1 → ...]",
        n_ker,
        n_gpu,
        trials_per_ker,
        secs,
        est_min.max(1)
      );
      g.benchmark_best.clear();
      g.benchmark_speed_kps = 0.0;
      g.generation_speed = 0.0;
      g.benchmark_best_kernel = None;
      g.benchmark_best_size = None;
      g.benchmark_best_block = None;
      g.benchmark_best_grid = None;
      g.benchmark_best_keys = None;
      g.benchmark_best_chunk = None;
    }

    self.speed_history.clear();
    self.max_speed_ever = 0.0;
    self.last_speed_sample = Instant::now();
    self.graph_view_start = 0.0;
    self.graph_view_span = 1.0;

    self.benchmark_running = true;

    let block = self.block_size;
    let grid = self.grid_size;
    let pkeys = self.parallel_keys;
    let chunk_mult = self.chunk_multiplier.max(1);
    let dash = self.state.clone();

    let handle = std::thread::spawn(move || {
      let progress = Box::new({
        let dash = dash.clone();
        move |_done: usize, _total: usize, msg: &str, speed_mkeys: Option<f64>| {
          if let Ok(mut g) = dash.lock() {
            g.benchmark_status = msg.to_string();
            if let Some(m) = speed_mkeys {
              let kps = m * 1000.0;
              g.benchmark_speed_kps = kps;
              g.generation_speed = kps;
            }
          }
        }
      });

      match run_gpu_benchmark(&kernel_list, block, grid, pkeys, chunk_mult, secs as u64, Some(progress)) {
        Ok(best) => {
          let line = format!(
            "Best: {} size={}  block={}  grid={}  keys={}  chunk={}x   {:.2} Mkeys/s",
            best.kernel_type.display_name(),
            best.kernel_size,
            best.block_size,
            best.grid_size,
            best.parallel_keys,
            best.chunk_multiplier,
            best.speed_mkeys
          );
          crate::app_log!("[Bench] {}", line);
          if let Ok(mut g) = dash.lock() {
            g.benchmark_best = line;
            g.benchmark_status = "Benchmark finished".to_string();
            g.benchmark_speed_kps = best.speed_mkeys * 1000.0;
            g.generation_speed = best.speed_mkeys * 1000.0;
            g.benchmark_best_kernel = Some(best.kernel_type.as_str().to_string());
            g.benchmark_best_size = Some(best.kernel_size.clone());
            g.benchmark_best_block = Some(best.block_size);
            g.benchmark_best_grid = Some(best.grid_size);
            g.benchmark_best_keys = Some(best.parallel_keys);
            g.benchmark_best_chunk = Some(best.chunk_multiplier);
          }
        }
        Err(e) => {
          if e.contains("cancelled") {
            if let Ok(mut g) = dash.lock() {
              g.benchmark_status = format!("Benchmark cancelled. {}", e);
            }
          } else {
            report_err_to_dash(&dash, &e);
            if let Ok(mut g) = dash.lock() {
              g.benchmark_status = format!("Benchmark failed: {}", e);
            }
          }
        }
      }
    });

    self.benchmark_handle = Some(handle);
  }
}

fn report_err_to_dash(
  dash: &Arc<Mutex<DashboardState>>,
  msg: &str,
) {
  crate::report_gpu_error(msg);
  if let Ok(mut g) = dash.lock() {
    g.last_error = msg.to_string();
  }
}

fn parse_best_line(line: &str) -> Option<(u32, u32, u32, u32, String, Option<String>)> {
  let mut block = None;
  let mut grid = None;
  let mut keys = None;
  let mut mult = None;
  let mut ksize: Option<String> = None;

  let parts: Vec<&str> = line.split_whitespace().collect();

  let mut kty = String::new();
  if let Some(best_i) = parts.iter().position(|p| *p == "Best:") {
    let mut name_parts = Vec::new();
    for part in parts.iter().skip(best_i + 1) {
      if part.contains('=') {
        break;
      }
      name_parts.push(*part);
    }
    kty = name_parts.join(" ");
  }

  for part in &parts {
    if let Some(v) = part.strip_prefix("block=") {
      block = v.parse().ok();
    } else if let Some(v) = part.strip_prefix("grid=") {
      grid = v.parse().ok();
    } else if let Some(v) = part.strip_prefix("keys=") {
      keys = v.parse().ok();
    } else if let Some(v) = part.strip_prefix("chunk=") {
      let v = v.trim_end_matches('x');
      mult = v.parse().ok();
    } else if let Some(v) = part.strip_prefix("size=") {
      ksize = Some(v.to_string());
    }
  }
  match (block, grid, keys, mult) {
    (Some(b), Some(g), Some(k), Some(m)) => Some((b, g, k, m, kty, ksize)),
    _ => None,
  }
}

impl eframe::App for KeyHunterApp {
  fn ui(
    &mut self,
    ui: &mut egui::Ui,
    _frame: &mut eframe::Frame,
  ) {
    tray_store_egui_ctx(ui.ctx());

    self.handle_tray_events(ui.ctx());
    self.poll_idle_solving();
    self.sync_tray_icon();

    let minimized = ui.ctx().input(|i| i.viewport().minimized.unwrap_or(false));
    if minimized && self.window_visible {
      self.window_visible = false;
    }

    if self.tray_enabled && !self.window_visible {
      ui.ctx().request_repaint_after(Duration::from_millis(500));
    }

    if ui.ctx().input(|i| i.viewport().close_requested()) {
      if self.force_quit {
        self.stop_workers();
        self.shutdown.store(true, Ordering::Relaxed);
        self.tray_icon = None;
      } else if self.hide_on_close {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.hide_to_tray(ui.ctx());
        crate::app_log!("[TRAY] Window hidden (Hide on close is ON)");
      } else {
        self.force_quit = true;
        self.stop_workers();
        self.shutdown.store(true, Ordering::Relaxed);
        self.tray_icon = None;
      }
    }

    if let Some(path) = self.pending_kernel_autoload.take()
      && matches!(self.device.as_str(), "GPU" | "Both")
      && !self.kernels_loading
    {
      crate::app_log!("[SETTINGS] Auto-loading kernel package: {path}");
      self.begin_kernel_load(std::path::PathBuf::from(path), ui.ctx().clone());
    }

    self.poll_kernel_load();
    if self.kernels_loading {
      ui.ctx().request_repaint_after(Duration::from_millis(50));
    }

    self.ingest_pending_logs();

    if ui.ctx().input(|i| i.key_pressed(egui::Key::F12)) {
      self.show_log_window = !self.show_log_window;
    }

    if ui.ctx().input(|i| i.key_pressed(egui::Key::F10)) {
      self.show_settings_window = !self.show_settings_window;
    }

    if ui.ctx().input(|i| i.key_pressed(egui::Key::F2)) {
      if self.show_buy_window {
        self.show_buy_window = false;
      } else {
        self.open_buy_window();
      }
    }

    if ui.ctx().input(|i| i.key_pressed(egui::Key::F1)) {
      self.show_help_window = !self.show_help_window;
    }

    let mut start_clicked = false;
    let mut stop_clicked = false;
    let mut bench_clicked = false;

    if let Some(err) = take_gpu_error(true) {
      if let Ok(mut g) = self.state.lock() {
        g.last_error = err;
      }
      if self.running {
        self.stop_workers();
      }
    }

    if self.running && !self.worker_handles.is_empty() && self.worker_handles.iter().all(|h| h.is_finished()) {
      self.running = false;
      for h in self.worker_handles.drain(..) {
        let _ = h.join();
      }
    }

    if self.benchmark_running {
      if let Some(handle) = self.benchmark_handle.take() {
        if handle.is_finished() {
          let _ = handle.join();
          self.benchmark_running = false;

          let applied = self.state.lock().ok().and_then(|g| {
            match (
              g.benchmark_best_kernel.clone(),
              g.benchmark_best_size.clone(),
              g.benchmark_best_block,
              g.benchmark_best_grid,
              g.benchmark_best_keys,
              g.benchmark_best_chunk,
            ) {
              (Some(k), Some(sz), Some(b), Some(gr), Some(keys), Some(ch)) => Some((k, sz, b, gr, keys, ch)),
              _ => None,
            }
          });

          if let Some((k_name, k_size, block, grid, keys, chunk)) = applied {
            self.block_size = block;
            self.grid_size = grid;
            self.parallel_keys = keys;
            self.chunk_multiplier = chunk;

            if self.gpu_count <= 1 {
              self.launch_manual = true;
            }

            if let Some(ty) = KernelType::from_string(&k_name).or_else(|| KernelType::ALL.iter().copied().find(|t| t.display_name() == k_name)) {
              self.kernel.kernel_type = ty;
              self.kernel.kernel_size = k_size;
              self.refresh_kernel();
            }
          } else {
            let best_line = if let Ok(g) = self.state.lock() {
              let s = g.benchmark_best.clone();
              if s.is_empty() { None } else { Some(s) }
            } else {
              None
            };

            if let Some(best_line) = best_line
              && let Some((block, grid, keys, mult, kty_name, ksize)) = parse_best_line(&best_line)
            {
              self.block_size = block;
              self.grid_size = grid;
              self.parallel_keys = keys;
              self.chunk_multiplier = mult;

              if self.gpu_count <= 1 {
                self.launch_manual = true;
              }

              if let Some(ksize) = ksize
                && let Some(ty) =
                  KernelType::from_string(&kty_name).or_else(|| KernelType::ALL.iter().copied().find(|t| t.display_name() == kty_name))
              {
                self.kernel.kernel_type = ty;
                self.kernel.kernel_size = ksize;
                self.refresh_kernel();
              }
            }
          }
        } else {
          self.benchmark_handle = Some(handle);
        }
      } else {
        self.benchmark_running = false;
      }
    }

    egui::Panel::top("header_panel").resizable(false).show(ui, |ui| {
      let group_frame = |color: egui::Color32| {
        egui::Frame::NONE
          .stroke(egui::Stroke::new(1.5, color))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::symmetric(MARGIN_WIDE as i8, MARGIN_NARROW as i8))
          .outer_margin(egui::Margin::ZERO)
          .fill(egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 18))
      };

      let fixed_frame = |ui: &mut egui::Ui, color: egui::Color32, add: &mut dyn FnMut(&mut egui::Ui)| {
        ui.allocate_ui_with_layout(egui::vec2(FRAME_W, FRAME_H), egui::Layout::top_down(egui::Align::Min), |ui| {
          group_frame(color).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = MARGIN_NARROW;
            add(ui);
          });
        });
      };

      let show_gpu = matches!(self.device.as_str(), "GPU" | "Both") && self.kernels_loaded;
      let show_bench = show_gpu && self.show_benchmark_frame;

      let n_frames = if !show_gpu {
        2
      } else if show_bench {
        4
      } else {
        3
      };
      let frames_total_w = n_frames as f32 * FRAME_W + (n_frames - 1) as f32 * MARGIN_WIDE;

      ui.vertical_centered(|ui| {
        #[cfg(feature = "test_vectors")]
        let title = "Bitcoin Puzzle Test";

        #[cfg(not(feature = "test_vectors"))]
        let title = "Bitcoin Puzzle";

        ui.heading(egui::RichText::new(title).color(COL_ACCENT).size(28.0).strong());
        ui.add_space(MARGIN_NARROW);
      });

      ui.vertical_centered(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(frames_total_w, 25.0), egui::Layout::top_down(egui::Align::Center), |ui| {
          let puzzle_enabled = !self.running && !self.benchmark_running;

          ui.add_enabled_ui(puzzle_enabled, |ui| {
            let mut selected = self.selected_puzzle;
            let selected_label = if selected == ALL_PUZZLES {
              "ALL".to_string()
            } else {
              match puzzle_by_number(selected) {
                Some(info) if info.has_pubkey() => format!("#{}*", selected),
                _ => format!("#{}", selected),
              }
            };

            egui::ComboBox::from_id_salt("puzzle_select")
              .selected_text(selected_label)
              .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, ALL_PUZZLES, "ALL");
                for p in active_puzzles() {
                  let label = if p.has_pubkey() {
                    format!("#{}*", p.number)
                  } else {
                    format!("#{}", p.number)
                  };
                  ui.selectable_value(&mut selected, p.number, label);
                }
              });

            if selected != self.selected_puzzle {
              self.selected_puzzle = selected;
              if selected == ALL_PUZZLES {
                if let Ok(mut gui) = self.state.lock() {
                  gui.target = "ALL unsolved puzzles (#71-#160)".to_string();
                  gui.range = "round-robin batches across remaining puzzles".to_string();
                  gui.all_progress = 0.0;
                  gui.all_current_puzzle = 0;
                  gui.all_total_puzzles = remaining_puzzles().len();
                }
              } else if let Some(info) = puzzle_by_number(selected)
                && let Ok(mut gui) = self.state.lock()
              {
                gui.target = if info.has_pubkey() {
                  format!("{}  [pubkey]", info.address)
                } else {
                  info.address.to_string()
                };
                gui.range = format!("{} - {}", info.range_start().to_hex(), info.range_end().to_hex());
                gui.all_progress = 0.0;
                gui.all_current_puzzle = 0;
                gui.all_total_puzzles = 0;
              }
            }
          });
        });
        ui.add_space(MARGIN_NARROW);
      });

      ui.vertical_centered(|ui| {
        ui.allocate_ui_with_layout(
          egui::vec2(frames_total_w, FRAME_H),
          egui::Layout::left_to_right(egui::Align::Center),
          |ui| {
            ui.horizontal_top(|ui| {
              fixed_frame(ui, COL_ACCENT, &mut |ui| {
                ui.set_height(FRAME_H);
                ui.label(egui::RichText::new("Solver").color(COL_ACCENT).strong());
                ui.add_space(MARGIN_NARROW);

                ui.horizontal(|ui| {
                  let can_start = !self.running && !self.benchmark_running && (self.device == "CPU" || self.kernels_loaded);
                  ui.add_enabled_ui(can_start, |ui| {
                    let t = egui::RichText::new("  Start  ").color(COL_ACCENT).strong();
                    if ui.button(t).clicked() {
                      start_clicked = true;
                    }
                  });

                  ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(self.running || self.benchmark_running, |ui| {
                      let c = if self.benchmark_running { COL_BENCH } else { COL_ACCENT };
                      let t = egui::RichText::new("  Stop  ").color(c).strong();
                      if ui
                        .button(t)
                        .on_hover_text(if self.benchmark_running {
                          "Cancel the running GPU benchmark (stops after current trial)"
                        } else {
                          "Stop the key search"
                        })
                        .clicked()
                      {
                        stop_clicked = true;
                      }
                    });
                  });
                });

                ui.add_space(MARGIN_NARROW);

                ui.label(egui::RichText::new("Device").color(COL_ACCENT));
                ui.add_enabled_ui(!self.running, |ui| {
                  let prev = self.device.clone();
                  let device_color = match self.device.as_str() {
                    "CPU" => COL_CPU,
                    "GPU" => COL_GPU,
                    _ => COL_ACCENT,
                  };
                  egui::ComboBox::from_id_salt("device_combo")
                    .width(ui.available_width())
                    .selected_text(egui::RichText::new(&self.device).color(device_color))
                    .show_ui(ui, |ui| {
                      ui.selectable_value(&mut self.device, "CPU".to_string(), egui::RichText::new("CPU").color(COL_CPU));
                      if self.has_gpu {
                        ui.selectable_value(&mut self.device, "GPU".to_string(), egui::RichText::new("GPU").color(COL_GPU));
                        ui.selectable_value(&mut self.device, "Both".to_string(), egui::RichText::new("Both").color(COL_ACCENT));
                      }
                    });
                  if self.device != prev && !self.running {
                    let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(self.parallel_keys);
                    let inventory = if self.device == "CPU" || !self.has_gpu {
                      GpuInventory { devices: vec![] }
                    } else {
                      match detect_gpus(self.kernel.kernel_type, &self.kernel.kernel_size, ks) {
                        Ok(inv) => inv,
                        Err(e) => {
                          crate::app_log!("[WARN] GPU detection on device change failed: {}", e);
                          GpuInventory { devices: vec![] }
                        }
                      }
                    };
                    if let Ok(mut gui) = self.state.lock() {
                      Self::fill_workers_table(&mut gui.workers, &self.device, &inventory);
                    }
                  }
                });

                ui.label(egui::RichText::new("Mode").color(COL_ACCENT));
                let mode_text = match self.solving_mode {
                  SearchMode::Sequence => "Sequence",
                  SearchMode::Pcg => "PCG",
                };

                ui.add_enabled_ui(!self.running, |ui| {
                  let prev_mode = self.solving_mode;
                  egui::ComboBox::from_id_salt("mode_combo")
                    .width(ui.available_width())
                    .selected_text(egui::RichText::new(mode_text).color(COL_ACCENT))
                    .show_ui(ui, |ui| {
                      ui.selectable_value(&mut self.solving_mode, SearchMode::Sequence, "Sequence");
                      ui.selectable_value(&mut self.solving_mode, SearchMode::Pcg, "PCG");
                    });

                  if self.solving_mode != prev_mode {
                    let reset = AllCycleState { index: 0, sweep: 0 };
                    save_all_cycle_state(&reset);
                    if let Ok(mut gui) = self.state.lock() {
                      gui.all_progress = 0.0;
                      gui.all_current_puzzle = 0;
                      gui.all_sweep = 0;
                      gui.all_total_puzzles = remaining_puzzles().len();
                      gui.status_message = format!("Mode → {:?} - ALL cycle reset to first puzzle", self.solving_mode);
                    }
                    crate::app_log!("[ALL] Mode switched {:?} → {:?} - cycle reset to index 0", prev_mode, self.solving_mode);
                  }
                });

                ui.label(egui::RichText::new("Intensity").color(COL_ACCENT));
                let mut pct = (self.intensity * 100.0).round() as u32;
                if ui.add(egui::Slider::new(&mut pct, 10..=100).step_by(10.0).suffix("%")).changed() {
                  self.intensity = pct as f32 / 100.0;
                  self.intensity_atomic.store(pct, Ordering::Relaxed);
                }

                let mut turbo = self.intel_turbo;
                if ui
                  .checkbox(&mut turbo, egui::RichText::new("Intel Turbo").color(COL_CPU))
                  .on_hover_text(
                    "Enable/disable Intel Turbo Boost via\n\
                   /sys/devices/system/cpu/intel_pstate/no_turbo",
                  )
                  .changed()
                {
                  self.intel_turbo = turbo;
                  Self::set_intel_turbo(turbo);
                }

                let wants_gpu = matches!(self.device.as_str(), "GPU" | "Both");
                if wants_gpu {
                  ui.horizontal(|ui| {
                    ui.add_space(MARGIN_NARROW);

                    ui.add_enabled_ui(!self.running && !self.kernels_loading, |ui| {
                      let label = if self.kernels_loaded { "Reload kernels" } else { "Load kernels" };
                      if ui
                        .button(egui::RichText::new(label).color(COL_GPU).strong())
                        .on_hover_text("Select a machine-locked kernels.OWL package")
                        .clicked()
                      {
                        self.open_kernels_dialog(ui.ctx().clone());
                      }
                    });

                    if !self.kernels_loaded {
                      ui.add_space(MARGIN_NARROW);
                      if ui
                        .button(egui::RichText::new("Buy Kernel Pack").color(COL_ACCENT).strong())
                        .on_hover_text(
                          "Open the Buy window.\n\
                       Generate a short encrypted payload that you send to the seller\n\
                       so a machine-locked kernels.OWL can be built for this PC only.",
                        )
                        .clicked()
                      {
                        self.open_buy_window();
                      }
                    }
                  });
                }
              });

              if show_gpu {
                fixed_frame(ui, COL_GPU, &mut |ui| {
                  ui.set_height(FRAME_H);
                  ui.label(egui::RichText::new("GPU").color(COL_GPU).strong());
                  ui.add_space(MARGIN_NARROW);

                  ui.horizontal(|ui| {
                    if !self.kernels_status.is_empty() && !self.kernels_loading {
                      let color = if self.kernels_loaded { COL_GPU } else { COL_ERROR };
                      let fmt = "cubin";

                      ui.label(egui::RichText::new(format!("{} {}", self.kernels_status, fmt)).color(color))
                        .on_hover_text("Format is determined by the loaded kernels.OWL package.\nUser cannot change it.");
                    } else if self.kernels_loading {
                      ui.label(
                        egui::RichText::new("Loading package… (see progress bar above devices)")
                          .color(COL_ACCENT)
                          .small(),
                      );
                    }
                  });

                  ui.add_enabled_ui(!self.running, |ui| {
                    egui::ComboBox::from_id_salt("kernel_type_combo")
                      .width(ui.available_width())
                      .selected_text(egui::RichText::new(self.kernel.kernel_type.display_name()).color(COL_GPU))
                      .show_ui(ui, |ui| {
                        for &ty in &[KernelType::ChudnovskyAddG, KernelType::ChudnovskyMeloniCoZ] {
                          if !self.registry.sizes_for(ty).is_empty()
                            && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed()
                          {
                            self.on_kernel_type_changed();
                          }
                        }

                        ui.separator();

                        for &ty in &[
                          KernelType::Jacobian,
                          KernelType::JacobianCoZMeloni,
                          KernelType::JacobianLinearGlv,
                          KernelType::JacobianEndoGlv,
                          KernelType::JacobianKaratsuba,
                        ] {
                          if !self.registry.sizes_for(ty).is_empty()
                            && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed()
                          {
                            self.on_kernel_type_changed();
                          }
                        }

                        ui.separator();

                        for &ty in &[
                          KernelType::ProjectiveAddGLazy,
                          KernelType::ProjectiveBaby,
                          KernelType::ProjectiveLatticeGlv,
                          KernelType::ProjectiveRestrictedGlv,
                          KernelType::ProjectiveXOnlyRegG,
                          KernelType::ProjectiveXZBatch,
                          KernelType::ProjectiveZFactorBatch,
                        ] {
                          if !self.registry.sizes_for(ty).is_empty()
                            && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed()
                          {
                            self.on_kernel_type_changed();
                          }
                        }

                        ui.separator();

                        if !self.registry.sizes_for(KernelType::Xyzz).is_empty()
                          && ui
                            .selectable_value(&mut self.kernel.kernel_type, KernelType::Xyzz, KernelType::Xyzz.display_name())
                            .changed()
                        {
                          self.on_kernel_type_changed();
                        }

                        ui.separator();

                        if !self.registry.sizes_for(KernelType::XzDifferential).is_empty()
                          && ui
                            .selectable_value(
                              &mut self.kernel.kernel_type,
                              KernelType::XzDifferential,
                              KernelType::XzDifferential.display_name(),
                            )
                            .changed()
                        {
                          self.on_kernel_type_changed();
                        }
                      });
                  });

                  ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                      ui.label(egui::RichText::new("Size").color(COL_GPU));
                      ui.add_enabled_ui(!self.running, |ui| {
                        let sizes: Vec<String> = self.registry.sizes_for(self.kernel.kernel_type).to_vec();
                        egui::ComboBox::from_id_salt("kernel_size_combo")
                          .width(110.0)
                          .selected_text(egui::RichText::new(&self.kernel.kernel_size).color(COL_GPU))
                          .show_ui(ui, |ui| {
                            for s in sizes {
                              if ui.selectable_value(&mut self.kernel.kernel_size, s.clone(), s.as_str()).changed() {
                                self.parallel_keys = s.parse().unwrap_or(1024);
                                self.refresh_kernel();
                              }
                            }
                          });
                      });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                      ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Keys").color(COL_GPU));
                        ui.add_enabled_ui(!self.running, |ui| {
                          ui.horizontal(|ui| {
                            let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(1024);
                            let min_pk = (ks / 8).max(8);
                            let max_pk = (ks * 8).max(4096);
                            let step = if ui.input(|i| i.modifiers.shift) { 64 } else { 8 };
                            if ui
                              .add_enabled(self.parallel_keys > min_pk, egui::Button::new(egui::RichText::new("-").color(COL_GPU)))
                              .on_hover_text("Click: −8\nShift+click: −64")
                              .clicked()
                            {
                              self.parallel_keys = self.parallel_keys.saturating_sub(step).max(min_pk);
                              self.parallel_keys = (self.parallel_keys / 8) * 8;
                              if self.parallel_keys < min_pk {
                                self.parallel_keys = min_pk;
                              }
                            }
                            let display = self.parallel_keys.clamp(min_pk, max_pk);
                            ui.monospace(egui::RichText::new(format!("{display:4}")).color(COL_GPU));
                            if ui
                              .add_enabled(self.parallel_keys < max_pk, egui::Button::new(egui::RichText::new("+").color(COL_GPU)))
                              .on_hover_text("Click: +8\nShift+click: +64")
                              .clicked()
                            {
                              self.parallel_keys = (self.parallel_keys + step).min(max_pk);
                              self.parallel_keys = (self.parallel_keys / 8) * 8;
                              if self.parallel_keys > max_pk {
                                self.parallel_keys = max_pk;
                              }
                            }
                          });
                        });
                      });
                    });
                  });

                  ui.label(egui::RichText::new("Launch mode").color(COL_GPU));
                  let multi_gpu = self.gpu_count > 1;
                  if multi_gpu {
                    self.launch_manual = false;
                  }
                  ui.add_enabled_ui(!self.running && !multi_gpu, |ui| {
                    let launch_label = if self.launch_manual { "Manual" } else { "Auto" };
                    let response = egui::ComboBox::from_id_salt("launch_mode_combo")
                      .width(ui.available_width())
                      .selected_text(egui::RichText::new(launch_label).color(COL_GPU))
                      .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.launch_manual, false, "Auto");
                        ui.selectable_value(&mut self.launch_manual, true, "Manual");
                      })
                      .response;
                    if multi_gpu {
                      response.on_disabled_hover_text(
                        "Multi-GPU detected: Manual launch is disabled in v1.\n\
                         Each GPU uses Auto parameters derived from its SM count.\n\
                         Per-GPU Manual tuning will be supported in v2.",
                      );
                    }
                  });
                  if multi_gpu {
                    ui.label(
                      egui::RichText::new("Multi-GPU: Auto only (per-GPU Manual in v2)")
                        .small()
                        .italics()
                        .color(egui::Color32::from_gray(160)),
                    );
                  }

                  ui.horizontal(|ui| {
                    let three_size = (ui.available_width() / 3.0) - MARGIN_NARROW;

                    ui.vertical(|ui| {
                      ui.label(egui::RichText::new("Block").color(COL_GPU));
                      ui.add_enabled_ui(!self.running && self.launch_manual, |ui| {
                        egui::ComboBox::from_id_salt("block_size_combo")
                          .width(three_size)
                          .selected_text(egui::RichText::new(format!("{}", self.block_size)).color(COL_GPU))
                          .show_ui(ui, |ui| {
                            for &b in BLOCK_SIZES {
                              ui.selectable_value(&mut self.block_size, b, format!("{b}"));
                            }
                          });
                      });
                    });

                    ui.vertical(|ui| {
                      ui.label(egui::RichText::new("Grid").color(COL_GPU));
                      ui.add_enabled_ui(!self.running && self.launch_manual, |ui| {
                        egui::ComboBox::from_id_salt("grid_size_combo")
                          .width(three_size)
                          .selected_text(egui::RichText::new(format!("{}", self.grid_size)).color(COL_GPU))
                          .show_ui(ui, |ui| {
                            for &g in GRID_SIZES {
                              ui.selectable_value(&mut self.grid_size, g, format!("{g}"));
                            }
                          });
                      });
                    });

                    ui.vertical(|ui| {
                      ui.label(egui::RichText::new("Chunk").color(COL_GPU));
                      ui.add_enabled_ui(!self.running && self.launch_manual, |ui| {
                        egui::ComboBox::from_id_salt("chunk_mult_combo")
                          .width(three_size)
                          .selected_text(egui::RichText::new(format!("{}x", self.chunk_multiplier)).color(COL_GPU))
                          .show_ui(ui, |ui| {
                            for &m in CHUNK_MULTIPLIERS {
                              ui.selectable_value(&mut self.chunk_multiplier, m, format!("{m}x"));
                            }
                          })
                          .response
                          .on_hover_text("Full GPU launches per allocator chunk (1x = blockxgridxkeys).");
                      });
                    });
                  });

                  ui.checkbox(&mut self.show_benchmark_frame, egui::RichText::new("Benchmark GPU").color(COL_GPU))
                    .on_hover_text("Show the Benchmark frame next to GPU");
                });
              }

              if show_bench {
                fixed_frame(ui, COL_BENCH, &mut |ui| {
                  ui.set_height(FRAME_H);
                  ui.label(egui::RichText::new("Benchmark").color(COL_BENCH).strong());
                  ui.add_space(MARGIN_WIDE);

                  ui.add_enabled_ui(!self.running && !self.benchmark_running && self.has_gpu && self.kernels_loaded, |ui| {
                    let t = egui::RichText::new("  Run Benchmark  ").color(COL_BENCH).strong();
                    if ui
                      .button(t)
                      .on_hover_text(
                        "Hill-climb per kernelxsize from the loaded kernels.OWL package.\n\
                         Best params applied when done.\n\
                         Stop cancels between trials.\n\
                         Requires a loaded package (Load kernels).",
                      )
                      .clicked()
                    {
                      bench_clicked = true;
                    }
                  });

                  if !self.kernels_loaded {
                    ui.label(egui::RichText::new("Load kernels.OWL first").color(COL_ERROR).small());
                  }

                  ui.add_space(MARGIN_WIDE);

                  ui.add_enabled_ui(!self.running && !self.benchmark_running, |ui| {
                    ui.label(egui::RichText::new("Seconds / trial").color(COL_BENCH));
                    egui::ComboBox::from_id_salt("bench_secs_combo")
                      .width(ui.available_width())
                      .selected_text(egui::RichText::new(format!("{}s", self.bench_seconds)).color(COL_BENCH))
                      .show_ui(ui, |ui| {
                        for &s in &[1u32, 2, 3, 5, 10] {
                          ui.selectable_value(&mut self.bench_seconds, s, format!("{s}s"));
                        }
                      });

                    ui.add_space(MARGIN_NARROW);
                    ui.checkbox(&mut self.bench_all_kernels, egui::RichText::new("All kernels").color(COL_BENCH))
                      .on_hover_text("Bench every kernel type x size present in the loaded package.");
                  });
                });
              }

              fixed_frame(ui, COL_CPU, &mut |ui| {
                ui.vertical_centered(|ui| {
                  ui.set_height(FRAME_H);
                  let state = self.state.lock().unwrap();

                  if state.found {
                    ui.colored_label(COL_GPU, "FOUND");
                  } else if self.benchmark_running {
                    ui.colored_label(COL_BENCH, "BENCHMARKING");
                  } else if self.running {
                    ui.colored_label(COL_GPU, "RUNNING");
                  } else {
                    ui.colored_label(COL_ERROR, "STOPPED");
                  }

                  ui.separator();

                  ui.label(egui::RichText::new(format!("Speed  {}", format_speed(state.generation_speed))).color(COL_ACCENT));
                  ui.separator();
                  ui.label(egui::RichText::new(format!("Checked  {}", format_count(state.total_checked))).color(COL_ACCENT));
                  ui.separator();
                  ui.label(egui::RichText::new(format!("Progress  {:.6}%", state.progress)).color(COL_ACCENT));
                  ui.separator();
                  ui.label(egui::RichText::new(format!("Time  {}", state.elapsed)).color(COL_ACCENT));
                  ui.separator();

                  if self.kernels_loaded {
                    if let Some(ref email) = self.loaded_email {
                      ui.label(egui::RichText::new(format!("LICENSED TO: \n{}", email)).color(COL_ACCENT));
                    }

                    if let Some(ref lid) = self.loaded_machine_fp {
                      ui.label(egui::RichText::new(lid).color(COL_ACCENT).small())
                        .on_hover_text("Machine fingerprint");
                    }
                  }
                });
              });
            });
          },
        );
      });

      ui.add_space(MARGIN_NARROW);

      ui.vertical_centered(|ui| {
        let st = self.state.lock().unwrap();

        if !st.last_error.is_empty() {
          ui.colored_label(COL_ERROR, format!("GPU ERROR: {}", st.last_error));
        }
        if !st.benchmark_status.is_empty() {
          ui.colored_label(COL_BENCH, format!("Bench: {}", st.benchmark_status));
        }
        if !st.benchmark_best.is_empty() {
          ui.colored_label(COL_BENCH, &st.benchmark_best);
        }
      });

      ui.add_space(MARGIN_NARROW);
    });

    if start_clicked {
      clear_gpu_error();
      if let Ok(mut g) = self.state.lock() {
        g.last_error.clear();
      }
      let puzzle_label = if is_all_mode(self.selected_puzzle) {
        "ALL".to_string()
      } else {
        format!("#{}", self.selected_puzzle)
      };
      crate::app_log!(
        "[Start] device={} mode={:?} puzzle={} kernel={}-{}",
        self.device,
        self.solving_mode,
        puzzle_label,
        self.kernel.kernel_type.as_str(),
        self.kernel.kernel_size
      );
      self.started_by_idle = false;
      self.start_workers();
    }

    if stop_clicked {
      crate::app_log!("[Stop] device={} (solver stopped by user)", self.device);
      self.started_by_idle = false;
      self.stop_workers();
    }

    if bench_clicked {
      self.start_benchmark();
    }

    if (self.running || self.benchmark_running) && self.graph_enabled {
      let now = Instant::now();

      let interval_ms: u128 = if self.benchmark_running { 200 } else { 250 };
      if now.duration_since(self.last_speed_sample).as_millis() >= interval_ms {
        let speed = if self.benchmark_running {
          self
            .state
            .lock()
            .map(|s| {
              if s.benchmark_speed_kps > 0.0 {
                s.benchmark_speed_kps
              } else {
                s.generation_speed
              }
            })
            .unwrap_or(0.0)
        } else if self.use_live_speed {
          self
            .state
            .lock()
            .map(|s| {
              let live: f64 = s.workers.iter().map(|w| w.speed_kps).sum();
              if live > 0.0 { live } else { s.generation_speed }
            })
            .unwrap_or(0.0)
        } else {
          self.state.lock().map(|s| s.generation_speed).unwrap_or(0.0)
        };

        let clock = chrono::Local::now().format("%H:%M:%S").to_string();

        if speed > 0.0 || !self.speed_history.is_empty() {
          let plot_speed = if speed > 0.0 {
            speed
          } else {
            self.speed_history.back().map(|(s, _)| *s).unwrap_or(0.0)
          };
          self.speed_history.push_back((plot_speed, clock));

          if plot_speed > self.max_speed_ever {
            self.max_speed_ever = plot_speed;
          }
        }

        let history_secs = if self.benchmark_running {
          self.graph_history_secs.max(300)
        } else {
          self.graph_history_secs
        };
        let sample_secs = interval_ms as f64 / 1000.0;
        let max_samples = ((history_secs as f64) / sample_secs).ceil() as usize;
        while self.speed_history.len() > max_samples.max(2) {
          self.speed_history.pop_front();
        }

        self.last_speed_sample = now;
      }
    }

    {
      let state = self.state.lock().unwrap();

      if state.found && self.running {
        self.running = false;
        self.shutdown.store(true, Ordering::Relaxed);
      }
    }

    let state = self.state.lock().unwrap();

    if state.found {
      egui::Panel::top("found_banner").resizable(false).show(ui, |ui| {
        ui.add_space(MARGIN_NARROW);
        ui.colored_label(COL_GPU, "PRIVATE KEY FOUND");

        if !state.found_key.is_empty() {
          for line in state.found_key.lines() {
            ui.horizontal(|ui| {
              if let Some((label, value)) = line.split_once(": ") {
                ui.label(format!("{}:", label));
                ui.monospace(value);
                if ui.small_button("Copy").clicked() {
                  ui.ctx().copy_text(value.to_string());
                }
              } else {
                ui.monospace(line);
              }
            });
          }
        }
      });
    }

    egui::Panel::bottom("speed_graph_panel")
      .resizable(self.graph_enabled)
      .default_size(if self.graph_enabled { self.graph_panel_height } else { 50.0 })
      .min_size(120.0)
      .max_size(if self.graph_enabled { f32::INFINITY } else { 40.0 })
      .show(ui, |ui| {
        const SAMPLE_SECS: f64 = 0.25;
        const CONTROLS_H: f32 = 28.0;

        if self.graph_enabled {
          let avail_h = ui.available_height();
          if avail_h >= 50.0 {
            self.graph_panel_height = avail_h;
          }
          ui.set_min_height(avail_h);

          let plot_h_avail = (avail_h - CONTROLS_H).max(10.0);

          let history: Vec<(f64, String)> = self.speed_history.iter().cloned().collect();
          let max_ever = self.max_speed_ever;
          let total_samples = history.len();
          let total_secs = total_samples as f64 * SAMPLE_SECS;

          let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), plot_h_avail), egui::Sense::click_and_drag());
          let rect = response.rect;
          let plot_left = rect.left() + 52.0;
          let plot_right = rect.right() - 8.0;
          let plot_top = rect.top() + 4.0;
          let plot_bottom = rect.bottom() - 18.0;
          let plot_w = (plot_right - plot_left).max(1.0);
          let plot_h = (plot_bottom - plot_top).max(1.0);

          painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(18, 18, 22));

          if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 && total_samples >= 2 {
              let mouse = response.hover_pos().unwrap_or(rect.center());
              let focus = ((mouse.x - plot_left) / plot_w).clamp(0.0, 1.0) as f64;
              let old_span = self.graph_view_span;
              let zoom = if scroll > 0.0 { 0.85 } else { 1.18 };
              let new_span = (old_span * zoom).clamp(0.02, 1.0);
              let old_start = self.graph_view_start;
              let focus_abs = old_start + focus * old_span;
              let mut new_start = focus_abs - focus * new_span;
              if new_start < 0.0 {
                new_start = 0.0;
              }
              if new_start + new_span > 1.0 {
                new_start = 1.0 - new_span;
              }
              self.graph_view_span = new_span;
              self.graph_view_start = new_start.max(0.0);
            }
          }

          if response.drag_started()
            && let Some(pos) = response.interact_pointer_pos()
          {
            self.graph_drag_origin = Some((pos.x, self.graph_view_start));
          }
          if response.dragged()
            && let (Some((ox, ostart)), Some(pos)) = (self.graph_drag_origin, response.interact_pointer_pos())
          {
            let dx = (pos.x - ox) as f64 / plot_w as f64;
            let mut new_start = ostart - dx * self.graph_view_span;
            new_start = new_start.clamp(0.0, (1.0 - self.graph_view_span).max(0.0));
            self.graph_view_start = new_start;
          }
          if response.drag_stopped() {
            self.graph_drag_origin = None;
          }

          if response.double_clicked() {
            self.graph_view_start = 0.0;
            self.graph_view_span = 1.0;
          }

          let has_data = total_samples >= 2;
          let (vis_lo, vis_hi) = if has_data {
            let lo = (self.graph_view_start * (total_samples - 1) as f64).floor() as usize;
            let hi = ((self.graph_view_start + self.graph_view_span) * (total_samples - 1) as f64).ceil() as usize;
            (lo.min(total_samples - 1), hi.min(total_samples - 1).max(lo + 1))
          } else {
            (0, 0)
          };

          let visible: Vec<(usize, f64, String)> = if has_data {
            history[vis_lo..=vis_hi]
              .iter()
              .enumerate()
              .map(|(i, (s, clock))| (vis_lo + i, *s, clock.clone()))
              .collect()
          } else {
            Vec::new()
          };

          let (y_min, y_max) = if !visible.is_empty() {
            let raw_min = visible.iter().map(|(_, s, _)| *s).fold(f64::INFINITY, f64::min);
            let raw_max = visible
              .iter()
              .map(|(_, s, _)| *s)
              .fold(0.0_f64, f64::max)
              .max(if self.graph_view_span >= 0.99 { max_ever } else { 0.0 });

            let mut span = (raw_max - raw_min).max(raw_max * 0.05).max(0.01);
            if span < raw_max * 0.15 {
              span = raw_max * 0.15;
            }
            let mid = (raw_min + raw_max) * 0.5;
            let mut y_min = (mid - span * 0.6).max(0.0);
            let mut y_max = mid + span * 0.6;
            if y_max <= y_min {
              y_max = y_min + 0.01;
            }
            if raw_max > y_max {
              y_max = raw_max * 1.05;
            }
            if raw_min < y_min {
              y_min = (raw_min * 0.95).max(0.0);
            }
            (y_min, y_max)
          } else {
            (0.0, 1.0)
          };

          let num_lines = 5;
          for i in 0..num_lines {
            let t = i as f32 / (num_lines - 1) as f32;
            let y = plot_bottom - t * plot_h;
            painter.line_segment(
              [egui::pos2(plot_left, y), egui::pos2(plot_right, y)],
              egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 48)),
            );
            let label_v = y_min + (y_max - y_min) * t as f64;
            painter.text(
              egui::pos2(plot_left - 4.0, y),
              egui::Align2::RIGHT_CENTER,
              format_speed(label_v),
              egui::FontId::monospace(10.0),
              egui::Color32::from_rgb(120, 120, 130),
            );
          }

          if has_data && total_secs > 0.0 {
            let view_t0 = self.graph_view_start * total_secs;
            let view_t1 = (self.graph_view_start + self.graph_view_span) * total_secs;
            let view_dur = (view_t1 - view_t0).max(0.001);
            let nice_steps = [
              1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 14400.0,
            ];
            let target = view_dur / 5.0;
            let mut step = nice_steps[0];
            for &s in &nice_steps {
              step = s;
              if s >= target {
                break;
              }
            }
            let first = (view_t0 / step).ceil() * step;
            let mut t = first;
            while t <= view_t1 + 0.001 {
              let frac = ((t - view_t0) / view_dur) as f32;
              let x = plot_left + frac.clamp(0.0, 1.0) * plot_w;
              painter.line_segment(
                [egui::pos2(x, plot_bottom), egui::pos2(x, plot_bottom + 4.0)],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)),
              );
              painter.text(
                egui::pos2(x, plot_bottom + 6.0),
                egui::Align2::CENTER_TOP,
                format_duration(t),
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(120, 120, 130),
              );
              t += step;
            }
          }

          if has_data && !visible.is_empty() {
            if max_ever > 0.0 && y_max > y_min && self.graph_view_span >= 0.99 {
              let t_max = ((max_ever - y_min) / (y_max - y_min)) as f32;
              let y_max_line = plot_bottom - t_max.clamp(0.0, 1.0) * plot_h;
              let mut x = plot_left;
              let dash = 6.0;
              let gap = 4.0;
              while x < plot_right {
                let x2 = (x + dash).min(plot_right);
                painter.line_segment(
                  [egui::pos2(x, y_max_line), egui::pos2(x2, y_max_line)],
                  egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 140, 255)),
                );
                x += dash + gap;
              }
              painter.text(
                egui::pos2(plot_right - 2.0, y_max_line - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("max {}", format_speed(max_ever)),
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(60, 140, 255),
              );
            }

            let target_points = (plot_w as usize).max(2).min(visible.len());
            let mut points = Vec::with_capacity(target_points);
            let n = (target_points - 1) as f32;
            let current_speed = visible.last().map(|(_, s, _)| *s).unwrap_or(0.0);

            for i in 0..target_points {
              let src_idx = if target_points == visible.len() {
                i
              } else {
                (i as f64 * (visible.len() - 1) as f64 / (target_points - 1) as f64).round() as usize
              };
              let (_, v, _) = visible[src_idx.min(visible.len() - 1)];
              let x = plot_left + (i as f32 / n) * plot_w;
              let t = ((v - y_min) / (y_max - y_min)) as f32;
              let y = plot_bottom - t.clamp(0.0, 1.0) * plot_h;
              points.push(egui::pos2(x, y));
            }

            painter.add(egui::Shape::line(
              points.clone(),
              egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 120)),
            ));

            if let Some(last) = points.last() {
              painter.circle_filled(*last, 3.5, egui::Color32::from_rgb(0, 200, 120));
              painter.text(
                egui::pos2(last.x - 4.0, last.y + 4.0),
                egui::Align2::RIGHT_TOP,
                format!("now {}", format_speed(current_speed)),
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(0, 200, 120),
              );
            }

            if let Some(pointer) = response.hover_pos()
              && pointer.x >= plot_left
              && pointer.x <= plot_right
              && pointer.y >= plot_top
              && pointer.y <= plot_bottom
            {
              let frac = ((pointer.x - plot_left) / plot_w).clamp(0.0, 1.0);
              let idx_in_vis = (frac * (visible.len() - 1) as f32).round() as usize;
              let idx_in_vis = idx_in_vis.min(visible.len() - 1);
              let (_global_idx, speed, ref clock) = visible[idx_in_vis];
              let x = plot_left + (idx_in_vis as f32 / (visible.len() - 1).max(1) as f32) * plot_w;
              let ty = ((speed - y_min) / (y_max - y_min)) as f32;
              let y = plot_bottom - ty.clamp(0.0, 1.0) * plot_h;

              painter.line_segment(
                [egui::pos2(x, plot_top), egui::pos2(x, plot_bottom)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(200, 200, 220, 80)),
              );
              painter.circle_filled(egui::pos2(x, y), 4.0, egui::Color32::from_rgb(220, 220, 240));
              painter.circle_filled(egui::pos2(x, y), 2.5, egui::Color32::from_rgb(0, 200, 120));

              let tip = format!("{}  *  {}", format_speed(speed), clock);
              let tip_pos = egui::pos2(x + 8.0, y - 8.0);
              let galley = painter.layout_no_wrap(tip, egui::FontId::monospace(11.0), egui::Color32::from_rgb(230, 230, 240));
              let pad = egui::vec2(6.0, 4.0);
              let mut box_rect = egui::Rect::from_min_size(tip_pos, galley.size() + pad * 2.0);
              if box_rect.right() > plot_right {
                box_rect = box_rect.translate(egui::vec2(plot_right - box_rect.right(), 0.0));
              }
              if box_rect.top() < plot_top {
                box_rect = box_rect.translate(egui::vec2(0.0, plot_top - box_rect.top() + 4.0));
              }
              painter.rect_filled(box_rect, 3.0, egui::Color32::from_rgba_unmultiplied(20, 20, 28, 220));
              painter.galley(box_rect.min + pad, galley, egui::Color32::from_rgb(230, 230, 240));
            }

            if self.graph_view_span < 0.999 {
              let zoom_pct = ((1.0 - self.graph_view_span) / 0.98 * 100.0).clamp(0.0, 100.0);
              let label = format!("zoom {:.0}%  (double-click reset)", zoom_pct);
              painter.text(
                egui::pos2(plot_left + 4.0, plot_top + 2.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(140, 140, 160),
              );
            }
          } else {
            let t = ui.input(|i| i.time) as f32;
            let steps = 64;
            let mut points = Vec::with_capacity(steps + 1);
            let y_mid = (plot_top + plot_bottom) * 0.5;
            let amp = plot_h * 0.03;
            for i in 0..=steps {
              let x_t = i as f32 / steps as f32;
              let x = plot_left + x_t * plot_w;
              let wave = ((x_t * std::f32::consts::TAU * 1.5) + t * 2.0).sin() * amp;
              points.push(egui::pos2(x, y_mid + wave));
            }
            painter.add(egui::Shape::line(
              points,
              egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(0, 200, 120, 90)),
            ));
          }
        }

        ui.horizontal(|ui| {
          let graph_btn_text = if self.graph_enabled { "Hide Graph" } else { "Show Graph" };
          if ui.button(graph_btn_text).clicked() {
            self.graph_enabled = !self.graph_enabled;
            if !self.graph_enabled {
              self.speed_history.clear();
              self.max_speed_ever = 0.0;
              self.graph_view_start = 0.0;
              self.graph_view_span = 1.0;
              self.graph_drag_origin = None;
            } else {
              self.last_speed_sample = Instant::now();
            }
          }

          ui.separator();

          let speed_btn_text = if self.use_live_speed { "Speed: Live" } else { "Speed: Global" };
          if ui
            .button(speed_btn_text)
            .on_hover_text("Toggle between live worker speed and global average speed")
            .clicked()
          {
            self.use_live_speed = !self.use_live_speed;
          }

          ui.separator();

          ui.label("History:");
          let history_options: &[(u32, &str)] = &[(30, "30s"), (60, "1m"), (120, "2m"), (180, "3m"), (300, "5m")];

          for &(secs, label) in history_options {
            let selected = self.graph_history_secs == secs;
            if ui.selectable_label(selected, label).clicked() {
              self.graph_history_secs = secs;
              const SAMPLE_SECS: f64 = 0.25;
              let max_samples = ((secs as f64) / SAMPLE_SECS).ceil() as usize;
              while self.speed_history.len() > max_samples.max(2) {
                self.speed_history.pop_front();
              }
            }
          }
        });
      });

    egui::CentralPanel::default().show(ui, |ui| {
      let avail_w = ui.available_width();
      ui.set_min_width(avail_w);

      if self.kernels_loading {
        ui.add_space(MARGIN_NARROW);
        ui.horizontal(|ui| {
          ui.label(egui::RichText::new("Loading kernels").color(COL_ACCENT).strong());
          if !self.kernels_load_stage.is_empty() {
            ui.label(egui::RichText::new(format!("  {}", self.kernels_load_stage)).color(COL_GPU));
          }
        });

        let bar = egui::ProgressBar::new(self.kernels_load_progress.clamp(0.0, 1.0))
          .desired_width(avail_w)
          .show_percentage()
          .animate(true);

        ui.add(bar);
      } else if self.selected_puzzle == ALL_PUZZLES {
        let all_prog = state.all_progress;
        let cur_puz = state.all_current_puzzle;
        let sweep = state.all_sweep;
        let total_p = state.all_total_puzzles;
        ui.horizontal(|ui| {
          ui.label(egui::RichText::new("ALL sweep").color(COL_ACCENT).strong());
          if cur_puz != 0 {
            ui.label(egui::RichText::new(format!("  now: #{}", cur_puz)).color(COL_GPU));
          }
          if total_p > 0 {
            ui.label(egui::RichText::new(format!("  ({:.0}% of {} puzzles)", all_prog, total_p)).small());
          }
          if sweep > 0 {
            ui.label(egui::RichText::new(format!("  sweep {}", sweep)).small().color(egui::Color32::GRAY));
          }
          ui.separator();
          ui.label(egui::RichText::new("turn").small());
          let mut secs = self.all_turn_secs;
          egui::ComboBox::from_id_salt("all_turn_secs")
            .selected_text(format!("{}s", secs))
            .width(52.0)
            .show_ui(ui, |ui| {
              for &opt in &ALL_MODE_TURN_SECS_OPTIONS {
                ui.selectable_value(&mut secs, opt, format!("{}s", opt));
              }
            });
          if secs != self.all_turn_secs {
            self.all_turn_secs = secs;
            self.all_turn_secs_atomic.store(secs, Ordering::Relaxed);
            if self.running && is_all_mode(self.selected_puzzle) {
              arm_all_mode_turn(secs.max(1));
              crate::app_log!("[ALL] live turn duration → {}s (timer reset for current puzzle)", secs);
            }
          }
          ui.label(egui::RichText::new("per puzzle").small().color(egui::Color32::GRAY))
            .on_hover_text(
              "How long each puzzle runs before rotating to the next.\n\
Can be changed while running - takes effect immediately (timer restarts).\n\
Workers finish the current chunk after the timer, then GLOBAL_NEXT / ALLOC.log are saved.",
            );
        });
        let bar = egui::ProgressBar::new((all_prog / 100.0).clamp(0.0, 1.0) as f32)
          .desired_width(avail_w)
          .show_percentage();
        ui.add(bar);
      }

      egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("workers_table_scroll")
        .show(ui, |ui| {
          ui.set_min_width(avail_w);

          let min_model = 100.0;
          let min_index = 100.0;
          let min_key = 100.0;

          let w_device = 70.0;
          let w_units = 70.0;
          let w_clock = 90.0;
          let w_checked = 90.0;
          let w_speed = 150.0;

          let fixed_sum = w_device + w_units + w_clock + w_checked + w_speed;
          let remaining = (avail_w - fixed_sum).max(0.0);

          let w_model = (remaining * 0.20).max(min_model);
          let w_index = (remaining * 0.40).max(min_index);
          let w_key = (remaining * 0.40).max(min_key);

          let total = w_device + w_model + w_units + w_clock + w_index + w_key + w_checked + w_speed;
          let scale = if total > avail_w && total > 0.0 { avail_w / total } else { 1.0 };

          TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(w_device * scale).clip(true))
            .column(Column::exact(w_model * scale).clip(true))
            .column(Column::exact(w_units * scale).clip(true))
            .column(Column::exact(w_clock * scale).clip(true))
            .column(Column::exact(w_index * scale).clip(true))
            .column(Column::exact(w_key * scale).clip(true))
            .column(Column::exact(w_checked * scale).clip(true))
            .column(Column::exact(w_speed * scale).clip(true))
            .header(26.0, |mut header| {
              for title in ["Device", "Model", "Units", "Clock", "Index", "Key", "Checked", "Speed"] {
                header.col(|ui| {
                  ui.strong(egui::RichText::new(title).color(COL_ACCENT));
                });
              }
            })
            .body(|mut body| {
              for row in state.workers.iter() {
                body.row(20.0, |mut table_row| {
                  table_row.col(|ui| {
                    let color = if row.device.starts_with("GPU") { COL_GPU } else { COL_CPU };
                    ui.colored_label(color, &row.device);
                  });
                  table_row.col(|ui| {
                    ui.monospace(&row.model);
                  });
                  table_row.col(|ui| {
                    ui.monospace(&row.units);
                  });
                  table_row.col(|ui| {
                    ui.monospace(&row.clock);
                  });
                  table_row.col(|ui| {
                    ui.monospace(&row.index_hex);
                  });
                  table_row.col(|ui| {
                    ui.monospace(&row.key_hex);
                  });
                  table_row.col(|ui| {
                    ui.monospace(format_count(row.checked));
                  });
                  table_row.col(|ui| {
                    ui.monospace(format_speed(row.speed_kps));
                  });
                });
              }
            });
        });
    });

    drop(state);

    self.draw_buy_window(ui.ctx());
    self.draw_log_window(ui.ctx());
    self.draw_settings_window(ui.ctx());
    self.draw_help_window(ui.ctx());

    ui.ctx().request_repaint_after(Duration::from_millis(UI_REFRESH_INTERVAL_MS));
  }
}

impl KeyHunterApp {
  fn draw_settings_window(
    &mut self,
    ctx: &egui::Context,
  ) {
    if !self.show_settings_window {
      return;
    }

    #[cfg(feature = "next_release")]
    if self.settings_tab == SettingsTab::Tray && self.idle_solving {
      ctx.request_repaint_after(Duration::from_millis(500));
    }

    let mut open = true;

    egui::Window::new("Settings  (F10)")
      .open(&mut open)
      .collapsible(true)
      .resizable(true)
      .default_size([560.0, 520.0])
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          for &tab in SettingsTab::ALL {
            let selected = self.settings_tab == tab;
            let color = match tab {
              SettingsTab::Core => COL_ACCENT,
              SettingsTab::Solver => COL_GPU,
              SettingsTab::Gui => COL_ACCENT,

              #[cfg(feature = "next_release")]
              SettingsTab::Tray => COL_CPU,
            };
            let text = if selected {
              egui::RichText::new(tab.label()).strong().color(color)
            } else {
              egui::RichText::new(tab.label())
            };
            if ui.selectable_label(selected, text).clicked() {
              self.settings_tab = tab;
            }
          }
        });
        ui.separator();

        let footer_h = 52.0;
        let body_h = (ui.available_height() - footer_h).max(80.0);
        egui::ScrollArea::vertical()
          .max_height(body_h)
          .auto_shrink([false, false])
          .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            match self.settings_tab {
              SettingsTab::Core => self.draw_settings_tab_core(ui),
              SettingsTab::Solver => self.draw_settings_tab_solver(ui),
              SettingsTab::Gui => self.draw_settings_tab_gui(ui),

              #[cfg(feature = "next_release")]
              SettingsTab::Tray => self.draw_settings_tab_tray(ui),
            }
          });

        ui.separator();
        ui.horizontal(|ui| {
          if ui
            .button(egui::RichText::new("Save config").strong().color(COL_ACCENT))
            .on_hover_text(format!("Write current values to {SETTINGS_FILE}"))
            .clicked()
          {
            match self.save_settings_to_file(SETTINGS_FILE) {
              Ok(()) => {
                self.settings_status = format!("Saved {SETTINGS_FILE}");
                crate::app_log!("[SETTINGS] Saved {SETTINGS_FILE}");
              }
              Err(e) => {
                self.settings_status = format!("Save failed: {e}");
                crate::app_log!("[SETTINGS] Save failed: {e}");
              }
            }
          }
          if ui
            .button(egui::RichText::new("Load config").strong().color(COL_GPU))
            .on_hover_text(format!("Reload values from {SETTINGS_FILE}"))
            .clicked()
          {
            match self.load_settings_from_file(SETTINGS_FILE) {
              Ok(()) => {
                self.settings_status = format!("Loaded {SETTINGS_FILE}");
                crate::app_log!("[SETTINGS] Loaded {SETTINGS_FILE}");

                if matches!(self.device.as_str(), "GPU" | "Both")
                  && let Some(path) = self.kernels_path.clone()
                  && !path.is_empty()
                  && std::path::Path::new(&path).is_file()
                {
                  self.pending_kernel_autoload = Some(path);
                }
              }
              Err(e) => {
                self.settings_status = format!("Load failed: {e}");
                crate::app_log!("[SETTINGS] Load failed: {e}");
              }
            }
          }
          if ui
            .button(egui::RichText::new("Load defaults").strong().color(COL_CPU))
            .on_hover_text("Reset all settings to built-in defaults (not written until Save)")
            .clicked()
          {
            self.load_defaults();
          }
        });
        if !self.settings_status.is_empty() {
          ui.label(egui::RichText::new(&self.settings_status).small().color(COL_GPU));
        }
      });

    if !open {
      self.show_settings_window = false;
    }
  }

  fn settings_section(
    ui: &mut egui::Ui,
    title: &str,
    color: egui::Color32,
  ) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title).strong().color(color).size(15.0));
    ui.separator();
    ui.add_space(2.0);
  }

  fn settings_row_label(
    ui: &mut egui::Ui,
    text: &str,
    color: egui::Color32,
  ) {
    ui.label(egui::RichText::new(text).strong().color(color));
  }

  fn settings_hint(
    ui: &mut egui::Ui,
    text: &str,
  ) {
    ui.label(egui::RichText::new(text).small().weak());
    ui.add_space(6.0);
  }

  fn draw_settings_tab_gui(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    Self::settings_section(ui, "Startup windows", COL_ACCENT);

    let mut help = self.auto_start_help;
    if ui
      .checkbox(&mut help, egui::RichText::new("Auto-start Help window").color(COL_ACCENT))
      .on_hover_text("When enabled, the Help window opens collapsed on every app start.")
      .changed()
    {
      self.auto_start_help = help;
    }
    Self::settings_hint(ui, "Help opens collapsed. Disable to keep it closed at startup.");

    let mut log = self.auto_start_log;
    if ui
      .checkbox(&mut log, egui::RichText::new("Auto-start Log window").color(COL_GPU))
      .on_hover_text("When enabled, the Log window opens collapsed on every app start.")
      .changed()
    {
      self.auto_start_log = log;
    }
    Self::settings_hint(ui, "Log opens collapsed. Disable to keep it closed at startup.");

    ui.label(
      egui::RichText::new("Changes apply on next start. Save config to keep them.")
        .small()
        .weak(),
    );
  }

  fn draw_settings_tab_core(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    let busy = self.running || self.benchmark_running;

    Self::settings_section(ui, "Search target", COL_ACCENT);

    ui.add_enabled_ui(!busy, |ui| {
      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Puzzle", COL_ACCENT);
        let mut selected = self.selected_puzzle;
        let selected_label = if selected == ALL_PUZZLES {
          "ALL".to_string()
        } else {
          match puzzle_by_number(selected) {
            Some(info) if info.has_pubkey() => format!("#{}*", selected),
            _ => format!("#{}", selected),
          }
        };
        egui::ComboBox::from_id_salt("settings_puzzle")
          .width(160.0)
          .selected_text(selected_label)
          .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, ALL_PUZZLES, "ALL");
            for p in active_puzzles() {
              let label = if p.has_pubkey() {
                format!("#{}*", p.number)
              } else {
                format!("#{}", p.number)
              };
              ui.selectable_value(&mut selected, p.number, label);
            }
          });
        if selected != self.selected_puzzle {
          self.selected_puzzle = selected;
          if selected == ALL_PUZZLES {
            if let Ok(mut gui) = self.state.lock() {
              gui.target = "ALL unsolved puzzles (#71-#160)".to_string();
              gui.range = "round-robin batches across remaining puzzles".to_string();
              gui.all_progress = 0.0;
              gui.all_current_puzzle = 0;
              gui.all_total_puzzles = remaining_puzzles().len();
            }
          } else if let Some(info) = puzzle_by_number(selected)
            && let Ok(mut gui) = self.state.lock()
          {
            gui.target = if info.has_pubkey() {
              format!("{}  [pubkey]", info.address)
            } else {
              info.address.to_string()
            };
            gui.range = format!("{} - {}", info.range_start().to_hex(), info.range_end().to_hex());
            gui.all_progress = 0.0;
            gui.all_current_puzzle = 0;
            gui.all_total_puzzles = 0;
          }
        }
      });
      Self::settings_hint(ui, "Which Bitcoin puzzle to hunt. ALL cycles unsolved #71-#160.");

      if is_all_mode(self.selected_puzzle) {
        ui.horizontal(|ui| {
          Self::settings_row_label(ui, "ALL turn time", COL_ACCENT);
          let mut secs = self.all_turn_secs;
          egui::ComboBox::from_id_salt("settings_all_turn")
            .width(100.0)
            .selected_text(format!("{secs} s"))
            .show_ui(ui, |ui| {
              for &opt in &ALL_MODE_TURN_SECS_OPTIONS {
                ui.selectable_value(&mut secs, opt, format!("{opt} s"));
              }
            });
          if secs != self.all_turn_secs {
            self.all_turn_secs = secs;
            self.all_turn_secs_atomic.store(secs, Ordering::Relaxed);
          }
        });

        Self::settings_hint(ui, "Seconds spent on each puzzle before rotating to the next.");
      }

      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Search mode", COL_ACCENT);
        egui::ComboBox::from_id_salt("settings_mode")
          .width(140.0)
          .selected_text(match self.solving_mode {
            SearchMode::Sequence => "Sequence",
            SearchMode::Pcg => "PCG",
          })
          .show_ui(ui, |ui| {
            ui.selectable_value(&mut self.solving_mode, SearchMode::Sequence, "Sequence");
            ui.selectable_value(&mut self.solving_mode, SearchMode::Pcg, "PCG");
          });
      });
      Self::settings_hint(ui, "Sequence walks the range in order. PCG samples pseudo-randomly.");
    });

    Self::settings_section(ui, "Performance", COL_GPU);

    ui.horizontal(|ui| {
      Self::settings_row_label(ui, "Intensity", COL_GPU);
      let mut pct = ((self.intensity * 100.0).round() as u32).clamp(10, 100);
      if ui.add(egui::Slider::new(&mut pct, 10..=100).step_by(10.0).suffix("%")).changed() {
        self.intensity = pct as f32 / 100.0;
        self.intensity_atomic.store(pct, Ordering::Relaxed);
      }
    });
    Self::settings_hint(ui, "Fraction of GPU/CPU budget used per batch (10-100%).");

    let mut turbo = self.intel_turbo;
    if ui.checkbox(&mut turbo, egui::RichText::new("Intel Turbo Boost").color(COL_GPU)).changed() {
      self.intel_turbo = turbo;
      Self::set_intel_turbo(turbo);
    }
    Self::settings_hint(ui, "Toggle CPU turbo via intel_pstate when the sysfs path is writable.");

    ui.label(
      egui::RichText::new("Changes apply now. Save config to keep them after restart.")
        .small()
        .weak(),
    );
  }

  fn draw_settings_tab_solver(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    let busy = self.running || self.benchmark_running;

    Self::settings_section(ui, "Device", COL_ACCENT);

    ui.add_enabled_ui(!busy, |ui| {
      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Run on", COL_ACCENT);
        egui::ComboBox::from_id_salt("settings_device")
          .width(120.0)
          .selected_text(&self.device)
          .show_ui(ui, |ui| {
            ui.selectable_value(&mut self.device, "CPU".into(), "CPU");
            ui.selectable_value(&mut self.device, "GPU".into(), "GPU");
            ui.selectable_value(&mut self.device, "Both".into(), "Both");
          });
      });
      Self::settings_hint(ui, "CPU only, GPU only, or both in parallel.");
    });

    Self::settings_section(ui, "Kernel", COL_GPU);

    ui.add_enabled_ui(!busy, |ui| {
      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Type", COL_GPU);
        egui::ComboBox::from_id_salt("settings_kernel_type")
          .width(280.0)
          .selected_text(egui::RichText::new(self.kernel.kernel_type.display_name()).color(COL_GPU))
          .show_ui(ui, |ui| {
            for &ty in &[KernelType::ChudnovskyAddG, KernelType::ChudnovskyMeloniCoZ] {
              if !self.registry.sizes_for(ty).is_empty() && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed() {
                self.on_kernel_type_changed();
              }
            }
            ui.separator();
            for &ty in &[
              KernelType::Jacobian,
              KernelType::JacobianCoZMeloni,
              KernelType::JacobianLinearGlv,
              KernelType::JacobianEndoGlv,
              KernelType::JacobianKaratsuba,
            ] {
              if !self.registry.sizes_for(ty).is_empty() && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed() {
                self.on_kernel_type_changed();
              }
            }
            ui.separator();
            for &ty in &[
              KernelType::ProjectiveAddGLazy,
              KernelType::ProjectiveBaby,
              KernelType::ProjectiveLatticeGlv,
              KernelType::ProjectiveRestrictedGlv,
              KernelType::ProjectiveXOnlyRegG,
              KernelType::ProjectiveXZBatch,
              KernelType::ProjectiveZFactorBatch,
            ] {
              if !self.registry.sizes_for(ty).is_empty() && ui.selectable_value(&mut self.kernel.kernel_type, ty, ty.display_name()).changed() {
                self.on_kernel_type_changed();
              }
            }
            ui.separator();
            if !self.registry.sizes_for(KernelType::Xyzz).is_empty()
              && ui
                .selectable_value(&mut self.kernel.kernel_type, KernelType::Xyzz, KernelType::Xyzz.display_name())
                .changed()
            {
              self.on_kernel_type_changed();
            }
            ui.separator();
            if !self.registry.sizes_for(KernelType::XzDifferential).is_empty()
              && ui
                .selectable_value(
                  &mut self.kernel.kernel_type,
                  KernelType::XzDifferential,
                  KernelType::XzDifferential.display_name(),
                )
                .changed()
            {
              self.on_kernel_type_changed();
            }
          });
      });
      Self::settings_hint(ui, "Curve arithmetic variant used by the GPU kernel.");

      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Size", COL_GPU);
        let sizes: Vec<String> = self.registry.sizes_for(self.kernel.kernel_type).to_vec();
        egui::ComboBox::from_id_salt("settings_kernel_size")
          .width(120.0)
          .selected_text(egui::RichText::new(&self.kernel.kernel_size).color(COL_GPU))
          .show_ui(ui, |ui| {
            for s in sizes {
              if ui.selectable_value(&mut self.kernel.kernel_size, s.clone(), s.as_str()).changed() {
                self.refresh_kernel();
              }
            }
          });
      });
      Self::settings_hint(ui, "BATCH_STORAGE_LIMIT baked into the CUBIN (keys held per thread).");

      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Keys / thread", COL_GPU);
        let ks: u32 = self.kernel.kernel_size.parse().unwrap_or(1024);
        let min_pk = 1u32;
        let max_pk = (ks * 8).max(4096);
        let mut pk = self.parallel_keys.clamp(min_pk, max_pk);
        if ui.add(egui::DragValue::new(&mut pk).range(min_pk..=max_pk).speed(8.0)).changed() {
          self.parallel_keys = pk;
        }
      });
      Self::settings_hint(ui, "How many private keys each GPU thread walks per launch.");
    });

    Self::settings_section(ui, "Launch geometry", COL_CPU);

    ui.add_enabled_ui(!busy, |ui| {
      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Mode", COL_CPU);
        let mut manual = self.launch_manual;
        egui::ComboBox::from_id_salt("settings_launch")
          .width(120.0)
          .selected_text(if manual { "Manual" } else { "Auto" })
          .show_ui(ui, |ui| {
            ui.selectable_value(&mut manual, false, "Auto");
            ui.selectable_value(&mut manual, true, "Manual");
          });
        self.launch_manual = manual;
      });
      Self::settings_hint(ui, "Auto picks block/grid. Manual uses the values below (single GPU).");

      ui.add_enabled_ui(self.launch_manual, |ui| {
        ui.horizontal(|ui| {
          Self::settings_row_label(ui, "Block", COL_CPU);
          let mut b = self.block_size;
          egui::ComboBox::from_id_salt("settings_block")
            .width(100.0)
            .selected_text(b.to_string())
            .show_ui(ui, |ui| {
              for &v in BLOCK_SIZES {
                ui.selectable_value(&mut b, v, v.to_string());
              }
            });
          self.block_size = b.max(1);

          Self::settings_row_label(ui, "Grid", COL_CPU);
          let mut g = self.grid_size;
          egui::ComboBox::from_id_salt("settings_grid")
            .width(100.0)
            .selected_text(g.to_string())
            .show_ui(ui, |ui| {
              for &v in GRID_SIZES {
                ui.selectable_value(&mut g, v, v.to_string());
              }
            });
          self.grid_size = g.max(1);
        });
        Self::settings_hint(ui, "CUDA block size and grid size for each kernel launch.");

        ui.horizontal(|ui| {
          Self::settings_row_label(ui, "Chunk ×", COL_CPU);
          let mut c = self.chunk_multiplier;
          egui::ComboBox::from_id_salt("settings_chunk")
            .width(100.0)
            .selected_text(c.to_string())
            .show_ui(ui, |ui| {
              for &v in CHUNK_MULTIPLIERS {
                ui.selectable_value(&mut c, v, v.to_string());
              }
            });
          self.chunk_multiplier = c.max(1);
        });
        Self::settings_hint(ui, "Multiplies work per chunk (higher = longer launches).");
      });
    });

    let package_color = egui::Color32::from_rgb(230, 230, 235);
    Self::settings_section(ui, "Package", package_color);

    ui.horizontal(|ui| {
      Self::settings_row_label(ui, "Last .OWL", package_color);
      let mut path = self.kernels_path.as_deref().unwrap_or("(none)");

      ui.add(egui::TextEdit::multiline(&mut path));
    });

    if !self.kernels_status.is_empty() {
      ui.label(
        egui::RichText::new(&self.kernels_status)
          .small()
          .color(if self.kernels_loaded { COL_GPU } else { COL_BENCH }),
      );
    }
    Self::settings_hint(ui, "Loaded from the main window. Path is stored when you Save config.");
  }

  #[cfg(feature = "next_release")]
  fn draw_settings_tab_tray(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    Self::settings_section(ui, "System tray", COL_ACCENT);

    let mut tray_on = self.tray_enabled;
    if ui
      .checkbox(&mut tray_on, egui::RichText::new("Enable system tray icon").color(COL_ACCENT))
      .changed()
    {
      self.tray_enabled = tray_on;
      self.ensure_tray_state();
    }
    Self::settings_hint(ui, "Show the Puzzle Hunter Pro icon in the desktop tray.");

    ui.add_enabled_ui(self.tray_enabled, |ui| {
      let mut hide = self.hide_on_close;
      if ui.checkbox(&mut hide, egui::RichText::new("Hide on close").color(COL_ACCENT)).changed() {
        self.hide_on_close = hide;
        if let Some(item) = &self.tray_hide_item {
          item.set_checked(self.hide_on_close);
        }
      }
      Self::settings_hint(ui, "Window close button minimizes to tray instead of quitting.");

      Self::settings_section(ui, "Idle solving", COL_GPU);

      let mut idle = self.idle_solving;
      if ui
        .checkbox(&mut idle, egui::RichText::new("Start solving when PC is idle").color(COL_GPU))
        .changed()
      {
        self.idle_solving = idle;
        if let Some(item) = &self.tray_idle_item {
          item.set_checked(self.idle_solving);
        }
      }
      Self::settings_hint(ui, "Auto-Start after idle timeout; Stop when you use the PC again.");

      ui.add_enabled_ui(self.idle_solving, |ui| {
        ui.horizontal(|ui| {
          Self::settings_row_label(ui, "Idle time before start", COL_GPU);
          let mut mins = self.idle_minutes;
          if ui.add(egui::Slider::new(&mut mins, 1..=60).suffix(" min")).changed() {
            self.idle_minutes = mins;
          }
        });
        Self::settings_hint(ui, "Minutes of no input before the solver starts.");

        let idle_secs = Self::linux_idle_ms() / 1000;
        let threshold_secs = (self.idle_minutes.max(1) as u64).saturating_mul(60);
        ui.horizontal(|ui| {
          ui.label(egui::RichText::new(format!("PC idle: {idle_secs} s")).small().monospace().color(COL_CPU));
          ui.label(egui::RichText::new(format!("/ threshold {threshold_secs} s")).small().weak());
          if idle_secs == 0 {
            ui.label(egui::RichText::new("(idle source returned 0)").small().color(COL_BENCH));
          }
        });
        Self::settings_hint(ui, "Live idle timer from X11/Wayland (system-idle-time).");
      });
    });

    ui.label(egui::RichText::new("Left-click tray: hide/show. Right-click: menu.").small().weak());
  }

  fn draw_help_window(
    &mut self,
    ctx: &egui::Context,
  ) {
    if !self.show_help_window {
      return;
    }

    let mut open = true;

    egui::Window::new("Help (F1)")
      .open(&mut open)
      .collapsible(true)
      .default_open(false)
      .resizable(true)
      .default_size([450.0, 250.0])
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          for &tab in HelpTab::ALL {
            let selected = self.help_tab == tab;
            let color = match tab {
              HelpTab::Shortcuts => COL_ACCENT,
              HelpTab::System => COL_GPU,
              HelpTab::Wiki => COL_CPU,
            };

            let text = if selected {
              egui::RichText::new(tab.label()).strong().color(color)
            } else {
              egui::RichText::new(tab.label())
            };

            if ui.selectable_label(selected, text).clicked() {
              self.help_tab = tab;
            }
          }
        });

        ui.separator();

        let body_h = ui.available_height().max(80.0);

        egui::ScrollArea::vertical()
          .max_height(body_h)
          .auto_shrink([false, false])
          .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            match self.help_tab {
              HelpTab::Shortcuts => self.draw_help_tab_shortcuts(ui),
              HelpTab::System => self.draw_help_tab_system(ui),
              HelpTab::Wiki => self.draw_help_tab_wiki(ui),
            }
          });
      });

    if self.show_help_window && self.help_tab == HelpTab::System {
      ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }

    if !open {
      self.show_help_window = false;
    }
  }

  fn draw_help_tab_shortcuts(
    &self,
    ui: &mut egui::Ui,
  ) {
    Self::settings_section(ui, "Keyboard Shortcuts", COL_ACCENT);

    let shortcuts = [
      ("F1", "Toggle this Help window"),
      ("F2", "Toggle Buy Kernels window"),
      ("F10", "Toggle Settings window"),
      ("F12", "Toggle Log window"),
    ];

    for (key, desc) in shortcuts {
      ui.horizontal(|ui| {
        ui.label(egui::RichText::new(key).strong().monospace().color(COL_ACCENT));
        ui.label(desc);
      });
      ui.add_space(2.0);
    }

    ui.add_space(MARGIN_NARROW);
    Self::settings_hint(ui, "These shortcuts work from the main window.");
  }

  fn refresh_system_info_if_due(&mut self) {
    if self.system_info_last_refresh.elapsed() < std::time::Duration::from_secs(1) {
      return;
    }
    self.system_info_last_refresh = Instant::now();

    let cpu = detect_cpu_info();
    self.system_info_cpu_model = cpu.model.clone();
    self.system_info_cpu_line = if cpu.freq_mhz > 0.0 {
      format!("{} cores / {} threads   ~{}", cpu.cores, cpu.threads, format_clock_ghz(cpu.freq_mhz))
    } else {
      format!("{} cores / {} threads", cpu.cores, cpu.threads)
    };

    self.system_info_mem_lines.clear();
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
      let mut total_kb = 0u64;
      let mut available_kb = 0u64;
      for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
          total_kb = rest.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
          available_kb = rest.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
        }
      }
      if total_kb > 0 {
        let total_gb = total_kb as f64 / 1024.0 / 1024.0;
        let avail_gb = available_kb as f64 / 1024.0 / 1024.0;
        self.system_info_mem_lines.push(format!("{:.1} GB total", total_gb));
        self.system_info_mem_lines.push(format!("{:.1} GB available", avail_gb));
      }
    }

    self.system_info_gpu_lines.clear();
    if let Ok(output) = std::process::Command::new("nvidia-smi")
      .args(["--query-gpu=index,name,memory.total,clocks.sm", "--format=csv,noheader,nounits"])
      .output()
      && output.status.success()
    {
      let text = String::from_utf8_lossy(&output.stdout);
      for line in text.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 2 {
          let idx = parts[0];
          let name = parts.get(1).unwrap_or(&"?");
          let mem = parts.get(2).unwrap_or(&"?");
          let clock = parts.get(3).unwrap_or(&"?");
          self.system_info_gpu_lines.push(format!("GPU {idx}: {name}"));
          self.system_info_gpu_lines.push(format!("  Memory: {mem} MiB   Clock: {clock} MHz"));
        }
      }
    }
    if self.system_info_gpu_lines.is_empty() {
      let clocks = detect_gpu_clocks_mhz();
      if self.has_gpu {
        self
          .system_info_gpu_lines
          .push(format!("{} GPU(s) detected (CUDA present)", self.gpu_count));
        for (id, mhz) in clocks {
          self.system_info_gpu_lines.push(format!("GPU {id} clock: {mhz} MHz"));
        }
      } else {
        self.system_info_gpu_lines.push("No GPU / CUDA detected".to_string());
      }
    }
  }

  fn draw_help_tab_system(
    &mut self,
    ui: &mut egui::Ui,
  ) {
    self.refresh_system_info_if_due();

    Self::settings_section(ui, "System Information", COL_GPU);

    ui.horizontal(|ui| {
      Self::settings_row_label(ui, "CPU", COL_CPU);
      ui.label(egui::RichText::new(&self.system_info_cpu_model).monospace());
    });
    if !self.system_info_cpu_line.is_empty() {
      ui.label(egui::RichText::new(&self.system_info_cpu_line).small().color(COL_CPU));
    }

    ui.add_space(6.0);

    if self.system_info_mem_lines.is_empty() {
      ui.label(egui::RichText::new("Memory: n/a").small().weak());
    } else {
      ui.horizontal(|ui| {
        Self::settings_row_label(ui, "Memory", COL_CPU);
        if let Some(first) = self.system_info_mem_lines.first() {
          ui.label(egui::RichText::new(first).monospace());
        }
      });
      if self.system_info_mem_lines.len() > 1 {
        ui.label(egui::RichText::new(&self.system_info_mem_lines[1]).small().color(COL_CPU));
      }
    }

    ui.add_space(6.0);

    Self::settings_row_label(ui, "GPU", COL_GPU);
    for (i, line) in self.system_info_gpu_lines.iter().enumerate() {
      let rich = if i % 2 == 0 {
        egui::RichText::new(line).monospace()
      } else {
        egui::RichText::new(line).small().color(COL_GPU)
      };
      ui.label(rich);
    }

    ui.add_space(MARGIN_NARROW);
    Self::settings_hint(ui, "Clock and GPU info refresh once per second.");
  }

  fn draw_help_tab_wiki(
    &self,
    ui: &mut egui::Ui,
  ) {
    Self::settings_section(ui, "Wiki / Documentation", COL_CPU);

    struct WikiEntry {
      date: &'static str,
      name: &'static str,
      url: &'static str,
    }

    const WIKI: &[WikiEntry] = &[
      WikiEntry {
        date: "2025-11-16",
        name: "Puzzle Hunter Pro",
        url: "https://control-owl.github.io/blog/post/2026/11/16/puzzle-hunter-pro.html",
      },
      WikiEntry {
        date: "2025-11-05",
        name: "Key Hunter - How it all started",
        url: "https://control-owl.github.io/blog/post/2026/10/09/key-hunter.html",
      },
    ];

    if WIKI.is_empty() {
      ui.label(egui::RichText::new("No wiki entries defined yet.").weak());
      return;
    }

    for entry in WIKI {
      ui.horizontal(|ui| {
        ui.label(egui::RichText::new(entry.date).small().monospace().color(COL_CPU));
        ui.hyperlink_to(entry.name, entry.url);
      });
      ui.add_space(4.0);
    }

    ui.add_space(MARGIN_NARROW);
    Self::settings_hint(ui, "Click a link to open it in your browser.");
  }
}

fn format_speed(v: f64) -> String {
  if v < 0.001 {
    "0".to_string()
  } else if v < 1.0 {
    format!("{:.0}", v * 1000.0)
  } else if v < 1000.0 {
    format!("{:.1} K", v)
  } else if v < 1_000_000.0 {
    format!("{:.1} M", v / 1000.0)
  } else {
    format!("{:.1} G", v / 1_000_000.0)
  }
}

fn format_duration(secs: f64) -> String {
  let s = secs.max(0.0) as u64;
  let h = s / 3600;
  let m = (s % 3600) / 60;
  let sec = s % 60;
  if h > 0 {
    format!("{}h{:02}m", h, m)
  } else if m > 0 {
    format!("{}m{:02}s", m, sec)
  } else {
    format!("{}s", sec)
  }
}

fn format_count(n: u64) -> String {
  if n < 1000 {
    format!("{}", n)
  } else if n < 1_000_000 {
    format!("{:.1} K", n as f64 / 1000.0)
  } else if n < 1_000_000_000 {
    format!("{:.1} M", n as f64 / 1_000_000.0)
  } else {
    format!("{:.1} G", n as f64 / 1_000_000_000.0)
  }
}
