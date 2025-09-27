// src/main.rs
//! Minimal benchmark harness using the project's performance stack.
//! - Timing via TimeSpan (minstant under the hood).
//! - Power via CPUEnergy + sampler thread writing to PowerCsv.
//! - CSV headers match your tests:
//!     timing: "unix_ns,stage,elapsed_ms"
//!     power : "unix_ns,source,power_W,energy_J_total"

use astrum::performance::clock::now;
use astrum::performance::performance::*;
use astrum::performance::power::*;
use astrum::performance::timer::*;
use astrum::performance::{CPUEnergy, PowerCsv, TimeSpan, TimingCsv};

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

fn main() {
    let cfg = Config::from_env();

    // Output paths (default: $TMPDIR//astrum_*_<pid>.csv)
    let (timing_path, power_path) = make_paths(&cfg);

    // Leak the CSV sinks so &'static works across the sampler thread
    let timing: &'static TimingCsv =
        Box::leak(Box::new(TimingCsv::new(timing_path.to_str().unwrap())));
    let power: &'static PowerCsv =
        Box::leak(Box::new(PowerCsv::new(power_path.to_str().unwrap())));

    // Power source auto-detect (powercap/hwmon/none)
    let cpu = CPUEnergy::autodetect();

    // Start sampler thread (unless disabled)
    let sampler = if !cfg.no_power {
        Some(Sampler::start(cpu.clone(), power, cfg.period_ms))
    } else {
        eprintln!("power sampling disabled (--no-power)");
        None
    };

    // ---- Timed stages --------------------------------------------------------
    {
        let _t = TimeSpan::new("stage_rollout", timing);
        let _ = busy_compute(cfg.rollout_iters);
    }
    {
        let _t = TimeSpan::new("stage_gradients", timing);
        let _ = busy_compute(cfg.grad_iters);
    }
    {
        let _t = TimeSpan::new("stage_line_search", timing);
        let _ = busy_compute(cfg.line_iters);
    }

    // Let the sampler pick up a final sample edge, then stop.
    if let Some(s) = sampler {
        thread::sleep(Duration::from_millis(cfg.tail_sleep_ms));
        s.stop();
    }

    eprintln!("timing -> {}", timing_path.display());
    eprintln!("power  -> {}", power_path.display());

    // Optional: verify headers exist (mirrors your test expectations)
    if cfg.self_check {
        let t_head = fs::read_to_string(&timing_path).expect("read timing csv");
        assert!(t_head.lines().next().unwrap().trim() == "unix_ns,stage,elapsed_ms");

        let p_head = fs::read_to_string(&power_path).expect("read power csv");
        assert!(p_head.lines().next().unwrap().trim() == "unix_ns,source,power_W,energy_J_total");
    }
}

// ------------------------- Dummy workload -------------------------

fn busy_compute(iters: usize) -> f64 {
    let mut acc = 0.0f64;
    let mut x = 1.000_001_f64;
    for _ in 0..iters {
        // a little FP chaos to keep ALUs busy
        x = (x.sin().cos().tan()).abs() + 1e-9;
        // make sure the loop can’t be optimized out
        std::hint::black_box(&mut x);
        acc += x;
    }
    // side-effect to “use” the result
    let _ = fs::write("/dev/null", format!("{acc:.6}"));
    acc
}

// ------------------------- Sampler thread -------------------------

struct Sampler {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Sampler {
    fn start(cpu: CPUEnergy, sink: &'static PowerCsv, period_ms: u64) -> Self {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop_c = stop.clone();
        let handle = std::thread::spawn(move || {
            eprintln!("Power source: {}", cpu.debug_source_path());
            let mut last_j = cpu.read_total_j();
            let mut last_t = now();

            while !stop_c.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(period_ms));
                let t1 = now();

                match &cpu {
                    CPUEnergy::Powercap { .. } => {
                        let j1 = cpu.read_total_j();
                        if let (Some(j1), Some(j0)) = (j1, last_j) {
                            let dt = (t1 - last_t).as_secs_f64();
                            let p = if dt > 0.0 { Some((j1 - j0) / dt) } else { None };
                            sink.sample("cpu_pkg", p, Some(j1));
                        }
                        last_j = j1;
                        last_t = t1;
                    }
                    CPUEnergy::Hwmon { .. } => {
                        let w = cpu.read_watts();
                        sink.sample("cpu_pkg", w, None);
                        last_t = t1;
                    }
                    CPUEnergy::None => {
                        // Keep the CSV (header) around; rows may remain empty on WSL/unsupported hosts.
                        last_t = t1;
                    }
                }
            }
        });
        Self { stop, handle: Some(handle) }
    }

    fn stop(mut self) {
        use std::sync::atomic::Ordering;
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

// ------------------------- Config / CLI -------------------------

struct Config {
    rollout_iters: usize,
    grad_iters: usize,
    line_iters: usize,
    period_ms: u64,
    tail_sleep_ms: u64,
    csv_dir: Option<PathBuf>,
    csv_prefix: String,
    no_power: bool,
    self_check: bool,
}

impl Config {
    fn from_env() -> Self {
        let mut cfg = Self {
            rollout_iters: read_usize("ROLL_ITERS", 5_000_000),
            grad_iters: read_usize("GRAD_ITERS", 6_000_000),
            line_iters: read_usize("LINE_ITERS", 4_000_000),
            period_ms: read_u64("POWER_PERIOD_MS", 100),
            tail_sleep_ms: read_u64("TAIL_SLEEP_MS", 300),
            csv_dir: std::env::var_os("CSV_DIR").map(PathBuf::from),
            csv_prefix: std::env::var("CSV_PREFIX").unwrap_or_else(|_| "astrum".to_string()),
            no_power: read_bool("NO_POWER", false),
            self_check: read_bool("SELF_CHECK", false),
        };

        // Very small argv parser for convenience (env vars still work)
        let mut args = std::env::args().skip(1);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--roll" => if let Some(v) = args.next() { cfg.rollout_iters = v.parse().unwrap_or(cfg.rollout_iters); }
                "--grad" => if let Some(v) = args.next() { cfg.grad_iters    = v.parse().unwrap_or(cfg.grad_iters); }
                "--line" => if let Some(v) = args.next() { cfg.line_iters    = v.parse().unwrap_or(cfg.line_iters); }
                "--period-ms" => if let Some(v) = args.next() { cfg.period_ms = v.parse().unwrap_or(cfg.period_ms); }
                "--tail-ms"   => if let Some(v) = args.next() { cfg.tail_sleep_ms = v.parse().unwrap_or(cfg.tail_sleep_ms); }
                "--csv-dir"   => if let Some(v) = args.next() { cfg.csv_dir = Some(PathBuf::from(v)); }
                "--prefix"    => if let Some(v) = args.next() { cfg.csv_prefix = v; }
                "--no-power"  => { cfg.no_power = true; }
                "--self-check"=> { cfg.self_check = true; }
                "--help" | "-h" => {
                    eprintln!(
"Usage: bench [options]
  --roll N         rollout iterations        (default env ROLL_ITERS)
  --grad N         gradient iterations       (default env GRAD_ITERS)
  --line N         line-search iterations    (default env LINE_ITERS)
  --period-ms MS   power sample period       (default env POWER_PERIOD_MS)
  --tail-ms MS     extra sleep before stop   (default env TAIL_SLEEP_MS)
  --csv-dir PATH   output directory          (default: $TMPDIR)
  --prefix STR     csv file prefix           (default: 'astrum')
  --no-power       disable power sampling
  --self-check     assert CSV headers exist
"
                    );
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        cfg
    }
}

fn read_usize(k: &str, d: usize) -> usize {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}
fn read_u64(k: &str, d: u64) -> u64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}
fn read_bool(k: &str, d: bool) -> bool {
    match std::env::var(k) {
        Ok(s) => matches!(s.as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => d,
    }
}

// ------------------------- Paths -------------------------

fn make_paths(cfg: &Config) -> (PathBuf, PathBuf) {
    let pid = std::process::id();
    let ts = now();
    let dir = cfg
        .csv_dir
        .clone()
        .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
        .unwrap_or_else(|| std::env::temp_dir());

    let timing = dir.join(format!("{}_timing_{:?}_{:?}.csv", cfg.csv_prefix, ts, pid));
    let power  = dir.join(format!("{}_power_{:?}_{:?}.csv",  cfg.csv_prefix, ts, pid));
    (timing, power)
}
