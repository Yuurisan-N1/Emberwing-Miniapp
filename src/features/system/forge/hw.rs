#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;

#[cfg(unix)]
mod dynlib {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};
    use std::path::Path;

    const RTLD_NOW: c_int = 2;

    extern "C" {
        fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, sym: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
    }

    pub fn open(path: &Path) -> Option<*mut c_void> {
        let c = CString::new(path.to_string_lossy().as_bytes()).ok()?;
        let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW) };
        if h.is_null() {
            None
        } else {
            Some(h)
        }
    }

    pub fn sym(handle: *mut c_void, name: &str) -> *mut c_void {
        match CString::new(name) {
            Ok(n) => unsafe { dlsym(handle, n.as_ptr()) },
            Err(_) => std::ptr::null_mut(),
        }
    }

    pub fn close(handle: *mut c_void) {
        unsafe {
            dlclose(handle);
        }
    }
}

#[cfg(windows)]
mod dynlib {
    use std::os::raw::{c_char, c_void};
    use std::path::Path;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(file: *const c_char) -> *mut c_void;
        fn GetProcAddress(handle: *mut c_void, sym: *const c_char) -> *mut c_void;
        fn FreeLibrary(handle: *mut c_void) -> i32;
    }

    pub fn open(path: &Path) -> Option<*mut c_void> {
        let mut bytes = path.to_string_lossy().as_bytes().to_vec();
        bytes.push(0);
        let h = unsafe { LoadLibraryA(bytes.as_ptr() as *const c_char) };
        if h.is_null() {
            None
        } else {
            Some(h)
        }
    }

    pub fn sym(handle: *mut c_void, name: &str) -> *mut c_void {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe { GetProcAddress(handle, bytes.as_ptr() as *const c_char) }
    }

    pub fn close(handle: *mut c_void) {
        unsafe {
            FreeLibrary(handle);
        }
    }
}

type DevicesFn = unsafe extern "C" fn() -> c_int;
type EvalFn =
    unsafe extern "C" fn(*const c_void, *const c_int, c_int, c_int, u64, *mut c_int) -> c_int;
type PackFn = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    c_int,
    c_int,
    c_int,
    *const i8,
    *const i8,
    *const u64,
    *const i8,
    c_int,
    *const i8,
    c_int,
) -> c_int;
type SizeFn = unsafe extern "C" fn() -> c_int;
type ProveFn =
    unsafe extern "C" fn(*const c_void, *const c_int, c_int, c_int, i64, *mut c_int) -> c_int;

struct Cuda {
    handle: *mut c_void,
    devices: DevicesFn,
    eval: EvalFn,
    pack: PackFn,
    size: SizeFn,
    prove: Option<ProveFn>,
}

unsafe impl Send for Cuda {}
unsafe impl Sync for Cuda {}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    Cpu,
    Gpu,
}

/// The widest board the festival deals; the device struct has to fit it.
pub const MAX_BOARD: usize = 180;

impl Backend {
    pub fn tag(&self) -> &'static str {
        match self {
            Backend::Cpu => "cpu",
            Backend::Gpu => "gpu",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cpu {
    pub brand: String,
    pub physical: usize,
    pub logical: usize,
    pub avx2: bool,
    pub avx512: bool,
}

#[derive(Clone, Debug)]
pub struct Hw {
    pub cpu: Cpu,
    pub gpu: Option<String>,
    pub devices: i32,
    pub backend: Backend,
    pub threads: usize,
    pub samples: u32,
    pub note: String,
}

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key)
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
}

fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&n| n > 0)
}

fn env_str(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn simd_flags() -> (bool, bool) {
    (
        std::arch::is_x86_feature_detected!("avx2"),
        std::arch::is_x86_feature_detected!("avx512f"),
    )
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn simd_flags() -> (bool, bool) {
    (false, false)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn platform_cpu(logical: usize) -> (String, usize) {
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut brand = String::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "model name" | "Model" | "Hardware" if brand.is_empty() => brand = v.to_string(),
                "physical id" => pairs.push(("p".into(), v.to_string())),
                "core id" => pairs.push(("c".into(), v.to_string())),
                _ => {}
            }
        }
    }
    let mut cores: Vec<(String, String)> = Vec::new();
    let mut pending: Option<String> = None;
    for (k, v) in pairs {
        if k == "p" {
            pending = Some(v);
        } else if let Some(p) = pending.take() {
            cores.push((p, v));
        }
    }
    cores.sort();
    cores.dedup();
    let physical = if cores.is_empty() {
        logical
    } else {
        cores.len().min(logical)
    };
    (brand, physical)
}

#[cfg(target_os = "macos")]
fn platform_cpu(logical: usize) -> (String, usize) {
    let brand = sysctl("machdep.cpu.brand_string").unwrap_or_default();
    let cores = sysctl("hw.physicalcpu")
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(logical);
    (brand, cores)
}

#[cfg(target_os = "macos")]
fn sysctl(key: &str) -> Option<String> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos", windows)))]
fn platform_cpu(logical: usize) -> (String, usize) {
    (String::new(), logical)
}

#[cfg(windows)]
fn platform_cpu(logical: usize) -> (String, usize) {
    let brand = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default();
    (brand, win_physical_cores().unwrap_or(logical))
}

#[cfg(windows)]
fn win_physical_cores() -> Option<usize> {
    use std::process::Command;
    let probe = |exe: &str, args: &[&str]| -> Option<usize> {
        let out = Command::new(exe).args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        text.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<usize>().ok())
            .find(|n| *n > 0)
    };
    if let Some(n) = probe("wmic", &["cpu", "get", "NumberOfCores", "/value"]) {
        return Some(n);
    }
    probe(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Processor).NumberOfCores",
        ],
    )
}

fn read_cpu() -> Cpu {
    let logical = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let (brand, physical) = platform_cpu(logical);
    let (avx2, avx512) = simd_flags();
    let brand = if brand.trim().is_empty() {
        "unknown cpu".to_string()
    } else {
        brand
    };
    Cpu {
        brand,
        physical,
        logical,
        avx2,
        avx512,
    }
}

/// Every accelerator file name we are willing to load, most likely first.
/// The library is optional: on a machine with no cuda toolkit the bot still
/// runs, it just stays on the cpu. Names are listed for all platforms so a
/// package built on one os still auto-detects a library dropped in on another.
fn lib_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["forge_mc.dll", "libforge_mc.dll", "libforge_mc.so"]
    } else if cfg!(target_os = "macos") {
        vec!["libforge_mc.dylib", "forge_mc.dylib", "libforge_mc.so"]
    } else {
        vec!["libforge_mc.so", "forge_mc.so", "libforge_mc.dylib"]
    }
}

/// Name shown in logs before a library is loaded.
fn lib_default_name() -> &'static str {
    lib_names()[0]
}

fn lib_dirs() -> Vec<PathBuf> {
    let mut d: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            d.push(p.to_path_buf());
            d.push(p.join("cuda"));
            if let Some(up) = p.parent() {
                d.push(up.join("cuda"));
            }
        }
    }
    d.push(PathBuf::from("cuda"));
    d.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/cuda")));
    d
}

fn lib_candidates() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("EMB_FORGE_CUDA_LIB") {
        if !p.trim().is_empty() {
            v.push(PathBuf::from(p));
        }
    }
    let dirs = lib_dirs();
    for name in lib_names() {
        for d in &dirs {
            v.push(d.join(name));
        }
    }
    v
}

fn open_cuda() -> (Option<Cuda>, bool, Option<String>) {
    let mut seen = false;
    for path in lib_candidates() {
        if !path.exists() {
            continue;
        }
        seen = true;
        let handle = match dynlib::open(&path) {
            Some(h) => h,
            None => continue,
        };
        let label = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| lib_default_name().to_string());
        unsafe {
            let d = dynlib::sym(handle, "forge_mc_devices");
            let e = dynlib::sym(handle, "forge_mc_eval");
            let p = dynlib::sym(handle, "forge_mc_pack");
            let s = dynlib::sym(handle, "forge_mc_size");
            if d.is_null() || e.is_null() || p.is_null() || s.is_null() {
                dynlib::close(handle);
                continue;
            }
            let pr = dynlib::sym(handle, "forge_ev_prove");
            return (
                Some(Cuda {
                    handle,
                    devices: std::mem::transmute::<*mut c_void, DevicesFn>(d),
                    eval: std::mem::transmute::<*mut c_void, EvalFn>(e),
                    pack: std::mem::transmute::<*mut c_void, PackFn>(p),
                    size: std::mem::transmute::<*mut c_void, SizeFn>(s),
                    prove: if pr.is_null() {
                        None
                    } else {
                        Some(std::mem::transmute::<*mut c_void, ProveFn>(pr))
                    },
                }),
                true,
                Some(label),
            );
        }
    }
    (None, seen, None)
}

struct Probe {
    cuda: Option<Cuda>,
    seen: bool,
    lib: Option<String>,
}

fn probe() -> &'static Probe {
    static P: OnceLock<Probe> = OnceLock::new();
    P.get_or_init(|| {
        let (cuda, seen, lib) = open_cuda();
        Probe { cuda, seen, lib }
    })
}

/// Name of the accelerator file that actually loaded, or the one we looked for.
fn lib_label() -> String {
    probe()
        .lib
        .clone()
        .unwrap_or_else(|| lib_default_name().to_string())
}

fn cuda() -> &'static Option<Cuda> {
    &probe().cuda
}

fn lib_seen() -> bool {
    probe().seen
}

/// A device count is not enough: the device struct has to actually accept the
/// biggest board this game deals, otherwise every eval silently falls back to
/// the cpu while the log still claims the gpu is armed.
fn cuda_pack_ok(c: &Cuda, n: usize) -> bool {
    let stride = unsafe { (c.size)() };
    if stride <= 0 {
        return false;
    }
    let mut blob = vec![0u8; stride as usize];
    let loc = vec![0i8; n];
    let mut face = vec![-1i8; n];
    for (i, f) in face.iter_mut().enumerate() {
        *f = (i % 24) as i8;
    }
    let up = vec![0u64; n];
    let empty: [i8; 0] = [];
    let rc = unsafe {
        (c.pack)(
            blob.as_mut_ptr() as *mut c_void,
            0,
            n as c_int,
            7,
            2,
            loc.as_ptr(),
            face.as_ptr(),
            up.as_ptr(),
            empty.as_ptr(),
            0,
            empty.as_ptr(),
            0,
        )
    };
    rc == 0
}

fn gpu_name() -> Option<String> {
    let out = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s.lines().next().unwrap_or("").trim().to_string())
    }
}

fn auto_threads(cpu: &Cpu, _backend: Backend) -> usize {
    match cpu.logical {
        0 | 1 => 1,
        2 => 1,
        n => (n - 1).min(8),
    }
}

fn auto_samples(cpu: &Cpu, backend: Backend) -> u32 {
    match backend {
        Backend::Gpu => 4096,
        Backend::Cpu => match cpu.logical {
            0 | 1 | 2 => 8,
            3 | 4 => 24,
            _ => 48,
        },
    }
}

fn detect() -> Hw {
    let cpu = read_cpu();
    let cuda = cuda();
    let devices = match cuda {
        Some(c) => unsafe { (c.devices)() },
        None => -1,
    };
    let name = gpu_name();
    let gpu = if devices > 0 { name.clone() } else { None };
    let forced = env_str("emb_forge_device");
    let want_gpu = match forced.as_deref() {
        Some("gpu") | Some("cuda") => true,
        Some("cpu") => false,
        _ => true,
    };
    let seen = lib_seen();
    let pack_ok = match cuda {
        Some(c) => cuda_pack_ok(c, MAX_BOARD),
        None => false,
    };
    let (backend, note) = if devices > 0 && want_gpu && pack_ok {
        (
            Backend::Gpu,
            format!(
                "cuda device(s) {} armed via {}, device struct takes {} tiles",
                devices,
                lib_label(),
                MAX_BOARD
            ),
        )
    } else if devices > 0 && want_gpu {
        (
            Backend::Cpu,
            format!(
                "cuda device(s) {} found but {} rejects {} tiles, cpu fallback",
                devices,
                lib_label(),
                MAX_BOARD
            ),
        )
    } else if devices > 0 {
        (Backend::Cpu, "cuda present but forced cpu".to_string())
    } else if cuda.is_none() && seen {
        (
            Backend::Cpu,
            format!("{} present but not loadable, cpu fallback", lib_label()),
        )
    } else if cuda.is_none() {
        (
            Backend::Cpu,
            match &name {
                Some(n) => format!(
                    "gpu {} detected, cuda lib missing, cpu fallback",
                    n.replace("NVIDIA ", "")
                ),
                None => "no cuda runtime lib, cpu fallback".to_string(),
            },
        )
    } else {
        (
            Backend::Cpu,
            match &name {
                Some(n) => format!(
                    "gpu {} detected, cuda driver error, cpu fallback",
                    n.replace("NVIDIA ", "")
                ),
                None => "cuda lib loaded but 0 devices, cpu fallback".to_string(),
            },
        )
    };
    let threads = env_usize("emb_forge_threads").unwrap_or_else(|| auto_threads(&cpu, backend));
    let samples = env_u32("emb_forge_samples").unwrap_or_else(|| auto_samples(&cpu, backend));
    Hw {
        cpu,
        gpu,
        devices,
        backend,
        threads,
        samples,
        note,
    }
}

pub fn hw() -> &'static Hw {
    static H: OnceLock<Hw> = OnceLock::new();
    H.get_or_init(detect)
}

pub fn describe() -> String {
    let h = hw();
    let gpu = match (&h.gpu, h.devices) {
        (Some(n), d) => format!("{} ({} device(s))", n, d),
        (None, d) if d > 0 => format!("{} device(s)", d),
        _ => "none".to_string(),
    };
    let simd = match (h.cpu.avx512, h.cpu.avx2) {
        (true, _) => "avx512",
        (false, true) => "avx2",
        _ => "base",
    };
    format!(
        "cpu {} {} core {} thread {} gpu {} backend {} threads {} samples {} {}",
        h.cpu.brand,
        h.cpu.physical,
        h.cpu.logical,
        simd,
        gpu,
        h.backend.tag(),
        h.threads,
        h.samples,
        h.note
    )
}

pub fn backend() -> Backend {
    hw().backend
}

pub fn threads() -> usize {
    hw().threads
}

pub fn samples() -> u32 {
    hw().samples
}

pub fn cuda_ready() -> bool {
    hw().devices > 0
}

pub fn cuda_stride() -> usize {
    match cuda() {
        Some(c) => {
            let s = unsafe { (c.size)() };
            if s > 0 {
                s as usize
            } else {
                0
            }
        }
        None => 0,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cuda_pack(
    blob: &mut [u8],
    slot: usize,
    n: usize,
    slots: usize,
    stash: usize,
    loc: &[i8],
    face: &[i8],
    up: &[u64],
    tray: &[i8],
    side: &[i8],
) -> bool {
    let c = match cuda() {
        Some(c) => c,
        None => return false,
    };
    let stride = unsafe { (c.size)() };
    if stride <= 0 || (slot + 1) * stride as usize > blob.len() {
        return false;
    }
    let off = slot * stride as usize;
    let ptr = unsafe { blob.as_mut_ptr().add(off) as *mut c_void };
    let rc = unsafe {
        (c.pack)(
            ptr,
            0,
            n as c_int,
            slots as c_int,
            stash as c_int,
            loc.as_ptr(),
            face.as_ptr(),
            up.as_ptr(),
            tray.as_ptr(),
            tray.len() as c_int,
            side.as_ptr(),
            side.len() as c_int,
        )
    };
    rc == 0
}

pub fn cuda_eval(
    blob: &[u8],
    cand: &[i32],
    samples: u32,
    seed: u64,
    out: &mut [u32],
) -> bool {
    let c = match cuda() {
        Some(c) => c,
        None => return false,
    };
    if out.len() < cand.len() {
        return false;
    }
    let mut wins = vec![0i32; cand.len()];
    let rc = unsafe {
        (c.eval)(
            blob.as_ptr() as *const c_void,
            cand.as_ptr(),
            cand.len() as c_int,
            samples as c_int,
            seed,
            wins.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return false;
    }
    for (i, w) in wins.iter().enumerate() {
        out[i] = (*w).max(0) as u32;
    }
    true
}

pub fn cuda_prove(blob: &[u8], cand: &[i32], depth: u32, budget: i64) -> Option<Vec<i32>> {
    let c = match cuda() {
        Some(c) => c,
        None => return None,
    };
    let f = c.prove?;
    if cand.is_empty() || depth == 0 || budget <= 0 {
        return None;
    }
    let mut out = vec![0i32; cand.len() * 2];
    let rc = unsafe {
        f(
            blob.as_ptr() as *const c_void,
            cand.as_ptr(),
            cand.len() as c_int,
            depth as c_int,
            budget,
            out.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return None;
    }
    Some(out)
}

#[allow(dead_code)]
pub fn cuda_unload() {
    if let Some(c) = cuda() {
        dynlib::close(c.handle);
    }
}
