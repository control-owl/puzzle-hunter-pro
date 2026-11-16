use aes_gcm::{
  Aes256Gcm, Nonce,
  aead::{Aead, KeyInit},
};
/// authors = ["Control Owl <c0ntr01-_-0w1[at]r-o0-t[dot]wtf>"]
/// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..
/// CTRLOWL License
/// Copyright (c) 2025-2026 Control Owl. All rights reserved.
///
/// 1. Host Application
///    The free host application (binary) may be used and redistributed freely
///    provided it remains unmodified and this copyright notice is preserved.
///    You may not modify, reverse-engineer, rebrand, or sell the host application.
///
/// 2. Kernel Packages
///    The kernel packages (.OWL files) are commercial, closed-source products.
///    They are licensed strictly for use on the single machine whose hardware
///    fingerprint matches the one submitted at purchase.
///    You may not copy, share, redistribute, reverse-engineer, decompile,
///    disassemble, modify, or attempt to extract the kernels in any way.
///    Additional licenses may be purchased for additional machines.
///
/// 3. No Warranty - As Is
///    The software and kernels are provided "AS IS" without warranty of any kind,
///    express or implied. The author is not liable for any damages, losses,
///    failed searches, hardware issues, driver problems, or inability to run
///    the software on your system. By using or purchasing you accept full
///    responsibility for your environment and results.
///
/// 4. Acceptance
///    Using the host application or purchasing a kernel package constitutes
///    acceptance of these terms. If you do not agree, do not use or buy.
///
///    -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..
///
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

pub const DEVELOPER_GPG_FINGERPRINT: &str = "C88E6F25736AD83DA1C757B22524C8FEB60EFCB0";
pub const DEVELOPER_GPG_PUBLIC_KEY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/cert/Control_Owl_public.asc"));
pub const PACKAGE_MAGIC: &[u8] = b"CTRLOWL";
const HEADER_LEN: usize = 43;
const APP_SALT: &[u8] = b"CTRLOWL-PUZZLE-HUNTER-PRO-v1";
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

#[derive(Clone, Debug)]
pub struct LoadedPackage {
  pub encrypted_kernels: BTreeMap<String, Vec<u8>>,
  pub machine_fingerprint: [u8; 32],
  pub signed: bool,
  pub email: Option<String>,
}

impl Drop for LoadedPackage {
  fn drop(&mut self) {
    for blob in self.encrypted_kernels.values_mut() {
      blob.zeroize();
    }

    self.encrypted_kernels.clear();
    self.machine_fingerprint.zeroize();
  }
}

#[derive(Debug)]
pub enum PackageError {
  Io(String),
  InvalidMagic,
  InvalidFormat(String),
  MachineMismatch,
  NotSigned,
  SignatureInvalid(String),
  NoKernels,
  DecryptFailed(String),
}

impl std::fmt::Display for PackageError {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      PackageError::Io(e) => write!(f, "IO error: {}", e),
      PackageError::InvalidMagic => write!(f, "Invalid package magic (not a CTRLOWL file)"),
      PackageError::InvalidFormat(e) => write!(f, "Invalid package format: {}", e),
      PackageError::MachineMismatch => {
        write!(f, "This package is locked to a different machine")
      }
      PackageError::NotSigned => write!(f, "Package is not signed - refused"),
      PackageError::SignatureInvalid(e) => write!(f, "Signature rejected: {}", e),
      PackageError::NoKernels => write!(f, "Package contains no kernels"),
      PackageError::DecryptFailed(e) => write!(f, "Kernel decryption failed: {}", e),
    }
  }
}

impl std::error::Error for PackageError {}

fn collect_stable_ids() -> Vec<String> {
  let mut ids: Vec<String> = Vec::with_capacity(8);

  #[cfg(target_os = "linux")]
  {
    if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
      let t = id.trim();

      if !t.is_empty() {
        ids.push(format!("linux-machine-id:{}", t));
      }
    } else if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
      let t = id.trim();

      if !t.is_empty() {
        ids.push(format!("linux-dbus-id:{}", t));
      }
    }
  }

  #[cfg(target_os = "windows")]
  {
    if let Ok(out) = Command::new("reg")
      .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
      .output()
    {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout);

        for line in s.lines() {
          if line.contains("MachineGuid") {
            if let Some(guid) = line.split_whitespace().last() {
              if guid.len() >= 8 {
                ids.push(format!("win-machine-guid:{}", guid.trim()));
              }
            }
          }
        }
      }
    }
  }

  #[cfg(target_os = "macos")]
  {
    if let Ok(out) = Command::new("ioreg").args(["-rd1", "-c", "IOPlatformExpertDevice"]).output() {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout);

        for line in s.lines() {
          if line.contains("IOPlatformUUID") {
            if let Some(uuid) = line.split('"').nth(3) {
              if !uuid.is_empty() {
                ids.push(format!("mac-platform-uuid:{}", uuid));
              }
            }
          }
        }
      }
    }
  }

  #[cfg(target_os = "linux")]
  {
    for (path, tag) in [
      ("/sys/class/dmi/id/product_uuid", "dmi-product-uuid"),
      ("/sys/class/dmi/id/board_serial", "dmi-board-serial"),
      ("/sys/class/dmi/id/product_serial", "dmi-product-serial"),
      ("/sys/class/dmi/id/chassis_serial", "dmi-chassis-serial"),
    ] {
      if let Ok(v) = std::fs::read_to_string(path) {
        let t = v.trim();

        if !t.is_empty() && t != "None" && t != "To Be Filled By O.E.M." && t != "Default string" && t != "Not Specified" {
          ids.push(format!("{}:{}", tag, t));
        }
      }
    }
  }

  #[cfg(target_os = "windows")]
  {
    if let Ok(out) = Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_BaseBoard).SerialNumber"])
      .output()
    {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() && s != "None" && s != "To Be Filled By O.E.M." {
          ids.push(format!("win-baseboard-serial:{}", s));
        }
      }
    }

    if let Ok(out) = Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_ComputerSystemProduct).UUID"])
      .output()
    {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() && s != "None" {
          ids.push(format!("win-system-uuid:{}", s));
        }
      }
    }
  }

  #[cfg(target_os = "macos")]
  {
    if let Ok(out) = Command::new("system_profiler").args(["SPHardwareDataType"]).output() {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
          let line = line.trim();
          if let Some(rest) = line.strip_prefix("Serial Number (system):") {
            let v = rest.trim();
            if !v.is_empty() {
              ids.push(format!("mac-serial:{}", v));
            }
          }
          if let Some(rest) = line.strip_prefix("Hardware UUID:") {
            let v = rest.trim();
            if !v.is_empty() {
              ids.push(format!("mac-hw-uuid:{}", v));
            }
          }
        }
      }
    }
  }

  if let Ok(out) = Command::new("lscpu").output()
    && out.status.success()
  {
    let s = String::from_utf8_lossy(&out.stdout);
    let mut brand = String::new();
    let mut cores = String::new();

    for line in s.lines() {
      if let Some(rest) = line.strip_prefix("Model name:") {
        brand = rest.trim().to_string();
      }

      if let Some(rest) = line.strip_prefix("Core(s) per socket:") {
        cores = rest.trim().to_string();
      }
    }
    if !brand.is_empty() {
      ids.push(format!("cpu-brand:{}", brand));
    }

    if !cores.is_empty() {
      ids.push(format!("cpu-cores:{}", cores));
    }
  }

  #[cfg(target_os = "windows")]
  {
    if let Ok(out) = Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Processor).Name"])
      .output()
    {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();

        if !s.is_empty() {
          ids.push(format!("cpu-brand:{}", s));
        }
      }
    }
  }

  #[cfg(target_os = "macos")]
  {
    if let Ok(out) = Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
      if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();

        if !s.is_empty() {
          ids.push(format!("cpu-brand:{}", s));
        }
      }
    }
  }

  ids.sort();
  ids.dedup();

  ids
}

pub fn compute_machine_fingerprint() -> [u8; 32] {
  use sha2::{Digest, Sha256};

  let mut hasher = Sha256::new();
  hasher.update(APP_SALT);

  let ids = collect_stable_ids();

  if ids.is_empty() {
    hasher.update(b"fallback-empty");

    if let Ok(host) = hostname::get() {
      hasher.update(host.to_string_lossy().as_bytes());
    }
  } else {
    for id in &ids {
      hasher.update(id.as_bytes());
      hasher.update(b"|");
    }
  }

  let first = hasher.finalize();

  let raw_hex: String = first.iter().map(|b| format!("{:02x}", b)).collect();
  let mut hasher2 = Sha256::new();

  hasher2.update(APP_SALT);
  hasher2.update(raw_hex.as_bytes());

  let result = hasher2.finalize();

  let mut out = [0u8; 32];
  out.copy_from_slice(&result);

  out
}

pub fn compute_machine_fingerprint_hex() -> String {
  compute_machine_fingerprint().iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Clone, Debug, Default)]
pub struct GpuInfo {
  pub uuid: String,
  pub name: String,
  pub compute_cap: String,
}

#[derive(Clone, Debug, Default)]
pub struct MachineReport {
  pub fingerprint_hex: String,
  pub fingerprint_sources: Vec<String>,
  pub gpus: Vec<GpuInfo>,
  pub os: String,
  pub cpu: String,
  pub app_version: String,
}

pub fn collect_machine_report() -> MachineReport {
  let sources = collect_stable_ids();
  let fingerprint_hex = compute_machine_fingerprint_hex();

  let mut gpus = Vec::new();
  if let Ok(out) = Command::new("nvidia-smi")
    .args(["--query-gpu=uuid,name,compute_cap", "--format=csv,noheader,nounits"])
    .output()
    && out.status.success()
  {
    for line in String::from_utf8_lossy(&out.stdout).lines() {
      let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

      if parts.len() >= 3 {
        let sm = parts[2].replace('.', "");

        gpus.push(GpuInfo {
          uuid: parts[0].to_string(),
          name: parts[1].to_string(),
          compute_cap: format!("sm_{}", sm),
        });
      }
    }
  }

  let os = std::env::consts::OS.to_string();
  let cpu = sources
    .iter()
    .find(|s| s.starts_with("cpu-brand:"))
    .map(|s| s.trim_start_matches("cpu-brand:").to_string())
    .unwrap_or_default();

  MachineReport {
    fingerprint_hex,
    fingerprint_sources: sources,
    gpus,
    os,
    cpu,
    app_version: String::new(),
  }
}

pub fn build_license_request_payload(
  report: &MachineReport,
  product: &str,
  email: &str,
  coin: &str,
  transaction: &str,
) -> String {
  let mut lines = Vec::new();

  lines.push("CTRLOWL-LICENSE-REQUEST-v1".to_string());
  lines.push(format!("product={}", product));
  lines.push(format!("email={}", email.trim()));
  lines.push(format!("coin={}", coin.trim().to_lowercase()));
  lines.push(format!("transaction={}", transaction.trim()));
  lines.push(format!("fingerprint={}", report.fingerprint_hex));
  lines.push(format!("os={}", report.os));
  lines.push(format!("cpu={}", report.cpu));
  lines.push(format!("app={}", report.app_version));
  lines.push(format!("timestamp={}", chrono_like_timestamp()));
  lines.push(format!("gpu_count={}", report.gpus.len()));

  let mut arches: Vec<String> = report
    .gpus
    .iter()
    .map(|g| g.compute_cap.clone())
    .filter(|a| a.starts_with("sm_"))
    .collect();
  arches.sort();
  arches.dedup();

  lines.push(format!("arches={}", arches.join(",")));
  lines.push(format!("arch_count={}", arches.len()));

  if arches.len() > 1 {
    lines.push(format!("note=multi-arch machine; compile with --arch={}", arches.join(",")));
  }

  for (i, g) in report.gpus.iter().enumerate() {
    lines.push(format!("gpu{}={} | {} | {}", i, g.uuid, g.name, g.compute_cap));
  }

  for s in &report.fingerprint_sources {
    lines.push(format!("src={}", s));
  }

  let plaintext = lines.join("\n");

  match encrypt_to_developer(&plaintext) {
    Ok(armored) => {
      format!("-----BEGIN CTRLOWL REQUEST-----\n{}\n-----END CTRLOWL REQUEST-----", armored.trim())
    }
    Err(_) => {
      format!(
        "-----BEGIN CTRLOWL REQUEST (UNENCRYPTED)-----\n{}\n-----END CTRLOWL REQUEST-----",
        plaintext
      )
    }
  }
}

fn chrono_like_timestamp() -> String {
  let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

  format!("{}", secs)
}

fn encrypt_to_developer(plaintext: &str) -> Result<String, String> {
  if Command::new("gpg").arg("--version").output().is_err() {
    return Err("gpg not available".into());
  }

  let tmp = tempfile_dir()?;
  let homedir = tmp.join("gnupg");

  std::fs::create_dir_all(&homedir).map_err(|e| e.to_string())?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&homedir, std::fs::Permissions::from_mode(0o700));
  }

  let key_path = tmp.join("pubkey.asc");
  std::fs::write(&key_path, DEVELOPER_GPG_PUBLIC_KEY).map_err(|e| e.to_string())?;

  let import = Command::new("gpg")
    .args(["--batch", "--homedir", homedir.to_str().unwrap(), "--import", key_path.to_str().unwrap()])
    .output()
    .map_err(|e| format!("gpg import: {}", e))?;

  if !import.status.success() {
    let _ = std::fs::remove_dir_all(&tmp);
    return Err("could not import developer public key".into());
  }

  let plain_path = tmp.join("request.txt");
  std::fs::write(&plain_path, plaintext).map_err(|e| e.to_string())?;

  let out = Command::new("gpg")
    .args([
      "--batch",
      "--homedir",
      homedir.to_str().unwrap(),
      "--trust-model",
      "always",
      "--armor",
      "--encrypt",
      "--recipient",
      DEVELOPER_GPG_FINGERPRINT,
      "--output",
      "-",
      plain_path.to_str().unwrap(),
    ])
    .output()
    .map_err(|e| format!("gpg encrypt: {}", e))?;

  let _ = std::fs::remove_dir_all(&tmp);

  if !out.status.success() {
    return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
  }

  Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn read_u16_le(
  data: &[u8],
  off: usize,
) -> Result<u16, PackageError> {
  if off + 2 > data.len() {
    return Err(PackageError::InvalidFormat("truncated".into()));
  }

  Ok(u16::from_le_bytes([data[off], data[off + 1]]))
}

fn read_u32_le(
  data: &[u8],
  off: usize,
) -> Result<u32, PackageError> {
  if off + 4 > data.len() {
    return Err(PackageError::InvalidFormat("truncated".into()));
  }

  Ok(u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]))
}

fn verify_developer_signature(
  payload: &[u8],
  signature: &[u8],
) -> Result<String, PackageError> {
  if Command::new("gpg").arg("--version").output().is_err() {
    return Err(PackageError::SignatureInvalid(
      "gpg not available on this system - cannot verify package signature".into(),
    ));
  }

  let tmp = tempfile_dir().map_err(PackageError::Io)?;
  let homedir = tmp.join("gnupg");

  std::fs::create_dir_all(&homedir).map_err(|e| PackageError::Io(e.to_string()))?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&homedir, std::fs::Permissions::from_mode(0o700));
  }

  let key_path = tmp.join("pubkey.asc");
  std::fs::write(&key_path, DEVELOPER_GPG_PUBLIC_KEY).map_err(|e| PackageError::Io(e.to_string()))?;

  let import = Command::new("gpg")
    .args(["--batch", "--homedir", homedir.to_str().unwrap(), "--import", key_path.to_str().unwrap()])
    .output()
    .map_err(|e| PackageError::SignatureInvalid(format!("gpg import failed: {}", e)))?;

  if !import.status.success() {
    let err = String::from_utf8_lossy(&import.stderr);
    let _ = std::fs::remove_dir_all(&tmp);

    return Err(PackageError::SignatureInvalid(format!(
      "could not import developer public key: {}",
      err.trim()
    )));
  }

  let payload_path = tmp.join("payload.bin");
  let sig_path = tmp.join("payload.sig");

  std::fs::write(&payload_path, payload).map_err(|e| PackageError::Io(e.to_string()))?;
  std::fs::write(&sig_path, signature).map_err(|e| PackageError::Io(e.to_string()))?;

  let verify = Command::new("gpg")
    .args([
      "--batch",
      "--homedir",
      homedir.to_str().unwrap(),
      "--status-fd",
      "1",
      "--verify",
      sig_path.to_str().unwrap(),
      payload_path.to_str().unwrap(),
    ])
    .output()
    .map_err(|e| PackageError::SignatureInvalid(format!("gpg verify failed: {}", e)))?;

  let status_out = String::from_utf8_lossy(&verify.stdout);
  let _ = std::fs::remove_dir_all(&tmp);

  let mut good = false;
  let mut fingerprint = String::new();

  for line in status_out.lines() {
    let line = line.trim();

    if line.starts_with("[GNUPG:] GOODSIG ") {
      good = true;
    }

    if let Some(rest) = line.strip_prefix("[GNUPG:] VALIDSIG ")
      && let Some(fp) = rest.split_whitespace().next()
    {
      fingerprint = fp.to_uppercase();
    }

    if line.starts_with("[GNUPG:] BADSIG ") || line.starts_with("[GNUPG:] ERRSIG ") {
      return Err(PackageError::SignatureInvalid("GPG reported a bad or unusable signature".into()));
    }
  }

  if !good || fingerprint.is_empty() {
    return Err(PackageError::SignatureInvalid("no valid GPG signature found".into()));
  }

  let expected = DEVELOPER_GPG_FINGERPRINT
    .chars()
    .filter(|c| c.is_ascii_hexdigit())
    .collect::<String>()
    .to_uppercase();
  let got = fingerprint.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_uppercase();

  if got != expected {
    return Err(PackageError::SignatureInvalid(format!(
      "signed by {}, expected developer key {}",
      got, expected
    )));
  }

  Ok(got)
}

fn tempfile_dir() -> Result<std::path::PathBuf, String> {
  let base = std::env::temp_dir();
  let name = format!(
    "kh-pkg-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_nanos())
      .unwrap_or(0)
  );

  let dir = base.join(name);
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

  Ok(dir)
}

#[inline]
fn is_zstd_frame(buf: &[u8]) -> bool {
  buf.len() >= 4 && buf[0..4] == ZSTD_MAGIC
}

fn derive_package_key(machine_fp: &[u8; 32]) -> [u8; 32] {
  use hkdf::Hkdf;
  use sha2::Sha256;

  let mut ikm = [0u8; 32 + APP_SALT.len()];
  ikm[..32].copy_from_slice(machine_fp);
  ikm[32..].copy_from_slice(APP_SALT);

  let hk = Hkdf::<Sha256>::new(Some(&[]), &ikm);
  let mut key = [0u8; 32];

  hk.expand(b"CTRLOWL-AES-256-GCM-v1", &mut key)
    .expect("HKDF expand 32 bytes always succeeds");
  key
}

fn aes_gcm_decrypt_blob(
  key: &[u8; 32],
  blob: &[u8],
  name: &str,
) -> Result<Vec<u8>, PackageError> {
  if blob.len() < 12 + 16 {
    return Err(PackageError::DecryptFailed(format!(
      "{}: encrypted blob too short ({})",
      name,
      blob.len()
    )));
  }

  let nonce = Nonce::try_from(&blob[..12]).map_err(|e| PackageError::DecryptFailed(format!("{}: invalid nonce: {}", name, e)))?;
  let ct_and_tag = &blob[12..];
  let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| PackageError::DecryptFailed(format!("{}: invalid key: {}", name, e)))?;

  let mut plain = cipher
    .decrypt(&nonce, ct_and_tag)
    .map_err(|_| PackageError::DecryptFailed(format!("{}: GCM authentication failed (wrong machine or corrupted package)", name)))?;

  if plain.is_empty() && name.ends_with(".cubin") {
    plain.zeroize();

    return Err(PackageError::DecryptFailed(format!("{}: empty plaintext", name)));
  }

  Ok(plain)
}

pub fn decrypt_kernel_blob(
  machine_fp: &[u8; 32],
  encrypted_blob: &[u8],
  name: &str,
) -> Result<Vec<u8>, PackageError> {
  let mut key = derive_package_key(machine_fp);
  let mut inner = match aes_gcm_decrypt_blob(&key, encrypted_blob, name) {
    Ok(v) => v,
    Err(e) => {
      key.zeroize();
      return Err(e);
    }
  };

  key.zeroize();

  let cubin = if is_zstd_frame(&inner) {
    let decoded = zstd::decode_all(inner.as_slice()).map_err(|e| PackageError::DecryptFailed(format!("{}: zstd decompress failed: {e}", name)))?;
    inner.zeroize();

    decoded
  } else {
    inner
  };

  if cubin.is_empty() {
    return Err(PackageError::DecryptFailed(format!("{}: empty CUBIN after decrypt/decompress", name)));
  }

  Ok(cubin)
}

pub fn load_kernel_package_with_progress<F>(
  path: &Path,
  on_progress: &mut F,
) -> Result<LoadedPackage, PackageError>
where
  F: FnMut(&str, u32, u32) + ?Sized,
{
  let mut report = |stage: &str, done: u32, total: u32| {
    on_progress(stage, done, total);
  };

  report("Reading package", 0, 0);
  let raw = std::fs::read(path).map_err(|e| PackageError::Io(e.to_string()))?;

  let was_compressed = is_zstd_frame(&raw);
  report(if was_compressed { "Decompressing outer" } else { "Parsing" }, 0, 0);

  let data = if was_compressed {
    zstd::decode_all(raw.as_slice()).map_err(|e| PackageError::InvalidFormat(format!("zstd decompress failed: {e}")))?
  } else {
    raw
  };

  if data.len() < HEADER_LEN + 4 + 20 {
    return Err(PackageError::InvalidFormat("file too small".into()));
  }

  report("Verifying signature", 0, 0);
  let (payload, signature) = split_signature(&data)?;
  let Some(signature) = signature else {
    return Err(PackageError::NotSigned);
  };

  let signer_fp = verify_developer_signature(payload, &signature)?;

  if !payload.starts_with(PACKAGE_MAGIC) {
    return Err(PackageError::InvalidMagic);
  }

  let version = read_u16_le(payload, 7)?;
  if version != 1 {
    return Err(PackageError::InvalidFormat(format!(
      "unsupported package version {} (expected 1)",
      version
    )));
  }

  if payload[9] != 1 {
    return Err(PackageError::InvalidFormat(format!(
      "unsupported product byte {} (expected 1 = pro)",
      payload[9]
    )));
  }

  let _flags = payload[10];

  let mut locked_fp = [0u8; 32];
  locked_fp.copy_from_slice(&payload[11..43]);

  report("Checking machine lock", 0, 0);
  let current_fp = compute_machine_fingerprint();

  if locked_fp != current_fp {
    return Err(PackageError::MachineMismatch);
  }

  let mut offset = HEADER_LEN;
  let mut email: Option<String> = None;

  let email_len = read_u16_le(payload, offset)? as usize;

  offset += 2;
  if offset + email_len > payload.len() {
    return Err(PackageError::InvalidFormat("email truncated".into()));
  }

  if email_len > 0 {
    let s =
      String::from_utf8(payload[offset..offset + email_len].to_vec()).map_err(|e| PackageError::InvalidFormat(format!("bad email utf-8: {e}")))?;
    email = Some(s);
  }
  offset += email_len;

  let count = read_u32_le(payload, offset)? as usize;
  offset += 4;
  if count == 0 {
    return Err(PackageError::NoKernels);
  }

  let mut encrypted_kernels = BTreeMap::new();
  let total = count as u32;

  for i in 0..count {
    let name_len = read_u16_le(payload, offset)? as usize;
    offset += 2;
    if offset + name_len > payload.len() {
      return Err(PackageError::InvalidFormat(format!("kernel[{}] name truncated", i)));
    }

    let name = String::from_utf8(payload[offset..offset + name_len].to_vec())
      .map_err(|e| PackageError::InvalidFormat(format!("kernel[{}] bad name: {}", i, e)))?;
    offset += name_len;

    let enc_len = read_u32_le(payload, offset)? as usize;
    offset += 4;

    if offset + enc_len > payload.len() {
      return Err(PackageError::InvalidFormat(format!("kernel[{}] data truncated", i)));
    }

    let blob = payload[offset..offset + enc_len].to_vec();
    offset += enc_len;

    report("Indexing kernels", i as u32 + 1, total);

    if name.ends_with(".cubin") {
      encrypted_kernels.insert(name, blob);
    }
  }

  report("Indexing kernels", total, total);

  if encrypted_kernels.is_empty() {
    return Err(PackageError::InvalidFormat("Package contains no CUBIN kernels".into()));
  }

  eprintln!(
    "[INFO] Kernel Pro v1, images={}, signer={}{}",
    encrypted_kernels.len(),
    &signer_fp[..std::cmp::min(16, signer_fp.len())],
    email.as_ref().map(|e| format!(", email={e}")).unwrap_or_default()
  );

  Ok(LoadedPackage {
    encrypted_kernels,
    machine_fingerprint: locked_fp,
    signed: true,
    email,
  })
}

fn split_signature(data: &[u8]) -> Result<(&[u8], Option<Vec<u8>>), PackageError> {
  if data.len() < HEADER_LEN + 4 + 4 + 20 {
    return Ok((data, None));
  }

  let cand_len = read_u32_le(data, data.len() - 4)? as usize;

  if (20..=8192).contains(&cand_len) && data.len() >= HEADER_LEN + 4 + 4 + cand_len {
    let sig_start = data.len() - 4 - cand_len;
    let payload = &data[..sig_start];

    if payload.starts_with(PACKAGE_MAGIC) {
      let sig = data[sig_start..sig_start + cand_len].to_vec();

      return Ok((payload, Some(sig)));
    }
  }

  Ok((data, None))
}

mod hostname {
  pub fn get() -> Result<std::ffi::OsString, ()> {
    std::env::var_os("HOSTNAME").or_else(|| std::env::var_os("COMPUTERNAME")).ok_or(())
  }
}
