// authors = ["Control Owl <c0ntr01-_-0w1[at]r-o0-t[dot]wtf>"]
// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..
// CTRLOWL License
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

use chrono::Local;
use cudarc::{
  driver::{
    CudaContext, CudaFunction, CudaSlice, CudaStream, LaunchConfig, PushKernelArg, result,
    sys::{CUdevice, CUdevice_attribute},
  },
  nvrtc::Ptx,
};
use rayon::prelude::*;
use ripemd::Ripemd160;
use secp256k1::Secp256k1;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::BufRead;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;
use std::{
  collections::{BTreeMap, VecDeque},
  sync::atomic::AtomicU32,
};
use std::{
  fs::{self, File, OpenOptions},
  time::Duration,
};
use zeroize::Zeroize;

mod gui;
mod kernel;
#[cfg(any(test, feature = "test_vectors"))]
mod test_vector;

const APP_NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const APP_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const BACKUPS_DIR: &str = "backups";
const STATUS_DIR: &str = "status";
const PROGRESS_SAVE_INTERVAL_SEC: u64 = 10;
const CPU_PARALLEL_KEYS: usize = 64;
const CPU_CHUNK_SIZE: u128 = (CPU_PARALLEL_KEYS as u128) * 64;
const GPU_TEST_MODE: bool = false;
const BABY_TABLE_BITS: u32 = 22;
const GLOW_CURVE: [u8; 32] = [
  0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B,
  0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x41,
];
pub const UI_REFRESH_INTERVAL_MS: u64 = 200;
pub const DEFAULT_PUZZLE: u32 = 71;
pub const ALL_PUZZLES: u32 = 0;
pub const ALL_MODE_DEFAULT_TURN_SECS: u64 = 15;
pub const ALL_MODE_TURN_SECS_OPTIONS: [u64; 5] = [5, 10, 15, 30, 60];

static BENCH_CANCEL: AtomicBool = AtomicBool::new(false);
static TOTAL_CHECKED_LOW: AtomicU64 = AtomicU64::new(0);
static TOTAL_CHECKED_HIGH: AtomicU64 = AtomicU64::new(0);
static FOUND: AtomicBool = AtomicBool::new(false);
static PANIC_OCCURRED: AtomicBool = AtomicBool::new(false);
static ALL_MODE_DEADLINE_MS: AtomicU64 = AtomicU64::new(0);

pub static UNSOLVED_PUZZLES: &[PuzzleInfo] = &[
  PuzzleInfo {
    number: 71,
    n_bits: 70,
    range_start_hex: "400000000000000000",
    address: "1PWo3JeB9jrGwfHDNpdGK54CRas7fsVzXU",
    hash160: [
      0xf6, 0xf5, 0x43, 0x1d, 0x25, 0xbb, 0xf7, 0xb1, 0x2e, 0x8a, 0xdd, 0x9a, 0xf5, 0xe3, 0x47, 0x5c, 0x44, 0xa0, 0xa5, 0xb8,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 72,
    n_bits: 71,
    range_start_hex: "800000000000000000",
    address: "1JTK7s9YVYywfm5XUH7RNhHJH1LshCaRFR",
    hash160: [
      0xbf, 0x74, 0x13, 0xe8, 0xdf, 0x4e, 0x7a, 0x34, 0xce, 0x9d, 0xc1, 0x3e, 0x2f, 0x26, 0x48, 0x78, 0x3e, 0xc5, 0x4a, 0xdb,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 73,
    n_bits: 72,
    range_start_hex: "1000000000000000000",
    address: "12VVRNPi4SJqUTsp6FmqDqY5sGosDtysn4",
    hash160: [
      0x10, 0x5b, 0x7f, 0x25, 0x3f, 0x0e, 0xbd, 0x78, 0x43, 0xad, 0xae, 0xbb, 0xd8, 0x05, 0xc9, 0x44, 0xbf, 0xb8, 0x63, 0xe4,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 74,
    n_bits: 73,
    range_start_hex: "2000000000000000000",
    address: "1FWGcVDK3JGzCC3WtkYetULPszMaK2Jksv",
    hash160: [
      0x9f, 0x1a, 0xdb, 0x20, 0xba, 0xea, 0xcc, 0x38, 0xb3, 0xf4, 0x9f, 0x3d, 0xf6, 0x90, 0x6a, 0x0e, 0x48, 0xf2, 0xdf, 0x3d,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 76,
    n_bits: 75,
    range_start_hex: "8000000000000000000",
    address: "1DJh2eHFYQfACPmrvpyWc8MSTYKh7w9eRF",
    hash160: [
      0x86, 0xf9, 0xfe, 0xa5, 0xcd, 0xec, 0xf0, 0x33, 0x16, 0x1d, 0xd2, 0xf8, 0xf8, 0x56, 0x07, 0x68, 0xae, 0x0a, 0x6d, 0x14,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 77,
    n_bits: 76,
    range_start_hex: "10000000000000000000",
    address: "1Bxk4CQdqL9p22JEtDfdXMsng1XacifUtE",
    hash160: [
      0x78, 0x3c, 0x13, 0x8a, 0xc8, 0x1f, 0x6a, 0x52, 0x39, 0x85, 0x64, 0xbb, 0x17, 0x45, 0x55, 0x76, 0xe8, 0x52, 0x5b, 0x29,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 78,
    n_bits: 77,
    range_start_hex: "20000000000000000000",
    address: "15qF6X51huDjqTmF9BJgxXdt1xcj46Jmhb",
    hash160: [
      0x35, 0x00, 0x3c, 0x3e, 0xf8, 0x75, 0x9c, 0x92, 0x09, 0x2f, 0x84, 0x88, 0xfc, 0xa5, 0x9a, 0x04, 0x28, 0x59, 0x01, 0x8c,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 79,
    n_bits: 78,
    range_start_hex: "40000000000000000000",
    address: "1ARk8HWJMn8js8tQmGUJeQHjSE7KRkn2t8",
    hash160: [
      0x67, 0x67, 0x1d, 0x54, 0x90, 0xc2, 0x72, 0xe3, 0xab, 0x7d, 0xdd, 0x34, 0x03, 0x0d, 0x58, 0x77, 0x38, 0xdf, 0x33, 0xda,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 81,
    n_bits: 80,
    range_start_hex: "100000000000000000000",
    address: "15qsCm78whspNQFydGJQk5rexzxTQopnHZ",
    hash160: [
      0x35, 0x1e, 0x60, 0x5f, 0xac, 0x81, 0x39, 0x65, 0x95, 0x1b, 0xa4, 0x33, 0xb7, 0xc2, 0x95, 0x6b, 0xf8, 0xad, 0x95, 0xce,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 82,
    n_bits: 81,
    range_start_hex: "200000000000000000000",
    address: "13zYrYhhJxp6Ui1VV7pqa5WDhNWM45ARAC",
    hash160: [
      0x20, 0xd2, 0x8d, 0x4e, 0x87, 0x54, 0x39, 0x47, 0xc7, 0xe4, 0x91, 0x3b, 0xcd, 0xce, 0xaa, 0x16, 0xe2, 0xf8, 0xf0, 0x61,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 83,
    n_bits: 82,
    range_start_hex: "400000000000000000000",
    address: "14MdEb4eFcT3MVG5sPFG4jGLuHJSnt1Dk2",
    hash160: [
      0x24, 0xce, 0xf1, 0x84, 0x71, 0x4b, 0xbd, 0x03, 0x08, 0x33, 0x90, 0x4f, 0x52, 0x65, 0xc9, 0xc3, 0xe1, 0x2a, 0x95, 0xa2,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 84,
    n_bits: 83,
    range_start_hex: "800000000000000000000",
    address: "1CMq3SvFcVEcpLMuuH8PUcNiqsK1oicG2D",
    hash160: [
      0x7c, 0x99, 0xce, 0x73, 0xe1, 0x9f, 0x9f, 0xbf, 0xcc, 0xe4, 0x82, 0x5a, 0xe8, 0x82, 0x61, 0xe2, 0xb0, 0xb0, 0xb0, 0x40,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 86,
    n_bits: 85,
    range_start_hex: "2000000000000000000000",
    address: "1K3x5L6G57Y494fDqBfrojD28UJv4s5JcK",
    hash160: [
      0xc6, 0x01, 0x11, 0xed, 0x3d, 0x63, 0xb4, 0x96, 0x65, 0x74, 0x7b, 0x0e, 0x31, 0xeb, 0x38, 0x2d, 0xa5, 0x19, 0x35, 0x35,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 87,
    n_bits: 86,
    range_start_hex: "4000000000000000000000",
    address: "1PxH3K1Shdjb7gSEoTX7UPDZ6SH4qGPrvq",
    hash160: [
      0xfb, 0xc7, 0x08, 0xd6, 0x71, 0xc0, 0x3e, 0x26, 0x66, 0x1b, 0x9c, 0x08, 0xf7, 0x75, 0x98, 0xa5, 0x29, 0x85, 0x8b, 0x5e,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 88,
    n_bits: 87,
    range_start_hex: "8000000000000000000000",
    address: "16AbnZjZZipwHMkYKBSfswGWKDmXHjEpSf",
    hash160: [
      0x38, 0xa9, 0x68, 0xfd, 0xfb, 0x45, 0x76, 0x54, 0xc5, 0x1b, 0xcf, 0xc4, 0xf9, 0x17, 0x4d, 0x6e, 0xe4, 0x87, 0xbb, 0x41,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 89,
    n_bits: 88,
    range_start_hex: "10000000000000000000000",
    address: "19QciEHbGVNY4hrhfKXmcBBCrJSBZ6TaVt",
    hash160: [
      0x5c, 0x38, 0x62, 0x20, 0x3d, 0x1e, 0x44, 0xab, 0x3a, 0xf4, 0x41, 0x50, 0x3e, 0x22, 0xdb, 0x97, 0xb1, 0xc5, 0x09, 0x7e,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 91,
    n_bits: 90,
    range_start_hex: "40000000000000000000000",
    address: "1EzVHtmbN4fs4MiNk3ppEnKKhsmXYJ4s74",
    hash160: [
      0x99, 0x78, 0xf6, 0x1b, 0x92, 0xd1, 0x6c, 0x5f, 0x1a, 0x46, 0x3a, 0x09, 0x95, 0xdf, 0x70, 0xda, 0x1f, 0x7a, 0x7d, 0x2a,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 92,
    n_bits: 91,
    range_start_hex: "80000000000000000000000",
    address: "1AE8NzzgKE7Yhz7BWtAcAAxiFMbPo82NB5",
    hash160: [
      0x65, 0x34, 0xb3, 0x12, 0x08, 0xfe, 0x6e, 0x10, 0x0d, 0x29, 0xf9, 0xc9, 0xc7, 0x5a, 0xac, 0x8b, 0xf0, 0x6f, 0xbb, 0x38,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 93,
    n_bits: 92,
    range_start_hex: "100000000000000000000000",
    address: "17Q7tuG2JwFFU9rXVj3uZqRtioH3mx2Jad",
    hash160: [
      0x46, 0x30, 0x13, 0xcd, 0x41, 0x27, 0x9f, 0x2f, 0xd0, 0xc3, 0x1d, 0x0a, 0x16, 0xdb, 0x39, 0x72, 0xbf, 0xff, 0xac, 0x8d,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 94,
    n_bits: 93,
    range_start_hex: "200000000000000000000000",
    address: "1K6xGMUbs6ZTXBnhw1pippqwK6wjBWtNpL",
    hash160: [
      0xc6, 0x92, 0x7a, 0x00, 0x97, 0x0d, 0x01, 0x65, 0x32, 0x7d, 0x0a, 0x6d, 0xb7, 0x95, 0x0f, 0x05, 0x72, 0x0c, 0x29, 0x5c,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 96,
    n_bits: 95,
    range_start_hex: "800000000000000000000000",
    address: "15ANYzzCp5BFHcCnVFzXqyibpzgPLWaD8b",
    hash160: [
      0x2d, 0xa6, 0x3c, 0xbd, 0x25, 0x1d, 0x23, 0xc7, 0xb6, 0x33, 0xcb, 0x28, 0x7c, 0x09, 0xe6, 0xcf, 0x88, 0x8b, 0x3f, 0xe4,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 97,
    n_bits: 96,
    range_start_hex: "1000000000000000000000000",
    address: "18ywPwj39nGjqBrQJSzZVq2izR12MDpDr8",
    hash160: [
      0x57, 0x8d, 0x94, 0xdc, 0x6f, 0x40, 0xff, 0xf3, 0x5f, 0x91, 0xf6, 0xfb, 0xa9, 0xb7, 0x1c, 0x46, 0xb3, 0x61, 0xdf, 0xf2,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 98,
    n_bits: 97,
    range_start_hex: "2000000000000000000000000",
    address: "1CaBVPrwUxbQYYswu32w7Mj4HR4maNoJSX",
    hash160: [
      0x7e, 0xef, 0xdd, 0xd9, 0x79, 0xa1, 0xd6, 0xbb, 0x6f, 0x29, 0x75, 0x7a, 0x1f, 0x46, 0x35, 0x79, 0x77, 0x0b, 0xa5, 0x66,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 99,
    n_bits: 98,
    range_start_hex: "4000000000000000000000000",
    address: "1JWnE6p6UN7ZJBN7TtcbNDoRcjFtuDWoNL",
    hash160: [
      0xc0, 0x1b, 0xf4, 0x30, 0xa9, 0x7c, 0xbc, 0xda, 0xed, 0xdd, 0xba, 0x87, 0xef, 0x4e, 0xa2, 0x1c, 0x45, 0x6c, 0xeb, 0xdb,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 101,
    n_bits: 100,
    range_start_hex: "10000000000000000000000000",
    address: "1CKCVdbDJasYmhswB6HKZHEAnNaDpK7W4n",
    hash160: [
      0x7c, 0x1a, 0x77, 0x20, 0x5c, 0x03, 0xb9, 0x90, 0x96, 0x63, 0xb2, 0x03, 0x4f, 0xaa, 0x0b, 0x54, 0x4e, 0x6b, 0xc9, 0x6b,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 102,
    n_bits: 101,
    range_start_hex: "20000000000000000000000000",
    address: "1PXv28YxmYMaB8zxrKeZBW8dt2HK7RkRPX",
    hash160: [
      0xf7, 0x2b, 0x81, 0x29, 0x32, 0xf6, 0xd7, 0x10, 0x22, 0x33, 0x97, 0x1d, 0x65, 0xce, 0xc0, 0xa2, 0x2b, 0x89, 0xe1, 0x36,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 103,
    n_bits: 102,
    range_start_hex: "40000000000000000000000000",
    address: "1AcAmB6jmtU6AiEcXkmiNE9TNVPsj9DULf",
    hash160: [
      0x69, 0x5f, 0xd6, 0xdc, 0xf3, 0x3f, 0x47, 0x16, 0x6b, 0x25, 0xde, 0x96, 0x8b, 0x29, 0x32, 0xb3, 0x51, 0xb0, 0xaf, 0xc4,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 104,
    n_bits: 103,
    range_start_hex: "80000000000000000000000000",
    address: "1EQJvpsmhazYCcKX5Au6AZmZKRnzarMVZu",
    hash160: [
      0x93, 0x02, 0x2a, 0xf9, 0xa3, 0x8f, 0x3e, 0xbb, 0x0c, 0x3f, 0x15, 0xdd, 0x1c, 0x83, 0xf8, 0xfa, 0xda, 0xf6, 0x4e, 0x74,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 106,
    n_bits: 105,
    range_start_hex: "200000000000000000000000000",
    address: "18KsfuHuzQaBTNLASyj15hy4LuqPUo1FNB",
    hash160: [
      0x50, 0x5a, 0xaa, 0x63, 0xa5, 0xe2, 0x09, 0xdf, 0xb9, 0x0c, 0xee, 0x68, 0x3a, 0x8e, 0x22, 0x7a, 0x8c, 0x27, 0x8e, 0x47,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 107,
    n_bits: 106,
    range_start_hex: "400000000000000000000000000",
    address: "15EJFC5ZTs9nhsdvSUeBXjLAuYq3SWaxTc",
    hash160: [
      0x2e, 0x64, 0x4e, 0x46, 0xb0, 0x42, 0xff, 0xa8, 0x6d, 0xa3, 0x5c, 0x54, 0xd7, 0x27, 0x5f, 0x1a, 0xbe, 0x6d, 0x49, 0x11,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 108,
    n_bits: 107,
    range_start_hex: "800000000000000000000000000",
    address: "1HB1iKUqeffnVsvQsbpC6dNi1XKbyNuqao",
    hash160: [
      0xb1, 0x66, 0xc4, 0x4f, 0x12, 0xc7, 0xfc, 0x56, 0x5f, 0x37, 0xff, 0x62, 0x88, 0xee, 0x64, 0xe0, 0xf0, 0xec, 0x9a, 0x0b,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 109,
    n_bits: 108,
    range_start_hex: "1000000000000000000000000000",
    address: "1GvgAXVCbA8FBjXfWiAms4ytFeJcKsoyhL",
    hash160: [
      0xae, 0xb0, 0xa0, 0x19, 0x74, 0x42, 0xd4, 0xad, 0xe8, 0xef, 0x41, 0x44, 0x2d, 0x55, 0x7b, 0x0e, 0x22, 0xb8, 0x5a, 0xc0,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 111,
    n_bits: 110,
    range_start_hex: "4000000000000000000000000000",
    address: "1824ZJQ7nKJ9QFTRBqn7z7dHV5EGpzUpH3",
    hash160: [
      0x4c, 0xfc, 0x43, 0xfe, 0x12, 0xa3, 0x30, 0xc8, 0x16, 0x42, 0x51, 0xe3, 0x8c, 0x0c, 0x0c, 0x3c, 0x84, 0xcf, 0x86, 0xf6,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 112,
    n_bits: 111,
    range_start_hex: "8000000000000000000000000000",
    address: "18A7NA9FTsnJxWgkoFfPAFbQzuQxpRtCos",
    hash160: [
      0x4e, 0x81, 0xef, 0xec, 0x43, 0xc5, 0x19, 0x5a, 0xec, 0xa0, 0xe3, 0x87, 0x76, 0x64, 0x33, 0x04, 0x18, 0xb8, 0xe4, 0x8e,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 113,
    n_bits: 112,
    range_start_hex: "10000000000000000000000000000",
    address: "1NeGn21dUDDeqFQ63xb2SpgUuXuBLA4WT4",
    hash160: [
      0xed, 0x67, 0x33, 0x89, 0xe4, 0xb1, 0x29, 0x25, 0x31, 0x6f, 0x91, 0x66, 0xd5, 0x6d, 0x70, 0x18, 0x29, 0xe5, 0x3c, 0xf8,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 114,
    n_bits: 113,
    range_start_hex: "20000000000000000000000000000",
    address: "174SNxfqpdMGYy5YQcfLbSTK3MRNZEePoy",
    hash160: [
      0x42, 0x77, 0x30, 0x05, 0xf9, 0x59, 0x4c, 0xd1, 0x6b, 0x10, 0x98, 0x5d, 0x42, 0x84, 0x18, 0xac, 0xb7, 0xf3, 0x52, 0xec,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 116,
    n_bits: 115,
    range_start_hex: "80000000000000000000000000000",
    address: "1MnJ6hdhvK37VLmqcdEwqC3iFxyWH2PHUV",
    hash160: [
      0xe3, 0xf3, 0x81, 0xc3, 0x4a, 0x20, 0xda, 0x04, 0x97, 0x79, 0xb4, 0x4c, 0xae, 0x04, 0x17, 0xc7, 0xfb, 0x28, 0x98, 0xd0,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 117,
    n_bits: 116,
    range_start_hex: "100000000000000000000000000000",
    address: "1KNRfGWw7Q9Rmwsc6NT5zsdvEb9M2Wkj5Z",
    hash160: [
      0xc9, 0x7f, 0x95, 0x91, 0xe2, 0x86, 0x87, 0xbe, 0x1c, 0x4d, 0x97, 0x2e, 0x25, 0xbe, 0x7c, 0x37, 0x2a, 0x32, 0x21, 0xb4,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 118,
    n_bits: 117,
    range_start_hex: "200000000000000000000000000000",
    address: "1PJZPzvGX19a7twf5HyD2VvNiPdHLzm9F6",
    hash160: [
      0xf4, 0xa4, 0xe1, 0xc1, 0x1a, 0x5b, 0xbb, 0xd2, 0xfc, 0x13, 0x9d, 0x22, 0x18, 0x25, 0x40, 0x7c, 0x66, 0xe0, 0xb8, 0xb4,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 119,
    n_bits: 118,
    range_start_hex: "400000000000000000000000000000",
    address: "1GuBBhf61rnvRe4K8zu8vdQB3kHzwFqSy7",
    hash160: [
      0xae, 0x68, 0x04, 0xb3, 0x5c, 0x82, 0xf4, 0x7f, 0x8b, 0x0a, 0x42, 0xd8, 0xc5, 0xe5, 0x14, 0xfe, 0x5e, 0xf0, 0xa8, 0x83,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 121,
    n_bits: 120,
    range_start_hex: "1000000000000000000000000000000",
    address: "1GDSuiThEV64c166LUFC9uDcVdGjqkxKyh",
    hash160: [
      0xa6, 0xe4, 0x81, 0x85, 0x37, 0xe4, 0x2f, 0x7b, 0x3f, 0x02, 0x1d, 0xaa, 0x81, 0x03, 0x67, 0xda, 0xd4, 0xdd, 0xa1, 0x6f,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 122,
    n_bits: 121,
    range_start_hex: "2000000000000000000000000000000",
    address: "1Me3ASYt5JCTAK2XaC32RMeH34PdprrfDx",
    hash160: [
      0xe2, 0x63, 0xb6, 0x2e, 0xa2, 0x94, 0xb9, 0x65, 0x06, 0x15, 0xa1, 0x3b, 0x92, 0x6e, 0x75, 0x94, 0x4c, 0x82, 0x39, 0x90,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 123,
    n_bits: 122,
    range_start_hex: "4000000000000000000000000000000",
    address: "1CdufMQL892A69KXgv6UNBD17ywWqYpKut",
    hash160: [
      0x7f, 0xa4, 0x51, 0x50, 0x66, 0xba, 0x69, 0x05, 0xf8, 0x94, 0xb2, 0x07, 0x8f, 0x9a, 0xf7, 0xb1, 0x37, 0x91, 0x69, 0xcf,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 124,
    n_bits: 123,
    range_start_hex: "8000000000000000000000000000000",
    address: "1BkkGsX9ZM6iwL3zbqs7HWBV7SvosR6m8N",
    hash160: [
      0x75, 0xf7, 0x44, 0x67, 0xce, 0x72, 0x14, 0xf1, 0x76, 0x74, 0x06, 0xd5, 0xed, 0x12, 0x01, 0x2a, 0xa5, 0x23, 0xc4, 0x8e,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 126,
    n_bits: 125,
    range_start_hex: "20000000000000000000000000000000",
    address: "1AWCLZAjKbV1P7AHvaPNCKiB7ZWVDMxFiz",
    hash160: [
      0x68, 0x3e, 0xa8, 0xa1, 0xef, 0x06, 0xea, 0xda, 0x90, 0x55, 0x60, 0x17, 0xd4, 0x43, 0x23, 0xb5, 0xc0, 0x4e, 0x00, 0xf1,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 127,
    n_bits: 126,
    range_start_hex: "40000000000000000000000000000000",
    address: "1G6EFyBRU86sThN3SSt3GrHu1sA7w7nzi4",
    hash160: [
      0xa5, 0x87, 0x08, 0xaa, 0x98, 0xad, 0x35, 0xc8, 0x89, 0xbb, 0x36, 0xd8, 0x04, 0x9b, 0xf9, 0xe9, 0xca, 0xcf, 0xd0, 0x2a,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 128,
    n_bits: 127,
    range_start_hex: "80000000000000000000000000000000",
    address: "1MZ2L1gFrCtkkn6DnTT2e4PFUTHw9gNwaj",
    hash160: [
      0xe1, 0x70, 0xef, 0x51, 0x46, 0x89, 0xd7, 0x23, 0x0d, 0xa3, 0x62, 0xa0, 0xc1, 0x21, 0xa0, 0x77, 0x23, 0x55, 0x05, 0x12,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 129,
    n_bits: 128,
    range_start_hex: "100000000000000000000000000000000",
    address: "1Hz3uv3nNZzBVMXLGadCucgjiCs5W9vaGz",
    hash160: [
      0xba, 0x4c, 0x27, 0x48, 0x36, 0x0a, 0x6b, 0x66, 0x26, 0x3e, 0x11, 0xd1, 0xdc, 0x86, 0x58, 0x46, 0x3c, 0xa5, 0xff, 0x18,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 131,
    n_bits: 130,
    range_start_hex: "400000000000000000000000000000000",
    address: "16zRPnT8znwq42q7XeMkZUhb1bKqgRogyy",
    hash160: [
      0x41, 0xb4, 0xb3, 0x6a, 0x6c, 0x03, 0x65, 0x68, 0x97, 0x23, 0x80, 0x17, 0x7e, 0xca, 0x29, 0x16, 0xca, 0xcd, 0x71, 0xde,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 132,
    n_bits: 131,
    range_start_hex: "800000000000000000000000000000000",
    address: "1KrU4dHE5WrW8rhWDsTRjR21r8t3dsrS3R",
    hash160: [
      0xce, 0xcd, 0x3c, 0xa4, 0x31, 0x96, 0x51, 0xbd, 0x3a, 0xfd, 0x1e, 0x23, 0xab, 0x66, 0xe1, 0x11, 0xed, 0x38, 0xd1, 0x6d,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 133,
    n_bits: 132,
    range_start_hex: "1000000000000000000000000000000000",
    address: "17uDfp5r4n441xkgLFmhNoSW1KWp6xVLD",
    hash160: [
      0x01, 0x4e, 0x15, 0xe4, 0xea, 0x6d, 0xa4, 0x60, 0xcc, 0x78, 0x35, 0xe2, 0x62, 0x67, 0x6b, 0xaa, 0x37, 0x98, 0x8e, 0x4f,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 134,
    n_bits: 133,
    range_start_hex: "2000000000000000000000000000000000",
    address: "13A3JrvXmvg5w9XGvyyR4JEJqiLz8ZySY3",
    hash160: [
      0x17, 0xa5, 0xeb, 0xfa, 0xf6, 0x2e, 0x73, 0xf1, 0x49, 0xe3, 0x3b, 0xa6, 0x74, 0x83, 0x68, 0x01, 0xf1, 0x3a, 0x80, 0xb9,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 136,
    n_bits: 135,
    range_start_hex: "8000000000000000000000000000000000",
    address: "1UDHPdovvR985NrWSkdWQDEQ1xuRiTALq",
    hash160: [
      0x05, 0x25, 0x7b, 0xe4, 0xb5, 0x7e, 0xe4, 0x3f, 0xc0, 0x97, 0x62, 0xd5, 0xd3, 0xa9, 0xad, 0x4a, 0x6e, 0x1a, 0x03, 0x64,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 137,
    n_bits: 136,
    range_start_hex: "10000000000000000000000000000000000",
    address: "15nf31J46iLuK1ZkTnqHo7WgN5cARFK3RA",
    hash160: [
      0x34, 0x82, 0xf8, 0x98, 0x6e, 0x13, 0xc0, 0x18, 0x69, 0x20, 0x53, 0xa7, 0x84, 0x48, 0x1c, 0x63, 0xa3, 0x55, 0x4c, 0x9c,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 138,
    n_bits: 137,
    range_start_hex: "20000000000000000000000000000000000",
    address: "1Ab4vzG6wEQBDNQM1B2bvUz4fqXXdFk2WT",
    hash160: [
      0x69, 0x2a, 0x8e, 0x58, 0x38, 0x66, 0xfc, 0x90, 0x56, 0xf5, 0xc6, 0x1a, 0x45, 0x96, 0x9f, 0xb9, 0xd8, 0x68, 0xa0, 0x8c,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 139,
    n_bits: 138,
    range_start_hex: "40000000000000000000000000000000000",
    address: "1Fz63c775VV9fNyj25d9Xfw3YHE6sKCxbt",
    hash160: [
      0xa4, 0x5d, 0xae, 0x9c, 0xd5, 0xd3, 0xfd, 0xe2, 0x1e, 0x5a, 0xa9, 0xa9, 0x53, 0x67, 0xd1, 0x07, 0x26, 0x7b, 0x3b, 0x8a,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 140,
    n_bits: 139,
    range_start_hex: "80000000000000000000000000000000000",
    address: "1QKBaU6WAeycb3DbKbLBkX7vJiaS8r42Xo",
    hash160: [
      0xff, 0xbb, 0x35, 0xa7, 0xbb, 0x9b, 0xbe, 0x16, 0xc1, 0xaa, 0x25, 0x34, 0xf7, 0xff, 0x11, 0xd5, 0x9c, 0x8e, 0x3d, 0x1a,
    ],
    pubkey_hex: Some("031f6a332d3c5c4f2de2378c012f429cd109ba07d69690c6c701b6bb87860d6640"),
  },
  PuzzleInfo {
    number: 141,
    n_bits: 140,
    range_start_hex: "100000000000000000000000000000000000",
    address: "1CD91Vm97mLQvXhrnoMChhJx4TP9MaQkJo",
    hash160: [
      0x7a, 0xf5, 0x0f, 0x73, 0xfd, 0x58, 0x0f, 0x17, 0x13, 0xaf, 0x3a, 0x6f, 0x9c, 0x5d, 0xe4, 0x96, 0x43, 0xec, 0x6f, 0xc6,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 142,
    n_bits: 141,
    range_start_hex: "200000000000000000000000000000000000",
    address: "15MnK2jXPqTMURX4xC3h4mAZxyCcaWWEDD",
    hash160: [
      0x2f, 0xce, 0xa5, 0x5e, 0x6d, 0x02, 0x7a, 0x2b, 0xa7, 0xc7, 0xeb, 0xe9, 0x5e, 0xed, 0xf4, 0x77, 0x66, 0x73, 0x0f, 0xe2,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 143,
    n_bits: 142,
    range_start_hex: "400000000000000000000000000000000000",
    address: "13N66gCzWWHEZBxhVxG18P8wyjEWF9Yoi1",
    hash160: [
      0x19, 0xed, 0x3e, 0x03, 0xd1, 0x9d, 0xdc, 0xed, 0xd5, 0xfa, 0x86, 0x54, 0x3b, 0xe8, 0x20, 0xb3, 0xa7, 0x95, 0x16, 0x50,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 144,
    n_bits: 143,
    range_start_hex: "800000000000000000000000000000000000",
    address: "1NevxKDYuDcCh1ZMMi6ftmWwGrZKC6j7Ux",
    hash160: [
      0xed, 0x87, 0x12, 0x00, 0x66, 0xe2, 0x44, 0xff, 0x53, 0x31, 0xd5, 0xf8, 0x62, 0x58, 0x73, 0xd7, 0xa3, 0xac, 0xc3, 0x9c,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 145,
    n_bits: 144,
    range_start_hex: "1000000000000000000000000000000000000",
    address: "19GpszRNUej5yYqxXoLnbZWKew3KdVLkXg",
    hash160: [
      0x5a, 0xbf, 0x36, 0x93, 0x88, 0xde, 0xb8, 0x07, 0x27, 0x41, 0xb4, 0xeb, 0x43, 0xef, 0x10, 0xfa, 0x93, 0x88, 0xa7, 0x29,
    ],
    pubkey_hex: Some("03afdda497369e219a2c1c369954a930e4d3740968e5e4352475bcffce3140dae5"),
  },
  PuzzleInfo {
    number: 146,
    n_bits: 145,
    range_start_hex: "2000000000000000000000000000000000000",
    address: "1M7ipcdYHey2Y5RZM34MBbpugghmjaV89P",
    hash160: [
      0xdc, 0xa7, 0xeb, 0xfb, 0x78, 0xce, 0x21, 0x88, 0x43, 0x00, 0xf1, 0x33, 0xd8, 0x92, 0x44, 0xbc, 0x4b, 0x1b, 0x75, 0x6f,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 147,
    n_bits: 146,
    range_start_hex: "4000000000000000000000000000000000000",
    address: "18aNhurEAJsw6BAgtANpexk5ob1aGTwSeL",
    hash160: [
      0x53, 0x18, 0xb9, 0xd7, 0xfc, 0xc9, 0x38, 0x73, 0xf7, 0x68, 0x72, 0x5e, 0xb6, 0x8b, 0xa2, 0xc9, 0x24, 0xbb, 0x07, 0xee,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 148,
    n_bits: 147,
    range_start_hex: "8000000000000000000000000000000000000",
    address: "1FwZXt6EpRT7Fkndzv6K4b4DFoT4trbMrV",
    hash160: [
      0xa3, 0xe3, 0x61, 0x2e, 0x58, 0x6f, 0xd2, 0x06, 0xef, 0xb8, 0xee, 0xe6, 0xcc, 0xd5, 0x83, 0x18, 0xe1, 0x82, 0x82, 0x9a,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 149,
    n_bits: 148,
    range_start_hex: "10000000000000000000000000000000000000",
    address: "1CXvTzR6qv8wJ7eprzUKeWxyGcHwDYP1i2",
    hash160: [
      0x7e, 0x82, 0x7e, 0x3b, 0x90, 0xda, 0x24, 0xc2, 0xa1, 0x5f, 0x7b, 0x67, 0xe3, 0xbb, 0xec, 0xe3, 0x99, 0x55, 0xa5, 0xd0,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 150,
    n_bits: 149,
    range_start_hex: "20000000000000000000000000000000000000",
    address: "1MUJSJYtGPVGkBCTqGspnxyHahpt5Te8jy",
    hash160: [
      0xe0, 0x8c, 0x4d, 0x3b, 0xc9, 0xcf, 0x2b, 0x3e, 0x2c, 0xb8, 0x8d, 0xe2, 0xbf, 0xaa, 0x4f, 0xe8, 0xc7, 0xaa, 0x3f, 0x24,
    ],
    pubkey_hex: Some("03137807790ea7dc6e97901c2bc87411f45ed74a5629315c4e4b03a0a102250c49"),
  },
  PuzzleInfo {
    number: 151,
    n_bits: 150,
    range_start_hex: "40000000000000000000000000000000000000",
    address: "13Q84TNNvgcL3HJiqQPvyBb9m4hxjS3jkV",
    hash160: [
      0x1a, 0x4f, 0xb6, 0x32, 0xf0, 0xde, 0x0c, 0x53, 0xa0, 0xa3, 0x1d, 0x57, 0xf8, 0x40, 0xa1, 0x9e, 0x56, 0xc6, 0x45, 0xee,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 152,
    n_bits: 151,
    range_start_hex: "80000000000000000000000000000000000000",
    address: "1LuUHyrQr8PKSvbcY1v1PiuGuqFjWpDumN",
    hash160: [
      0xda, 0x56, 0xcd, 0x81, 0x5f, 0xa2, 0xf0, 0xd6, 0xa4, 0xce, 0x6d, 0x25, 0xed, 0x7b, 0x1a, 0x01, 0xd9, 0xf9, 0xbc, 0x6b,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 153,
    n_bits: 152,
    range_start_hex: "100000000000000000000000000000000000000",
    address: "18192XpzzdDi2K11QVHR7td2HcPS6Qs5vg",
    hash160: [
      0x4c, 0xcf, 0x94, 0xa1, 0xb0, 0xef, 0xd6, 0x3c, 0xdd, 0xee, 0xe0, 0xef, 0x5e, 0xee, 0x5e, 0xbe, 0x72, 0x0c, 0xfc, 0xbf,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 154,
    n_bits: 153,
    range_start_hex: "200000000000000000000000000000000000000",
    address: "1NgVmsCCJaKLzGyKLFJfVequnFW9ZvnMLN",
    hash160: [
      0xed, 0xd2, 0xe2, 0x06, 0x82, 0x5f, 0xa8, 0x94, 0x9d, 0x13, 0x04, 0xcd, 0x82, 0xc0, 0x8d, 0x64, 0xb2, 0x22, 0xf2, 0xeb,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 155,
    n_bits: 154,
    range_start_hex: "400000000000000000000000000000000000000",
    address: "1AoeP37TmHdFh8uN72fu9AqgtLrUwcv2wJ",
    hash160: [
      0x6b, 0x8b, 0x78, 0x30, 0xf7, 0x3c, 0x5b, 0xf9, 0xe8, 0xbe, 0xb9, 0xf1, 0x61, 0xad, 0x82, 0xb3, 0xbd, 0xe9, 0x92, 0xe4,
    ],
    pubkey_hex: Some("035cd1854cae45391ca4ec428cc7e6c7d9984424b954209a8eea197b9e364c05f6"),
  },
  PuzzleInfo {
    number: 156,
    n_bits: 155,
    range_start_hex: "800000000000000000000000000000000000000",
    address: "1FTpAbQa4h8trvhQXjXnmNhqdiGBd1oraE",
    hash160: [
      0x9e, 0xa3, 0xf2, 0x9a, 0xae, 0xdf, 0x7d, 0xa1, 0x0b, 0x14, 0x88, 0x93, 0x4c, 0x50, 0xa3, 0x9e, 0x27, 0x1b, 0x0b, 0x64,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 157,
    n_bits: 156,
    range_start_hex: "1000000000000000000000000000000000000000",
    address: "14JHoRAdmJg3XR4RjMDh6Wed6ft6hzbQe9",
    hash160: [
      0x24, 0x2d, 0x79, 0x0e, 0x5a, 0x16, 0x80, 0x43, 0xc7, 0x6f, 0x05, 0x39, 0xfd, 0x89, 0x4b, 0x73, 0xee, 0x67, 0xb3, 0xb3,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 158,
    n_bits: 157,
    range_start_hex: "2000000000000000000000000000000000000000",
    address: "19z6waranEf8CcP8FqNgdwUe1QRxvUNKBG",
    hash160: [
      0x62, 0x8d, 0xac, 0xeb, 0xb0, 0xfa, 0xa7, 0xf8, 0x16, 0x70, 0xe1, 0x74, 0xca, 0x4c, 0x8a, 0x95, 0xa7, 0xe3, 0x70, 0x29,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 159,
    n_bits: 158,
    range_start_hex: "4000000000000000000000000000000000000000",
    address: "14u4nA5sugaswb6SZgn5av2vuChdMnD9E5",
    hash160: [
      0x2a, 0xc1, 0x29, 0x5b, 0x4e, 0x54, 0xb3, 0xf1, 0x5b, 0xb0, 0xa9, 0x9f, 0x84, 0x01, 0x8d, 0x20, 0x82, 0x49, 0x56, 0x45,
    ],
    pubkey_hex: None,
  },
  PuzzleInfo {
    number: 160,
    n_bits: 159,
    range_start_hex: "8000000000000000000000000000000000000000",
    address: "1NBC8uXJy1GiJ6drkiZa1WuKn51ps7EPTv",
    hash160: [
      0xe8, 0x48, 0x18, 0xe1, 0xbf, 0x7f, 0x69, 0x9a, 0xa6, 0xe2, 0x8e, 0xf9, 0xed, 0xfb, 0x58, 0x20, 0x99, 0x09, 0x92, 0x92,
    ],
    pubkey_hex: Some("02e0a8b039282faf6fe0fd769cfbc4b6b4cf8758ba68220eac420e32b91ddfa673"),
  },
];
pub static ACTIVE_PACKAGE: std::sync::Mutex<Option<KernelRegistry>> = std::sync::Mutex::new(None);

pub type BenchProgressFn = Box<dyn Fn(usize, usize, &str, Option<f64>) + Send>;

lazy_static::lazy_static! {
  static ref DASHBOARD: Arc<Mutex<Vec<ThreadStatus>>> = Arc::new(Mutex::new(Vec::new()));
  static ref FOUND_KEY_HEX: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  static ref LAST_GPU_ERROR: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

thread_local! {
  static LOCAL_SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}

// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct U256 {
  pub lo: u128,
  pub hi: u128,
}

impl U256 {
  pub const ZERO: U256 = U256 { lo: 0, hi: 0 };
  pub const ONE: U256 = U256 { lo: 1, hi: 0 };

  #[inline]
  pub fn from_u128(v: u128) -> Self {
    Self { lo: v, hi: 0 }
  }

  pub fn from_hex(s: &str) -> Self {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if s.is_empty() {
      return Self::ZERO;
    }

    if s.len() <= 32 {
      let lo = u128::from_str_radix(s, 16).expect("valid hex");
      return Self { lo, hi: 0 };
    }

    let hi_len = s.len() - 32;
    let hi = u128::from_str_radix(&s[..hi_len], 16).expect("valid hex hi");
    let lo = u128::from_str_radix(&s[hi_len..], 16).expect("valid hex lo");
    Self { lo, hi }
  }

  #[inline]
  pub fn bit(bit: u32) -> Self {
    debug_assert!(bit < 256);
    if bit < 128 {
      Self { lo: 1u128 << bit, hi: 0 }
    } else {
      Self {
        lo: 0,
        hi: 1u128 << (bit - 128),
      }
    }
  }

  #[inline]
  pub fn saturating_add(
    self,
    rhs: Self,
  ) -> Self {
    let (lo, c) = self.lo.overflowing_add(rhs.lo);
    let hi = self.hi.saturating_add(rhs.hi).saturating_add(c as u128);
    Self { lo, hi }
  }

  #[inline]
  pub fn wrapping_add(
    self,
    rhs: Self,
  ) -> Self {
    let (lo, c) = self.lo.overflowing_add(rhs.lo);
    let hi = self.hi.wrapping_add(rhs.hi).wrapping_add(c as u128);
    Self { lo, hi }
  }

  #[inline]
  pub fn wrapping_mul(
    self,
    rhs: Self,
  ) -> Self {
    let (ll_lo, ll_hi) = umul128(self.lo, rhs.lo);
    let cross = self.lo.wrapping_mul(rhs.hi).wrapping_add(self.hi.wrapping_mul(rhs.lo));
    Self {
      lo: ll_lo,
      hi: ll_hi.wrapping_add(cross),
    }
  }

  #[inline]
  pub fn saturating_sub(
    self,
    rhs: Self,
  ) -> Self {
    if self < rhs {
      return Self::ZERO;
    }
    let (lo, b) = self.lo.overflowing_sub(rhs.lo);
    let hi = self.hi - rhs.hi - (b as u128);
    Self { lo, hi }
  }

  #[inline]
  pub fn bit_and(
    self,
    rhs: Self,
  ) -> Self {
    Self {
      lo: self.lo & rhs.lo,
      hi: self.hi & rhs.hi,
    }
  }

  #[inline]
  pub fn fits_u128(self) -> bool {
    self.hi == 0
  }

  #[inline]
  pub fn as_u128(self) -> Option<u128> {
    if self.hi == 0 { Some(self.lo) } else { None }
  }

  pub fn to_be_bytes(self) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&self.hi.to_be_bytes());
    out[16..].copy_from_slice(&self.lo.to_be_bytes());
    out
  }

  pub fn to_hex(self) -> String {
    if self.hi == 0 {
      format!("{:X}", self.lo)
    } else {
      format!("{:X}{:032X}", self.hi, self.lo)
    }
  }

  #[inline]
  pub fn wrapping_add_u128(
    self,
    rhs: u128,
  ) -> Self {
    let (lo, c) = self.lo.overflowing_add(rhs);
    Self {
      lo,
      hi: self.hi.wrapping_add(c as u128),
    }
  }

  #[inline]
  pub fn saturating_add_u128(
    self,
    rhs: u128,
  ) -> Self {
    let (lo, c) = self.lo.overflowing_add(rhs);
    let (hi, o) = self.hi.overflowing_add(c as u128);
    if o {
      Self {
        lo: u128::MAX,
        hi: u128::MAX,
      }
    } else {
      Self { lo, hi }
    }
  }

  #[inline]
  pub fn saturating_sub_u128(
    self,
    rhs: u128,
  ) -> Self {
    if self.hi == 0 {
      Self {
        lo: self.lo.saturating_sub(rhs),
        hi: 0,
      }
    } else {
      let (lo, b) = self.lo.overflowing_sub(rhs);
      Self {
        lo,
        hi: self.hi - (b as u128),
      }
    }
  }

  #[inline]
  pub fn is_zero(self) -> bool {
    self.lo == 0 && self.hi == 0
  }

  #[inline]
  pub fn cmp_u128(
    self,
    rhs: u128,
  ) -> std::cmp::Ordering {
    if self.hi != 0 { std::cmp::Ordering::Greater } else { self.lo.cmp(&rhs) }
  }

  #[inline]
  pub fn min(
    self,
    other: Self,
  ) -> Self {
    if self <= other { self } else { other }
  }

  #[inline]
  pub fn max(
    self,
    other: Self,
  ) -> Self {
    if self >= other { self } else { other }
  }
}

impl PartialOrd for U256 {
  fn partial_cmp(
    &self,
    other: &Self,
  ) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for U256 {
  fn cmp(
    &self,
    other: &Self,
  ) -> std::cmp::Ordering {
    match self.hi.cmp(&other.hi) {
      std::cmp::Ordering::Equal => self.lo.cmp(&other.lo),
      o => o,
    }
  }
}

impl std::fmt::Display for U256 {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    f.write_str(&self.to_hex())
  }
}

// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..

#[derive(Clone, Copy, Debug)]
pub struct PuzzleInfo {
  pub number: u32,
  pub n_bits: u32,
  pub range_start_hex: &'static str,
  pub address: &'static str,
  pub hash160: [u8; 20],
  pub pubkey_hex: Option<&'static str>,
}

impl PuzzleInfo {
  #[inline]
  pub fn range_start(&self) -> U256 {
    U256::from_hex(self.range_start_hex)
  }

  #[inline]
  pub fn total_keys(&self) -> U256 {
    U256::bit(self.n_bits)
  }

  #[inline]
  pub fn n_mask(&self) -> U256 {
    self.total_keys().saturating_sub(U256::ONE)
  }

  #[inline]
  pub fn range_end(&self) -> U256 {
    let start = self.range_start();
    let mask = self.n_mask();

    U256 {
      lo: start.lo | mask.lo,
      hi: start.hi | mask.hi,
    }
  }

  #[inline]
  pub fn fits_u128(&self) -> bool {
    self.n_bits <= 127
  }

  #[inline]
  pub fn range_start_u128(&self) -> Option<u128> {
    self.range_start().as_u128()
  }

  #[inline]
  pub fn total_keys_u128(&self) -> Option<u128> {
    self.total_keys().as_u128()
  }

  #[inline]
  pub fn n_mask_u128(&self) -> Option<u128> {
    self.n_mask().as_u128()
  }

  #[inline]
  pub fn has_pubkey(&self) -> bool {
    self.pubkey_hex.is_some()
  }

  pub fn pubkey_compressed(&self) -> Option<[u8; 33]> {
    let h = self.pubkey_hex?;
    if h.len() != 66 {
      return None;
    }

    let mut out = [0u8; 33];
    for i in 0..33 {
      out[i] = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SearchMode {
  Sequence,
  Pcg,
}

#[derive(Clone)]
struct ThreadStatus {
  worker_type: WorkerType,
  checked: u128,
  last_i: u128,

  index_hex: String,
  key_hex: String,
  speed_kps: f64,
  last_update: Instant,
  last_checked: u128,
}

impl Default for ThreadStatus {
  fn default() -> Self {
    Self {
      worker_type: WorkerType::Cpu,
      checked: 0,
      last_i: 0,
      index_hex: String::from("0"),
      key_hex: String::from(""),
      speed_kps: 0.0,
      last_update: Instant::now(),
      last_checked: 0,
    }
  }
}

#[derive(Clone, Copy, Debug)]
struct Chunk {
  start_i: U256,
  len: u128,
}

pub struct AllocState {
  next_i: U256,
  pending: VecDeque<Chunk>,
}

#[derive(Clone, Copy)]
enum WorkerType {
  Cpu,
  Gpu,
}

enum AllocCommand {
  RequestChunk { chunk_size: u128, reply: Sender<Option<Chunk>> },
  ReportFinished { start_i: U256, len: u128 },
  SaveNow,
  Stop,
}

struct GpuSolver {
  stream: Arc<CudaStream>,
  function: CudaFunction,
  found_buffer: CudaSlice<u64>,

  block_size: u32,
  grid_size: u32,
  parallel_keys: u32,

  kernel_type: KernelType,
  baby_x: Option<CudaSlice<u32>>,
  baby_idx: Option<CudaSlice<u64>>,
  use_baby: i32,
}

impl GpuSolver {
  fn new(cfg: &GpuLaunchConfig) -> Result<Self, Box<dyn std::error::Error>> {
    let ctx = CudaContext::new(cfg.device_id as usize)
      .map_err(|e| format!("Failed to create CUDA context on device {} ({}): {:?}", cfg.device_id, cfg.name, e))?;
    crate::app_log!("Using GPU [{}]: {}  ({} SMs)", cfg.device_id, cfg.name, cfg.sm_count);

    let stream = ctx.default_stream();
    let kernel = with_active_package(|active| load_kernel_with_registry(cfg.kernel_type, &cfg.kernel_size, Some(cfg.compute_cap.as_str()), active))?;

    crate::app_log!(
      "Kernel image: {}  ({} bytes, CUBIN, GPU arch {})",
      kernel.image_name,
      kernel.image_bytes(),
      cfg.compute_cap
    );
    crate::app_log!(
      "Launch cfg  : block={}  grid={}  keys/thread={}  chunk={:#X}  type={}  size={}  arch={}",
      cfg.block_size,
      cfg.grid_size,
      cfg.parallel_keys,
      cfg.chunk_size,
      cfg.kernel_type.display_name(),
      cfg.kernel_size,
      cfg.compute_cap
    );

    let module = ctx
      .load_module(kernel.to_module_image())
      .map_err(|e| format_cuda_oom_or("load CUBIN module", &cfg.name, e))?;

    let function = module
      .load_function("generate_and_check_keys")
      .map_err(|e| format!("Failed to load kernel entry 'generate_and_check_keys' on {}: {:?}", cfg.name, e))?;

    let mut found_buffer = stream
      .alloc_zeros::<u64>(1)
      .map_err(|e| format_cuda_oom_or("allocate found_buffer", &cfg.name, e))?;

    stream
      .memcpy_htod(&[u64::MAX], &mut found_buffer)
      .map_err(|e| format!("Failed to init found_buffer on {}: {:?}", cfg.name, e))?;

    const BABY_TABLE_SIZE: usize = 1usize << BABY_TABLE_BITS;

    let (baby_x, baby_idx, use_baby) = if cfg.kernel_type == KernelType::ProjectiveBaby {
      let x_mb = (BABY_TABLE_SIZE * 32) / (1024 * 1024);
      let idx_mb = (BABY_TABLE_SIZE * 8) / (1024 * 1024);

      crate::app_log!(
        "[Baby] Allocating hybrid table: {} entries (~{} MB X + ~{} MB idx = ~{} MB total)",
        BABY_TABLE_SIZE,
        x_mb,
        idx_mb,
        x_mb + idx_mb
      );

      let bx = stream
        .alloc_zeros::<u32>(BABY_TABLE_SIZE * 8)
        .map_err(|e| format_cuda_oom_or("allocate Baby-step X table", &cfg.name, e))?;

      let bi = stream
        .alloc_zeros::<u64>(BABY_TABLE_SIZE)
        .map_err(|e| format_cuda_oom_or("allocate Baby-step index table", &cfg.name, e))?;

      crate::app_log!("[Baby] use_baby=0 (table empty - sequential HASH160/pubkey path only)");

      (Some(bx), Some(bi), 0i32)
    } else {
      (None, None, 0i32)
    };

    let storage_limit = cfg.kernel_size.parse::<u32>().unwrap_or(cfg.parallel_keys).max(1);

    if cfg.parallel_keys > storage_limit {
      crate::app_log!(
        "[GPU] parallel_keys={} > BATCH_STORAGE_LIMIT={} - \
         kernel will process work in successive storage-sized batches",
        cfg.parallel_keys,
        storage_limit
      );
    }

    Ok(Self {
      stream,
      function,
      found_buffer,
      block_size: cfg.block_size,
      grid_size: cfg.grid_size,
      parallel_keys: cfg.parallel_keys,
      kernel_type: cfg.kernel_type,
      baby_x,
      baby_idx,
      use_baby,
    })
  }

  #[inline]
  fn effective_keys_per_thread(&self) -> u32 {
    self.parallel_keys.max(1)
  }

  #[inline]
  fn launch_capacity(&self) -> u128 {
    (self.block_size as u128)
      .saturating_mul(self.grid_size as u128)
      .saturating_mul(self.effective_keys_per_thread() as u128)
      .max(1)
  }

  #[allow(clippy::too_many_arguments)]
  fn search_batch(
    &mut self,
    solving_mode: SearchMode,
    start_i: U256,
    actual_count: u128,
    range_start: U256,
    n_bits: u32,
    n_mask: U256,
    total_bits: u32,
    target_h160: [u8; 20],
    target_pubkey: Option<[u8; 33]>,
    pcg: PcgConsts,
  ) -> Result<Option<U256>, Box<dyn std::error::Error>> {
    let launch_cap = self.launch_capacity();
    let count_u64 = actual_count.min(launch_cap).min(u128::from(u64::MAX)) as u64;
    let effective_kpt = self.effective_keys_per_thread() as u64;

    let (mode_flag, block, grid, parallel_keys, chunk_size) = if GPU_TEST_MODE {
      (true, 1u32, 1u32, 1u64, 1u64)
    } else {
      let mode_flag = matches!(solving_mode, SearchMode::Sequence);
      (mode_flag, self.block_size, self.grid_size, effective_kpt, count_u64)
    };

    let (start_i_0, start_i_1, start_i_2, start_i_3) = u256_limbs(start_i);
    let (range_0, range_1, range_2, range_3) = u256_limbs(range_start);
    let (a0, a1, a2, a3) = u256_limbs(pcg.a);
    let (b0, b1, b2, b3) = u256_limbs(pcg.b);
    let (m0, m1, m2, m3) = u256_limbs(n_mask);

    self
      .stream
      .memcpy_htod(&[u64::MAX], &mut self.found_buffer)
      .map_err(|e| format!("Failed to reset found_buffer before launch: {:?}", e))?;

    let launch_cfg = LaunchConfig {
      grid_dim: (grid, 1, 1),
      block_dim: (block, 1, 1),
      shared_mem_bytes: 0,
    };

    let mut builder = self.stream.launch_builder(&self.function);

    builder.arg(&mode_flag);
    builder.arg(&parallel_keys);
    builder.arg(&start_i_0);
    builder.arg(&start_i_1);
    builder.arg(&start_i_2);
    builder.arg(&start_i_3);
    builder.arg(&chunk_size);
    builder.arg(&a0);
    builder.arg(&a1);
    builder.arg(&a2);
    builder.arg(&a3);
    builder.arg(&b0);
    builder.arg(&b1);
    builder.arg(&b2);
    builder.arg(&b3);
    builder.arg(&m0);
    builder.arg(&m1);
    builder.arg(&m2);
    builder.arg(&m3);
    builder.arg(&range_0);
    builder.arg(&range_1);
    builder.arg(&range_2);
    builder.arg(&range_3);
    builder.arg(&mut self.found_buffer);

    let total_bits_arg: u32 = total_bits;
    let match_mode: u32 = if target_pubkey.is_some() { 1 } else { 0 };
    let th0 = u32::from_le_bytes([target_h160[0], target_h160[1], target_h160[2], target_h160[3]]);
    let th1 = u32::from_le_bytes([target_h160[4], target_h160[5], target_h160[6], target_h160[7]]);
    let th2 = u32::from_le_bytes([target_h160[8], target_h160[9], target_h160[10], target_h160[11]]);
    let th3 = u32::from_le_bytes([target_h160[12], target_h160[13], target_h160[14], target_h160[15]]);
    let th4 = u32::from_le_bytes([target_h160[16], target_h160[17], target_h160[18], target_h160[19]]);

    let (pk_prefix, pkx) = if let Some(pk) = target_pubkey.as_ref() {
      pack_pubkey_x_limbs(pk)
    } else {
      (0u32, [0u32; 8])
    };

    let (pkx0, pkx1, pkx2, pkx3, pkx4, pkx5, pkx6, pkx7) = (pkx[0], pkx[1], pkx[2], pkx[3], pkx[4], pkx[5], pkx[6], pkx[7]);

    builder.arg(&total_bits_arg);
    builder.arg(&match_mode);
    builder.arg(&th0);
    builder.arg(&th1);
    builder.arg(&th2);
    builder.arg(&th3);
    builder.arg(&th4);
    builder.arg(&pk_prefix);
    builder.arg(&pkx0);
    builder.arg(&pkx1);
    builder.arg(&pkx2);
    builder.arg(&pkx3);
    builder.arg(&pkx4);
    builder.arg(&pkx5);
    builder.arg(&pkx6);
    builder.arg(&pkx7);

    if self.kernel_type == KernelType::ProjectiveBaby {
      let bx = self
        .baby_x
        .as_ref()
        .ok_or("baby_x missing for Baby kernel - GpuSolver was not constructed as ProjectiveBaby")?;
      let bi = self
        .baby_idx
        .as_ref()
        .ok_or("baby_idx missing for Baby kernel - GpuSolver was not constructed as ProjectiveBaby")?;
      builder.arg(bx);
      builder.arg(bi);
      builder.arg(&self.use_baby);
    }

    unsafe {
      builder.launch(launch_cfg).map_err(|e| {
        format_cuda_oom_or(
          &format!("kernel launch (block={} grid={} keys/thread={})", block, grid, parallel_keys),
          "GPU",
          e,
        )
      })?;
    }

    self
      .stream
      .synchronize()
      .map_err(|e| format_cuda_oom_or("stream synchronize after kernel", "GPU", e))?;

    let host_found = self
      .stream
      .clone_dtoh(&self.found_buffer)
      .map_err(|e| format!("Failed to copy found_buffer back to host: {:?}", e))?;

    let found_rel = host_found[0];

    if found_rel == u64::MAX {
      Ok(None)
    } else {
      let found_i = start_i.wrapping_add_u128(found_rel as u128);

      match resolve_key_from_index(solving_mode, found_i, range_start, n_bits, n_mask, &pcg) {
        Some(actual_key) => Ok(Some(actual_key)),
        None => Err("GPU returned index that could not be resolved to a key".into()),
      }
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct PcgConsts {
  pub a: U256,
  pub b: U256,
  pub mix_lo: u64,
  pub mix_hi: u64,
}

impl PcgConsts {
  pub fn derive(
    fingerprint: &[u8; 32],
    puzzle: u32,
  ) -> Self {
    let mut buf1 = [0u8; 32 + 6 + 4];
    buf1[..32].copy_from_slice(fingerprint);
    buf1[32..38].copy_from_slice(b"PCG-v1");
    buf1[38..42].copy_from_slice(&puzzle.to_le_bytes());
    let h1 = Sha256::digest(buf1);

    let mut buf2 = [0u8; 32 + 8 + 4];
    buf2[..32].copy_from_slice(fingerprint);
    buf2[32..40].copy_from_slice(b"PCG-v1-b");
    buf2[40..44].copy_from_slice(&puzzle.to_le_bytes());
    let h2 = Sha256::digest(buf2);

    let mut a = U256 {
      hi: u128::from_be_bytes(h1[0..16].try_into().unwrap()),
      lo: u128::from_be_bytes(h1[16..32].try_into().unwrap()),
    };
    a.lo |= 1;

    let b = U256 {
      hi: u128::from_be_bytes(h2[0..16].try_into().unwrap()),
      lo: u128::from_be_bytes(h2[16..32].try_into().unwrap()),
    };

    let mix_lo = u64::from_le_bytes(h2[0..8].try_into().unwrap()) | 1;
    let mix_hi = u64::from_le_bytes(h2[8..16].try_into().unwrap()) | 1;

    Self { a, b, mix_lo, mix_hi }
  }

  pub fn for_local_machine(puzzle: u32) -> Self {
    Self::derive(&kernel::compute_machine_fingerprint(), puzzle)
  }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AllCycleState {
  pub index: usize,
  pub sweep: u64,
}

#[derive(Clone, Debug, Default)]
pub struct GpuInventory {
  pub devices: Vec<GpuLaunchConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KernelType {
  ChudnovskyAddG,
  ChudnovskyMeloniCoZ,

  Jacobian,
  JacobianCoZMeloni,
  JacobianLinearGlv,
  JacobianEndoGlv,
  JacobianKaratsuba,

  ProjectiveAddGLazy,
  ProjectiveBaby,
  ProjectiveRestrictedGlv,
  ProjectiveLatticeGlv,
  ProjectiveXOnlyRegG,
  ProjectiveXZBatch,
  ProjectiveZFactorBatch,

  Xyzz,

  XzDifferential,

  Test,
  Demo,
}

impl KernelType {
  pub const ALL: &'static [KernelType] = &[
    KernelType::ChudnovskyAddG,
    KernelType::ChudnovskyMeloniCoZ,
    KernelType::Jacobian,
    KernelType::JacobianCoZMeloni,
    KernelType::JacobianLinearGlv,
    KernelType::JacobianEndoGlv,
    KernelType::JacobianKaratsuba,
    KernelType::ProjectiveAddGLazy,
    KernelType::ProjectiveRestrictedGlv,
    KernelType::ProjectiveLatticeGlv,
    KernelType::ProjectiveXOnlyRegG,
    KernelType::ProjectiveXZBatch,
    KernelType::ProjectiveZFactorBatch,
    KernelType::ProjectiveBaby,
    KernelType::Xyzz,
    KernelType::XzDifferential,
    KernelType::Test,
    KernelType::Demo,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::ChudnovskyAddG => "chudnovsky_addg",
      Self::ChudnovskyMeloniCoZ => "chudnovsky_meloni_coz",

      Self::Jacobian => "jacobian",
      Self::JacobianCoZMeloni => "jacobian_coz_meloni",
      Self::JacobianLinearGlv => "jacobian_linear_glv",
      Self::JacobianEndoGlv => "jacobian_endo_glv",
      Self::JacobianKaratsuba => "jacobian_karatsuba",

      Self::ProjectiveAddGLazy => "projective_addg_lazy",
      Self::ProjectiveBaby => "projective_baby",
      Self::ProjectiveRestrictedGlv => "projective_restricted_glv",
      Self::ProjectiveLatticeGlv => "projective_lattice_glv",
      Self::ProjectiveXOnlyRegG => "projective_x_only_regg",
      Self::ProjectiveXZBatch => "projective_xz_batch",
      Self::ProjectiveZFactorBatch => "projective_zfactor_batch",

      Self::Xyzz => "xyzz",

      Self::XzDifferential => "xz_differential",

      Self::Test => "test",
      Self::Demo => "demo",
    }
  }

  pub fn kernel_name(self) -> &'static str {
    match self {
      Self::ChudnovskyAddG => "kernel-chudnovsky_addg",
      Self::ChudnovskyMeloniCoZ => "kernel-chudnovsky_meloni_coz",

      Self::Jacobian => "kernel-jacobian",
      Self::JacobianCoZMeloni => "kernel-jacobian_coz_meloni",
      Self::JacobianLinearGlv => "kernel-jacobian-linear_glv",
      Self::JacobianEndoGlv => "kernel-jacobian_endo_glv",
      Self::JacobianKaratsuba => "kernel-jacobian_karatsuba",

      Self::ProjectiveAddGLazy => "kernel-projective_addg_lazy",
      Self::ProjectiveRestrictedGlv => "kernel-projective_restricted_glv",
      Self::ProjectiveLatticeGlv => "kernel-projective_lattice_glv",
      Self::ProjectiveXOnlyRegG => "kernel-projective_x_only_regg",
      Self::ProjectiveXZBatch => "kernel-projective_xz_batch",
      Self::ProjectiveBaby => "kernel-projective_baby",
      Self::ProjectiveZFactorBatch => "kernel-projective_zfactor_batch",

      Self::Xyzz => "kernel-xyzz",

      Self::XzDifferential => "kernel-xz_differential",

      Self::Test => "kernel-test",
      Self::Demo => "kernel-demo",
    }
  }

  pub fn display_name(self) -> &'static str {
    match self {
      Self::ChudnovskyAddG => "Chudnovsky + AddG",
      Self::ChudnovskyMeloniCoZ => "Chudnovsky + Meloni + CoZ",

      Self::Jacobian => "Jacobian",
      Self::JacobianCoZMeloni => "Jacobian + CoZ + Meloni",
      Self::JacobianLinearGlv => "Jacobian + Linear + GLV",
      Self::JacobianEndoGlv => "Jacobian + Endo + GLV",
      Self::JacobianKaratsuba => "Jacobian + Karatsuba",

      Self::ProjectiveAddGLazy => "Projective + AddG + Lazy",
      Self::ProjectiveRestrictedGlv => "Projective + Restricted GLV",
      Self::ProjectiveLatticeGlv => "Projective + Lattice GLV",
      Self::ProjectiveXOnlyRegG => "Projective + X-Only + RegG",
      Self::ProjectiveXZBatch => "Projective + XZ-batch",
      Self::ProjectiveZFactorBatch => "Projective + Z-factor batch",
      Self::ProjectiveBaby => "Projective + small BSGS",

      Self::Xyzz => "XYZZ",

      Self::XzDifferential => "XZ differential",

      Self::Test => "Test",
      Self::Demo => "Demo",
    }
  }

  pub fn from_string(s: &str) -> Option<Self> {
    match s.to_ascii_lowercase().as_str() {
      "chudnovsky_addg" | "kernel-chudnovsky_addg" => Some(Self::ChudnovskyAddG),
      "chudnovsky_meloni_coz" | "kernel-chudnovsky_meloni_coz" => Some(Self::ChudnovskyMeloniCoZ),

      "jacobian" | "kernel-jacobian" => Some(Self::Jacobian),
      "jacobian_coz_meloni" | "kernel-jacobian_coz_meloni" => Some(Self::JacobianCoZMeloni),
      "jacobian_linear_glv" | "kernel-jacobian_linear_glv" => Some(Self::JacobianLinearGlv),
      "jacobian_endo_glv" | "kernel-jacobian_endo_glv" => Some(Self::JacobianEndoGlv),
      "jacobian_karatsuba" | "kernel-jacobian_karatsuba" => Some(Self::JacobianKaratsuba),

      "projective_addg_lazy" | "kernel-projective_addg_lazy" => Some(Self::ProjectiveAddGLazy),
      "projective_baby" | "kernel-projective_baby" => Some(Self::ProjectiveBaby),
      "projective_restricted_glv" | "kernel-projective_restricted_glv" => Some(Self::ProjectiveRestrictedGlv),
      "projective_lattice_glv" | "kernel-projective_lattice_glv" => Some(Self::ProjectiveLatticeGlv),
      "projective_x_only_regg" | "kernel-projective_x_only_regg" => Some(Self::ProjectiveXOnlyRegG),
      "projective_xz_batch" | "kernel-projective_xz_batch" => Some(Self::ProjectiveXZBatch),
      "projective_zfactor_batch" | "kernel-projective_zfactor_batch" => Some(Self::ProjectiveZFactorBatch),

      "xyzz" | "kernel-xyzz" => Some(Self::Xyzz),

      "xz_differential" | "kernel-xz_differential" => Some(Self::XzDifferential),

      "test" | "kernel-test" => Some(Self::Test),
      "demo" | "kernel-demo" => Some(Self::Demo),

      _ => None,
    }
  }

  pub fn all() -> &'static [KernelType] {
    Self::ALL
  }
}

#[derive(Clone, Debug, Default)]
pub struct KernelRegistry {
  sizes: std::collections::HashMap<KernelType, Vec<String>>,
  pub images: std::collections::HashMap<String, Vec<u8>>,
  pub machine_fingerprint: [u8; 32],
  pub is_demo: bool,
}

impl KernelRegistry {
  pub fn empty() -> Self {
    Self::default()
  }

  pub fn from_package(
    encrypted_kernels: std::collections::BTreeMap<String, Vec<u8>>,
    machine_fingerprint: [u8; 32],
  ) -> Self {
    let mut sizes: std::collections::HashMap<KernelType, Vec<String>> = std::collections::HashMap::new();
    let mut images: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

    for (name, bytes) in encrypted_kernels {
      Self::ingest_name(&name, &mut sizes);
      images.insert(name, bytes);
    }

    for v in sizes.values_mut() {
      v.sort_by_key(|s| s.parse::<u32>().unwrap_or(0));
      v.dedup();
    }

    let is_demo = !images.is_empty()
      && images.keys().all(|k| {
        let name = std::path::Path::new(k).file_name().and_then(|n| n.to_str()).unwrap_or(k);
        name.starts_with("kernel-demo-") && name.ends_with(".cubin")
      });

    if is_demo {
      let mut demo_sizes: Vec<String> = sizes.get(&KernelType::Demo).cloned().unwrap_or_default();
      if demo_sizes.is_empty() {
        for key in images.keys() {
          if let Some(stem) = key.strip_suffix(".cubin") {
            let mut parts: Vec<&str> = stem.split('-').collect();
            if parts
              .last()
              .map(|p| p.starts_with("sm_") && p[3..].chars().all(|c| c.is_ascii_digit()))
              .unwrap_or(false)
            {
              parts.pop();
            }
            if let Some(sz) = parts.last()
              && sz.parse::<u32>().is_ok()
            {
              demo_sizes.push((*sz).to_string());
            }
          }
        }
        demo_sizes.sort_by_key(|s| s.parse::<u32>().unwrap_or(0));
        demo_sizes.dedup();
      }

      for &fallback in &["128", "512", "1024", "1664", "2048", "4096"] {
        if !demo_sizes.iter().any(|s| s == fallback) {
          demo_sizes.push(fallback.to_string());
        }
      }

      demo_sizes.sort_by_key(|s| s.parse::<u32>().unwrap_or(0));
      demo_sizes.dedup();
      sizes.clear();

      for &ty in KernelType::ALL {
        if matches!(ty, KernelType::Test | KernelType::Demo) {
          continue;
        }

        sizes.insert(ty, demo_sizes.clone());
      }
    }

    Self {
      sizes,
      images,
      machine_fingerprint,
      is_demo,
    }
  }

  pub fn encrypted_blob(
    &self,
    name: &str,
  ) -> Option<&[u8]> {
    self.images.get(name).map(|v| v.as_slice())
  }

  fn ingest_name(
    name: &str,
    sizes: &mut std::collections::HashMap<KernelType, Vec<String>>,
  ) {
    let name = std::path::Path::new(name).file_name().and_then(|n| n.to_str()).unwrap_or(name);

    let Some(stem) = name.strip_suffix(".cubin") else {
      return;
    };

    let mut parts: Vec<&str> = stem.split('-').collect();
    if parts.len() < 3 || parts[0] != "kernel" {
      return;
    }

    if parts
      .last()
      .map(|p| p.starts_with("sm_") && p[3..].chars().all(|c| c.is_ascii_digit()))
      .unwrap_or(false)
    {
      parts.pop();
    }

    if parts.len() < 3 {
      return;
    }

    let size_str = parts[parts.len() - 1];

    if size_str.parse::<u32>().is_err() {
      return;
    }

    let type_str = parts[1..parts.len() - 1].join("_");
    let type_alt = parts[1..parts.len() - 1].join("-");
    let ty = KernelType::from_string(&type_str)
      .or_else(|| KernelType::from_string(&type_alt))
      .or_else(|| KernelType::from_string(&format!("kernel-{}", type_str)))
      .or_else(|| KernelType::from_string(&format!("kernel-{}", type_alt)));

    let Some(ty) = ty else {
      return;
    };

    sizes.entry(ty).or_default().push(size_str.to_string());
  }

  pub fn sizes_for(
    &self,
    ty: KernelType,
  ) -> &[String] {
    self.sizes.get(&ty).map(|v| v.as_slice()).unwrap_or(&[])
  }

  pub fn has(
    &self,
    ty: KernelType,
    size: &str,
  ) -> bool {
    self.sizes_for(ty).iter().any(|s| s == size)
  }

  pub fn default_size_for(
    &self,
    ty: KernelType,
  ) -> Option<&str> {
    let sizes = self.sizes_for(ty);

    if sizes.is_empty() {
      return None;
    }

    if sizes.iter().any(|s| s == "1024") {
      Some("1024")
    } else {
      Some(sizes[0].as_str())
    }
  }

  pub fn first_available_type(&self) -> Option<KernelType> {
    for ty in KernelType::all() {
      if !self.sizes_for(*ty).is_empty() {
        return Some(*ty);
      }
    }
    None
  }
}

#[derive(Clone, Debug)]
pub struct KernelInfo {
  pub kernel_type: KernelType,
  pub kernel_size: String,
  pub image_name: String,
  pub cubin_bytes: Vec<u8>,
}

impl Default for KernelInfo {
  fn default() -> Self {
    Self {
      kernel_type: KernelType::Jacobian,
      kernel_size: "1024".to_string(),
      image_name: "kernel-jacobian-1024.cubin".to_string(),
      cubin_bytes: Vec::new(),
    }
  }
}

impl Drop for KernelInfo {
  fn drop(&mut self) {
    self.cubin_bytes.zeroize();
  }
}

impl KernelInfo {
  pub fn image_bytes(&self) -> usize {
    self.cubin_bytes.len()
  }

  pub fn to_module_image(&self) -> Ptx {
    Ptx::from_binary(self.cubin_bytes.clone())
  }
}

#[derive(Clone, Debug)]
pub struct GpuLaunchConfig {
  pub device_id: u32,
  pub name: String,
  pub compute_cap: String,
  pub sm_count: u32,
  pub clock_mhz: u32,
  pub max_threads_per_block: u32,
  pub warp_size: u32,
  pub max_grid_dim_x: u32,

  pub block_size: u32,
  pub grid_size: u32,
  pub parallel_keys: u32,
  pub chunk_size: u128,

  pub kernel_type: KernelType,
  pub kernel_size: String,
}

#[derive(Clone, Debug)]
pub struct BenchmarkResult {
  pub block_size: u32,
  pub grid_size: u32,
  pub parallel_keys: u32,
  pub chunk_multiplier: u32,
  pub speed_mkeys: f64,
  pub kernel_type: KernelType,
  pub kernel_size: String,
}

#[derive(Clone, Debug)]
pub struct CpuInfo {
  pub model: String,
  pub cores: usize,
  pub threads: usize,
  pub freq_mhz: f64,
}

// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..

pub fn active_puzzles() -> &'static [PuzzleInfo] {
  #[cfg(feature = "test_vectors")]
  {
    crate::test_vector::test_puzzle_infos()
  }

  #[cfg(not(feature = "test_vectors"))]
  {
    UNSOLVED_PUZZLES
  }
}

pub fn puzzle_by_number(n: u32) -> Option<&'static PuzzleInfo> {
  active_puzzles().iter().find(|p| p.number == n)
}

pub fn default_puzzle() -> &'static PuzzleInfo {
  puzzle_by_number(DEFAULT_PUZZLE).expect("default puzzle 72 is in the table")
}

pub fn request_bench_cancel() {
  BENCH_CANCEL.store(true, Ordering::Relaxed);
}

pub fn clear_bench_cancel() {
  BENCH_CANCEL.store(false, Ordering::Relaxed);
}

pub fn bench_cancel_requested() -> bool {
  BENCH_CANCEL.load(Ordering::Relaxed)
}

pub fn report_gpu_error(msg: impl Into<String>) {
  let m = msg.into();
  crate::app_log!("{}", m);
  if let Ok(mut slot) = LAST_GPU_ERROR.lock() {
    *slot = m;
  }
}

pub fn take_gpu_error(clear: bool) -> Option<String> {
  let Ok(mut slot) = LAST_GPU_ERROR.lock() else {
    return None;
  };

  if slot.is_empty() {
    return None;
  }

  let out = slot.clone();
  if clear {
    slot.clear();
  }

  Some(out)
}

pub fn clear_gpu_error() {
  if let Ok(mut slot) = LAST_GPU_ERROR.lock() {
    slot.clear();
  }
}

fn format_cuda_oom_or(
  action: &str,
  device_hint: &str,
  err: impl std::fmt::Debug,
) -> Box<dyn std::error::Error> {
  let s = format!("{:?}", err);
  let lower = s.to_lowercase();
  if lower.contains("out of memory") || lower.contains("cuda_error_out_of_memory") {
    format!(
      "GPU out of memory while trying to {} on {}. \
The selected kernel size / keys-per-thread is too large for this GPU's VRAM or local memory. \
Try a smaller size or lower parallel keys. \
Driver detail: {}",
      action, device_hint, s
    )
    .into()
  } else {
    format!("CUDA error while trying to {} on {}: {:?}", action, device_hint, err).into()
  }
}

fn pack_pubkey_x_limbs(compressed: &[u8; 33]) -> (u32, [u32; 8]) {
  let prefix = compressed[0] as u32;
  let mut limbs = [0u32; 8];

  for (dst, chunk) in limbs.iter_mut().zip(compressed[1..].as_chunks::<4>().0.iter()) {
    *dst = u32::from_be_bytes(*chunk);
  }

  (prefix, limbs)
}

fn u256_limbs(v: U256) -> (u64, u64, u64, u64) {
  (v.lo as u64, (v.lo >> 64) as u64, v.hi as u64, (v.hi >> 64) as u64)
}

#[inline(always)]
fn umul128(
  a: u128,
  b: u128,
) -> (u128, u128) {
  let a0 = a as u64;
  let a1 = (a >> 64) as u64;
  let b0 = b as u64;
  let b1 = (b >> 64) as u64;

  let p00 = (a0 as u128) * (b0 as u128);
  let p01 = (a0 as u128) * (b1 as u128);
  let p10 = (a1 as u128) * (b0 as u128);
  let p11 = (a1 as u128) * (b1 as u128);

  let mid = (p00 >> 64) + (p01 & 0xffff_ffff_ffff_ffff) + (p10 & 0xffff_ffff_ffff_ffff);
  let lo = (p00 & 0xffff_ffff_ffff_ffff) | (mid << 64);
  let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
  (lo, hi)
}

#[inline(always)]
fn permute_index_u256(
  i: U256,
  n_mask: U256,
  c: &PcgConsts,
) -> U256 {
  let mut y = c.a.wrapping_mul(i).wrapping_add(c.b);

  let mut lo = y.lo as u64;
  let mut hi = (y.lo >> 64) as u64;
  let mut hi2 = y.hi as u64;
  let mut hi3 = (y.hi >> 64) as u64;

  lo ^= hi;
  lo = lo.wrapping_mul(c.mix_lo);
  hi ^= lo >> 17;
  hi = hi.wrapping_mul(c.mix_hi);
  hi2 ^= hi;
  hi2 = hi2.wrapping_mul(c.mix_lo.rotate_left(7));
  hi3 ^= hi2 >> 13;
  hi3 = hi3.wrapping_mul(c.mix_hi.rotate_left(11));

  lo ^= hi3 >> 16;
  hi ^= hi2 >> 16;

  y.lo = ((hi as u128) << 64) | (lo as u128);
  y.hi = ((hi3 as u128) << 64) | (hi2 as u128);

  y.lo ^= y.hi >> 29;
  y.hi ^= y.lo >> 31;
  y.lo ^= y.lo >> 17;

  y.bit_and(n_mask)
}

fn save_alloc_state(
  puzzle: u32,
  solving_mode: SearchMode,
  alloc_arc: &Arc<Mutex<AllocState>>,
) {
  let a = alloc_arc.lock().unwrap();
  let path = get_global_next_path(puzzle, solving_mode);

  if let Err(e) = std::fs::write(&path, a.next_i.to_hex()) {
    crate::app_log!("Failed to save GLOBAL_NEXT ({}): {}", path, e);
  }
}

pub fn persist_alloc_progress(
  puzzle: u32,
  solving_mode: SearchMode,
  alloc_arc: &Arc<Mutex<AllocState>>,
) {
  setup_status_dir(puzzle, solving_mode);

  let a = alloc_arc.lock().unwrap();

  let next_path = get_global_next_path(puzzle, solving_mode);
  if let Err(e) = std::fs::write(&next_path, a.next_i.to_hex()) {
    crate::app_log!("Failed to save GLOBAL_NEXT ({}): {}", next_path, e);
  }
}

fn load_next_i(
  puzzle: u32,
  solving_mode: SearchMode,
) -> U256 {
  let path = get_global_next_path(puzzle, solving_mode);

  if let Ok(s) = std::fs::read_to_string(path) {
    let t = s.trim();
    if !t.is_empty() {
      return U256::from_hex(t);
    }
  }
  U256::ZERO
}

fn append_log_assigned(
  puzzle: u32,
  solving_mode: SearchMode,
  start_i: U256,
  len: u128,
) {
  append_log_line(puzzle, solving_mode, format!("ASSIGNED {} {:X}\n", start_i.to_hex(), len));
}

fn append_log_finished(
  puzzle: u32,
  solving_mode: SearchMode,
  start_i: U256,
) {
  append_log_line(puzzle, solving_mode, format!("FINISHED {}\n", start_i.to_hex()));
}

fn append_log_line(
  puzzle: u32,
  solving_mode: SearchMode,
  line: String,
) {
  let path = get_alloc_log_path(puzzle, solving_mode);

  if let Err(e) = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .and_then(|mut f| f.write_all(line.as_bytes()))
  {
    crate::app_log!("Failed to append to {}: {}", path, e);
  }
}

fn setup_status_dir(
  puzzle: u32,
  solving_mode: SearchMode,
) {
  let dir = format!("{}/{}/{}", STATUS_DIR, puzzle, mode_dir(solving_mode));

  if Path::new(&dir).exists() && fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0) > 0 {
    return;
  }

  fs::create_dir_all(&dir).expect("Failed to create status directory");
  File::create(get_alloc_log_path(puzzle, solving_mode)).expect("Failed to create new ALLOC_LOG_FILE");
  File::create(get_global_next_path(puzzle, solving_mode)).expect("Failed to create new GLOBAL_NEXT_FILE");
}

fn backup_last_session() {
  let has_content = Path::new(STATUS_DIR).exists() && fs::read_dir(STATUS_DIR).map(|mut d| d.next().is_some()).unwrap_or(false);
  if !has_content {
    return;
  }

  if !Path::new(BACKUPS_DIR).exists() {
    fs::create_dir(BACKUPS_DIR).expect("Failed to create backups directory");
  }

  let now = Local::now();
  let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
  let backup_path = format!("{}/{}", BACKUPS_DIR, timestamp);

  crate::app_log!("Creating backup of previous session: {}", backup_path);

  if let Err(e) = copy_dir_all(STATUS_DIR, &backup_path) {
    crate::app_log!("Backup failed: {}", e);
    return;
  }

  crate::app_log!("Backup completed: {}", backup_path);
}

fn copy_dir_all(
  src: impl AsRef<Path>,
  dst: &str,
) -> io::Result<()> {
  fs::create_dir_all(dst)?;

  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let path = entry.path();
    let target = format!("{}/{}", dst, path.file_name().unwrap().to_string_lossy());

    if path.is_dir() {
      copy_dir_all(&path, &target)?;
    } else {
      fs::copy(&path, target)?;
    }
  }

  Ok(())
}

fn load_assignment_log_from_path(path: &str) -> (VecDeque<Chunk>, U256) {
  let mut assigned: std::collections::HashMap<String, u128> = std::collections::HashMap::new();
  let mut finished: std::collections::HashSet<String> = std::collections::HashSet::new();

  let mut farthest_end = U256::ZERO;
  let mut pending = VecDeque::new();

  if let Ok(file) = File::open(path) {
    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
      let parts: Vec<&str> = line.split_whitespace().collect();

      if parts.is_empty() {
        continue;
      }

      match parts[0] {
        "ASSIGNED" if parts.len() == 3 => {
          let s = U256::from_hex(parts[1]);
          if let Ok(l) = u128::from_str_radix(parts[2], 16) {
            assigned.insert(s.to_hex(), l);
            farthest_end = farthest_end.max(s.wrapping_add_u128(l));
          }
        }

        "FINISHED" if parts.len() == 2 => {
          let s = U256::from_hex(parts[1]);
          finished.insert(s.to_hex());
        }

        _ => {}
      }
    }
  }

  for (s_hex, l) in assigned {
    if !finished.contains(&s_hex) {
      pending.push_back(Chunk {
        start_i: U256::from_hex(&s_hex),
        len: l,
      });
    }
  }

  (pending, farthest_end)
}

fn recover_allocator(
  puzzle: u32,
  solving_mode: SearchMode,
  total_keys: U256,
) -> AllocState {
  setup_status_dir(puzzle, solving_mode);

  let log_path = get_alloc_log_path(puzzle, solving_mode);
  let alloc_log_path = if Path::new(&log_path).exists() {
    log_path.clone()
  } else {
    String::new()
  };

  let (pending, farthest_end_from_log) = if !alloc_log_path.is_empty() {
    load_assignment_log_from_path(&alloc_log_path)
  } else {
    (VecDeque::new(), U256::ZERO)
  };

  let loaded = load_next_i(puzzle, solving_mode);
  let resume = farthest_end_from_log.max(loaded);

  let next_i = match solving_mode {
    SearchMode::Sequence => {
      if resume.is_zero() {
        U256::ONE
      } else {
        resume
      }
    }
    SearchMode::Pcg => resume,
  }
  .min(total_keys);

  if !Path::new(&log_path).exists() {
    File::create(&log_path).expect("Failed to create ALLOC.log");
  }

  append_log_line(
    puzzle,
    solving_mode,
    format!(
      "# RESUME next_i={} pending={} global_next={} farthest_log={}\n",
      next_i.to_hex(),
      pending.len(),
      loaded.to_hex(),
      farthest_end_from_log.to_hex()
    ),
  );

  AllocState { next_i, pending }
}

fn spawn_allocator_thread(
  puzzle: u32,
  solving_mode: SearchMode,
  alloc: Arc<Mutex<AllocState>>,
  rx: Receiver<AllocCommand>,
  total_keys: U256,
  shutdown: Arc<AtomicBool>,
  ready_tx: Sender<()>,
) -> JoinHandle<()> {
  thread::spawn(move || {
    let mut last_save = Instant::now();

    let mut finished_chunks: BTreeMap<U256, u128> = BTreeMap::new();

    {
      let _state = alloc.lock().unwrap();
    }
    let _ = ready_tx.send(());

    while let Ok(cmd) = rx.recv() {
      match cmd {
        AllocCommand::RequestChunk { chunk_size, reply } => {
          let mut state = alloc.lock().unwrap();

          if let Some(chunk) = state.pending.pop_front() {
            let _ = reply.send(Some(chunk));
            continue;
          }

          if state.next_i >= total_keys {
            let _ = reply.send(None);
            continue;
          }

          let start_i = state.next_i;

          let remaining = total_keys.saturating_sub(start_i);
          let len = if remaining.hi != 0 { chunk_size } else { chunk_size.min(remaining.lo) };
          state.next_i = start_i.wrapping_add_u128(len);

          append_log_assigned(puzzle, solving_mode, start_i, len);

          let now = Instant::now();
          if now.duration_since(last_save).as_secs() >= PROGRESS_SAVE_INTERVAL_SEC {
            save_alloc_state(puzzle, solving_mode, &alloc);
            last_save = now;
          }

          let _ = reply.send(Some(Chunk { start_i, len }));
        }

        AllocCommand::ReportFinished { start_i, len } => {
          append_log_finished(puzzle, solving_mode, start_i);

          let end_i = start_i.wrapping_add_u128(len);

          if let Some((&prev_start, &prev_len)) = finished_chunks.range(..start_i).next_back() {
            let prev_end = prev_start.wrapping_add_u128(prev_len);
            if start_i < prev_end {
              crate::app_log!(
                "[WARN] Finished chunk overlap: [{}..{}) overlaps with previous [{}..{})",
                start_i.to_hex(),
                end_i.to_hex(),
                prev_start.to_hex(),
                prev_end.to_hex()
              );
            }
          }

          let after = start_i.wrapping_add_u128(1);
          if let Some((&next_start, &next_len)) = finished_chunks.range(after..).next()
            && end_i > next_start
          {
            let next_end = next_start.wrapping_add_u128(next_len);
            crate::app_log!(
              "[WARN] Finished chunk overlap: [{}..{}) overlaps with next [{}..{})",
              start_i.to_hex(),
              end_i.to_hex(),
              next_start.to_hex(),
              next_end.to_hex()
            );
          }

          finished_chunks.insert(start_i, len);

          let now = Instant::now();
          if now.duration_since(last_save).as_secs() >= PROGRESS_SAVE_INTERVAL_SEC {
            save_alloc_state(puzzle, solving_mode, &alloc);
            last_save = now;
          }
        }

        AllocCommand::SaveNow => {
          persist_alloc_progress(puzzle, solving_mode, &alloc);
          last_save = Instant::now();
        }

        AllocCommand::Stop => {
          persist_alloc_progress(puzzle, solving_mode, &alloc);

          let state = alloc.lock().unwrap();
          let completed_full_range = !FOUND.load(Ordering::Relaxed) && state.next_i >= total_keys && state.pending.is_empty();

          if completed_full_range && !finished_chunks.is_empty() {
            let mut iter = finished_chunks.iter();
            if let Some((&first_start, &first_len)) = iter.next() {
              let current_start = first_start;
              let mut current_end = first_start.wrapping_add_u128(first_len);

              if current_start > U256::ZERO {
                crate::app_log!("[WARN] Gap at beginning: finished starts at {}, expected 0", current_start.to_hex());
              }

              for (&s, &l) in iter {
                if s > current_end {
                  crate::app_log!("[WARN] Gap detected: previous end {}, next start {}", current_end.to_hex(), s.to_hex());
                } else if s < current_end {
                  crate::app_log!(
                    "[WARN] Overlap detected while merging: [{}..{}) with [{}..{})",
                    current_start.to_hex(),
                    current_end.to_hex(),
                    s.to_hex(),
                    s.wrapping_add_u128(l).to_hex()
                  );
                }

                let new_end = s.wrapping_add_u128(l);
                if new_end > current_end {
                  current_end = new_end;
                }
              }

              if current_end < total_keys {
                crate::app_log!(
                  "[WARN] Gap at end: coverage ends at {}, expected {}",
                  current_end.to_hex(),
                  total_keys.to_hex()
                );
              }
            }
          }

          shutdown.store(true, Ordering::Relaxed);
          break;
        }
      }
    }
  })
}

fn install_panic_hook(
  puzzle: u32,
  solving_mode: SearchMode,
  alloc: Arc<Mutex<AllocState>>,
) {
  std::panic::set_hook(Box::new(move |info| {
    if PANIC_OCCURRED.swap(true, Ordering::Relaxed) {
      return;
    }

    let panic_line = format!("PANIC: {}", info);
    eprintln!("{}", panic_line);

    crate::gui::enqueue_log(panic_line);

    let _ = std::panic::catch_unwind(|| {
      save_alloc_state(puzzle, solving_mode, &alloc);
      backup_last_session();
      let done = "Progress saved and backup done.".to_string();
      eprintln!("{}", done);
      crate::gui::enqueue_log(done);
    });
  }));
}

#[allow(clippy::too_many_arguments)]
fn spawn_cpu_workers(
  puzzle: u32,
  address: &'static str,
  target_h160: [u8; 20],
  target_pubkey: Option<[u8; 33]>,
  range_start: U256,
  n_bits: u32,
  n_mask: U256,
  solving_mode: SearchMode,
  pcg: PcgConsts,
  cmd_tx: Sender<AllocCommand>,
  shutdown: Arc<AtomicBool>,
  threads: usize,
  intensity: Arc<AtomicU32>,
) -> JoinHandle<()> {
  thread::spawn(move || {
    (0..threads).into_par_iter().for_each(|t| {
      let thread_id = t;
      let mut checked: u128 = 0;
      let mut last_stat = Instant::now();
      let mut last_save_request = Instant::now();

      loop {
        if shutdown.load(Ordering::Relaxed) || FOUND.load(Ordering::Relaxed) {
          break;
        }
        if all_mode_turn_exhausted() {
          break;
        }

        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = cmd_tx.send(AllocCommand::RequestChunk {
          chunk_size: CPU_CHUNK_SIZE,
          reply: reply_tx,
        });

        let chunk = match reply_rx.recv() {
          Ok(Some(c)) => c,
          _ => break,
        };

        let mut offset: u128 = 0;

        while offset < chunk.len {
          if shutdown.load(Ordering::Relaxed) || FOUND.load(Ordering::Relaxed) {
            break;
          }

          let batch_len_u128 = (chunk.len - offset).min(CPU_PARALLEL_KEYS as u128);
          let batch_len = batch_len_u128 as usize;

          let batch_start = Instant::now();

          let mut key_bytes_batch = [[0u8; 32]; CPU_PARALLEL_KEYS];
          let mut k_batch = [U256::ZERO; CPU_PARALLEL_KEYS];

          for p in 0..batch_len {
            let current_i = chunk.start_i.wrapping_add_u128(offset + p as u128);
            let k = match resolve_key_from_index(solving_mode, current_i, range_start, n_bits, n_mask, &pcg) {
              Some(k) => k,
              None => continue,
            };
            k_batch[p] = k;
            key_bytes_batch[p] = k.to_be_bytes();
          }

          for p in 0..batch_len {
            let k = k_batch[p];
            let key_bytes = key_bytes_batch[p];

            LOCAL_SECP.with(|_| {
              if let Ok(sk) = secp256k1::SecretKey::from_secret_bytes(key_bytes) {
                let pk = secp256k1::PublicKey::from_secret_key(&sk);
                let compressed: [u8; 33] = pk.serialize();

                let hit = if let Some(ref target_pk) = target_pubkey {
                  compressed == *target_pk
                } else {
                  let h160 = Ripemd160::digest(Sha256::digest(compressed));
                  h160.as_slice() == target_h160.as_slice()
                };

                if hit {
                  save_found_key(puzzle, address, k);
                  FOUND.store(true, Ordering::Relaxed);
                  shutdown.store(true, Ordering::Relaxed);
                }
              }
            });
          }

          let batch_elapsed = batch_start.elapsed();

          checked += batch_len as u128;
          add_to_total_checked(batch_len as u128);

          let now = Instant::now();

          if now.duration_since(last_stat).as_millis() >= UI_REFRESH_INTERVAL_MS as u128 {
            let mut dash = DASHBOARD.lock().unwrap();
            if thread_id < dash.len() {
              let status = &mut dash[thread_id];
              let elapsed = now.duration_since(status.last_update).as_secs_f64();
              if elapsed >= 1.0 {
                status.speed_kps = (checked - status.last_checked) as f64 / elapsed / 1_000.0;
                status.last_update = now;
                status.last_checked = checked;
              }
              status.checked = checked;
              let show_i = chunk.start_i.wrapping_add_u128(offset);
              status.last_i = show_i.lo;
              let mut idx = show_i.to_hex();
              if let Some(k) = resolve_key_from_index(solving_mode, show_i, range_start, n_bits, n_mask, &pcg) {
                status.key_hex = k.to_hex();

                if idx.len() < status.key_hex.len() {
                  idx = format!("{:0>width$}", idx, width = status.key_hex.len());
                }
              }
              status.index_hex = idx;
            }
            last_stat = now;
          }

          if now.duration_since(last_save_request).as_secs() >= PROGRESS_SAVE_INTERVAL_SEC {
            let _ = cmd_tx.send(AllocCommand::SaveNow);
            last_save_request = now;
          }

          let intensity_pct = intensity.load(Ordering::Relaxed).clamp(10, 100) as f64 / 100.0;
          if intensity_pct < 1.0 {
            let desired_cycle = batch_elapsed.as_secs_f64() / intensity_pct;
            let sleep_secs = desired_cycle - batch_elapsed.as_secs_f64();
            if sleep_secs > 0.0002 {
              std::thread::sleep(Duration::from_secs_f64(sleep_secs.min(0.050)));
            }
          }

          offset += batch_len as u128;
        }

        let _ = cmd_tx.send(AllocCommand::ReportFinished {
          start_i: chunk.start_i,
          len: chunk.len,
        });
        if all_mode_note_chunk_finished() {
          let _ = cmd_tx.send(AllocCommand::SaveNow);
          break;
        }
      }
    });
  })
}

#[allow(clippy::too_many_arguments)]
fn spawn_gpu_worker(
  puzzle: u32,
  range_start: U256,
  n_bits: u32,
  n_mask: U256,
  target_h160: [u8; 20],
  target_pubkey: Option<[u8; 33]>,
  pcg: PcgConsts,
  solving_mode: SearchMode,
  cmd_tx: Sender<AllocCommand>,
  shutdown: Arc<AtomicBool>,
  dash_index: usize,
  intensity: Arc<AtomicU32>,
  gpu_cfg: GpuLaunchConfig,
) -> JoinHandle<()> {
  let total_bits = puzzle;
  thread::spawn(move || {
    if target_pubkey.is_some() {
      crate::app_log!("[GPU] Puzzle #{} - matching compressed public key (HASH160 skipped)", puzzle);
    }

    let mut solver = match GpuSolver::new(&gpu_cfg) {
      Ok(s) => s,
      Err(e) => {
        report_gpu_error(format!("GPU init failed on [{}] {}: {}", gpu_cfg.device_id, gpu_cfg.name, e));
        shutdown.store(true, Ordering::Relaxed);
        return;
      }
    };

    crate::app_log!(
      "GPU worker started on [{}] {}  |  SMs={}  block={}  grid={}  keys/thread={}  base_chunk={:#X}  type={}  size={}",
      gpu_cfg.device_id,
      gpu_cfg.name,
      gpu_cfg.sm_count,
      gpu_cfg.block_size,
      gpu_cfg.grid_size,
      gpu_cfg.parallel_keys,
      gpu_cfg.chunk_size,
      gpu_cfg.kernel_type.display_name(),
      gpu_cfg.kernel_size
    );

    let mut last_save_request = Instant::now();

    let effective_kpt = gpu_cfg.parallel_keys.max(1) as u128;
    let launch_capacity = (gpu_cfg.block_size as u128)
      .saturating_mul(gpu_cfg.grid_size as u128)
      .saturating_mul(effective_kpt)
      .max(1);

    while !shutdown.load(Ordering::Relaxed) && !FOUND.load(Ordering::Relaxed) {
      if all_mode_turn_exhausted() {
        break;
      }

      let intensity_pct = intensity.load(Ordering::Relaxed).clamp(10, 100) as f64 / 100.0;

      let effective_chunk_size = ((gpu_cfg.chunk_size as f64) * intensity_pct).max(4096.0).round() as u128;

      let (reply_tx, reply_rx) = mpsc::channel();
      let _ = cmd_tx.send(AllocCommand::RequestChunk {
        chunk_size: effective_chunk_size,
        reply: reply_tx,
      });

      let chunk = match reply_rx.recv() {
        Ok(Some(c)) => c,
        _ => break,
      };

      let mut offset: u128 = 0;
      let mut found_key: Option<U256> = None;
      let mut total_scanned: u128 = 0;
      let gpu_start = Instant::now();

      while offset < chunk.len && found_key.is_none() {
        if shutdown.load(Ordering::Relaxed) || FOUND.load(Ordering::Relaxed) {
          break;
        }

        let this_count = (chunk.len - offset).min(launch_capacity);
        if this_count == 0 {
          break;
        }

        match solver.search_batch(
          solving_mode,
          chunk.start_i.wrapping_add_u128(offset),
          this_count,
          range_start,
          n_bits,
          n_mask,
          total_bits,
          target_h160,
          target_pubkey,
          pcg,
        ) {
          Ok(Some(key)) => {
            found_key = Some(key);
            total_scanned += this_count;
            break;
          }
          Ok(None) => {
            total_scanned += this_count;
            offset += this_count;
          }
          Err(e) => {
            report_gpu_error(format!("GPU error on device {} ({}): {}", gpu_cfg.device_id, gpu_cfg.name, e));

            let _ = cmd_tx.send(AllocCommand::ReportFinished {
              start_i: chunk.start_i,
              len: total_scanned,
            });
            shutdown.store(true, Ordering::Relaxed);
            return;
          }
        }
      }

      let gpu_end = Instant::now();
      let gpu_elapsed = gpu_end.duration_since(gpu_start).as_secs_f64();

      if let Some(key) = found_key {
        let address = puzzle_by_number(puzzle).map(|p| p.address).unwrap_or("");
        save_found_key(puzzle, address, key);
        FOUND.store(true, Ordering::Relaxed);
        shutdown.store(true, Ordering::Relaxed);

        if total_scanned > 0 {
          add_to_total_checked(total_scanned);
        }
        let _ = cmd_tx.send(AllocCommand::ReportFinished {
          start_i: chunk.start_i,
          len: total_scanned,
        });
        break;
      }

      if total_scanned > 0 {
        add_to_total_checked(total_scanned);
      }
      let _ = cmd_tx.send(AllocCommand::ReportFinished {
        start_i: chunk.start_i,
        len: total_scanned,
      });

      if all_mode_note_chunk_finished() {
        let _ = cmd_tx.send(AllocCommand::SaveNow);
        break;
      }

      {
        let mut dash = DASHBOARD.lock().unwrap();
        if dash_index < dash.len() {
          let status = &mut dash[dash_index];

          if gpu_elapsed > 0.0 && total_scanned > 0 {
            status.speed_kps = (total_scanned as f64) / gpu_elapsed / 1_000.0;
          }

          status.checked += total_scanned;
          let last_index = chunk.start_i.wrapping_add_u128(total_scanned.saturating_sub(1));
          status.last_i = last_index.lo;
          status.last_update = gpu_end;
          status.last_checked = status.checked;

          let rs = range_start;
          let nm = n_mask;
          let mut idx = last_index.to_hex();
          if let Some(k) = resolve_key_from_index(solving_mode, last_index, rs, n_bits, nm, &pcg) {
            status.key_hex = k.to_hex();
            if idx.len() < status.key_hex.len() {
              idx = format!("{:0>width$}", idx, width = status.key_hex.len());
            }
          }
          status.index_hex = idx;
        }
      }

      let now = Instant::now();
      if now.duration_since(last_save_request).as_secs() >= PROGRESS_SAVE_INTERVAL_SEC {
        let _ = cmd_tx.send(AllocCommand::SaveNow);
        last_save_request = now;
      }
    }

    if FOUND.load(Ordering::Relaxed) {
      crate::app_log!("\nKEY FOUND BY GPU! Saved to {}", found_file_path(puzzle));
    } else if all_mode_turn_exhausted() {
      crate::app_log!("\n[ALL] GPU device {} finished its batch for puzzle #{}.", gpu_cfg.device_id, puzzle);
    } else {
      crate::app_log!("\nGPU search completed on device {}. No key found.", gpu_cfg.device_id);
    }
  })
}

fn private_key_to_wif(
  private_key: &U256,
  compressed: bool,
) -> String {
  let mut payload = Vec::with_capacity(37);
  payload.push(0x80);
  payload.extend_from_slice(&private_key.to_be_bytes());
  if compressed {
    payload.push(0x01);
  }
  let checksum = {
    let h1 = Sha256::digest(&payload);
    let h2 = Sha256::digest(h1);
    h2[..4].to_vec()
  };
  payload.extend_from_slice(&checksum);
  bs58::encode(payload).into_string()
}

fn save_found_key(
  puzzle: u32,
  address: &str,
  private_key: U256,
) {
  let key_hex = {
    let b = private_key.to_be_bytes();
    let mut s = String::with_capacity(64);
    for byte in b {
      s.push_str(&format!("{:02x}", byte));
    }
    s
  };

  let key_wif = private_key_to_wif(&private_key, true);
  let key_wif_uncompressed = private_key_to_wif(&private_key, false);
  let display = format!(
    "hex: {}\nWIF (compressed): {}\nWIF (uncompressed): {}",
    key_hex, key_wif, key_wif_uncompressed
  );

  if let Ok(mut slot) = FOUND_KEY_HEX.lock() {
    *slot = Some(display.clone());
  }

  let content = format!(
    "PRIVATE KEY FOUND!\n\
     Puzzle: #{}\n\
     Address: {}\n\
     Private key (hex): {}\n\
     Private key WIF (compressed): {}\n\
     Private key WIF (uncompressed): {}\n",
    puzzle, address, key_hex, key_wif, key_wif_uncompressed
  );

  let path = found_file_path(puzzle);

  if let Some(parent) = Path::new(&path).parent() {
    let _ = fs::create_dir_all(parent);
  }

  let _ = fs::write(&path, content);

  crate::app_log!(
    "\n!!! PRIVATE KEY FOUND !!!\nPuzzle: #{}\nAddress: {}\nHex: {}\nWIF (compressed): {}\nWIF (uncompressed): {}",
    puzzle,
    address,
    key_hex,
    key_wif,
    key_wif_uncompressed
  );
}

fn add_to_total_checked(added: u128) {
  let added_low = added as u64;
  let added_high = (added >> 64) as u64;

  let old_low = TOTAL_CHECKED_LOW.fetch_add(added_low, Ordering::Relaxed);
  let mut carry = if old_low > u64::MAX - added_low { 1 } else { 0 };
  carry += added_high;

  if carry > 0 {
    TOTAL_CHECKED_HIGH.fetch_add(carry, Ordering::Relaxed);
  }
}

#[inline(always)]
fn resolve_key_from_index(
  solving_mode: SearchMode,
  i: U256,
  range_start: U256,
  _n_bits: u32,
  n_mask: U256,
  pcg: &PcgConsts,
) -> Option<U256> {
  match solving_mode {
    SearchMode::Sequence => Some(range_start.wrapping_add(i)),

    SearchMode::Pcg => Some(range_start.wrapping_add(permute_index_u256(i, n_mask, pcg))),
  }
}

fn mode_dir(solving_mode: SearchMode) -> String {
  match solving_mode {
    SearchMode::Sequence => String::from("SEQ"),
    SearchMode::Pcg => String::from("PCG"),
  }
}

fn status_path(
  puzzle: u32,
  solving_mode: SearchMode,
  file: &str,
) -> String {
  format!("{}/{}/{}/{}", STATUS_DIR, puzzle, mode_dir(solving_mode), file)
}

fn get_global_next_path(
  puzzle: u32,
  solving_mode: SearchMode,
) -> String {
  status_path(puzzle, solving_mode, "GLOBAL_NEXT")
}

fn get_alloc_log_path(
  puzzle: u32,
  solving_mode: SearchMode,
) -> String {
  status_path(puzzle, solving_mode, "ALLOC.log")
}

fn found_file_path(puzzle: u32) -> String {
  format!("{}/{}/address.txt", STATUS_DIR, puzzle)
}

pub fn is_all_mode(puzzle: u32) -> bool {
  puzzle == ALL_PUZZLES
}

fn all_status_dir() -> String {
  format!("{}/ALL", STATUS_DIR)
}

fn all_cycle_state_path() -> String {
  format!("{}/cycle_state", all_status_dir())
}

pub fn setup_all_status_dir() {
  let dir = all_status_dir();
  if !Path::new(&dir).exists() {
    fs::create_dir_all(&dir).expect("Failed to create status/ALL directory");
  }
}

pub fn load_all_cycle_state() -> AllCycleState {
  setup_all_status_dir();
  let path = all_cycle_state_path();
  let mut state = AllCycleState::default();

  if let Ok(text) = fs::read_to_string(&path) {
    for line in text.lines() {
      if let Some(rest) = line.strip_prefix("index=")
        && let Ok(v) = rest.trim().parse::<usize>()
      {
        state.index = v;
      } else if let Some(rest) = line.strip_prefix("sweep=")
        && let Ok(v) = rest.trim().parse::<u64>()
      {
        state.sweep = v;
      }
    }
  }
  state
}

pub fn save_all_cycle_state(state: &AllCycleState) {
  setup_all_status_dir();
  let path = all_cycle_state_path();
  let body = format!("index={}\nsweep={}\n", state.index, state.sweep);

  if let Err(e) = fs::write(&path, body) {
    crate::app_log!("Failed to save ALL cycle state: {}", e);
  }
}

pub fn remaining_puzzles() -> Vec<&'static PuzzleInfo> {
  active_puzzles()
    .iter()
    .filter(|p| !Path::new(&found_file_path(p.number)).exists())
    .collect()
}

pub fn arm_all_mode_turn(duration_secs: u64) {
  let secs = duration_secs.max(1);
  let now_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0);

  let deadline = now_ms.saturating_add(secs.saturating_mul(1000));

  ALL_MODE_DEADLINE_MS.store(deadline, Ordering::Relaxed);
}

pub fn clear_all_mode_chunk_budget() {
  ALL_MODE_DEADLINE_MS.store(0, Ordering::Relaxed);
}

fn all_mode_note_chunk_finished() -> bool {
  all_mode_turn_exhausted()
}

fn all_mode_turn_exhausted() -> bool {
  let deadline = ALL_MODE_DEADLINE_MS.load(Ordering::Relaxed);

  if deadline == 0 {
    return false;
  }

  let now_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0);

  now_ms >= deadline
}

pub fn set_active_package(reg: Option<KernelRegistry>) {
  *ACTIVE_PACKAGE.lock().unwrap() = reg;
}

pub fn with_active_package<F, R>(f: F) -> R
where
  F: FnOnce(Option<&KernelRegistry>) -> R,
{
  let guard = ACTIVE_PACKAGE.lock().unwrap();
  f(guard.as_ref())
}

fn image_file_name(
  ty: KernelType,
  size: &str,
  arch: Option<&str>,
) -> String {
  match arch {
    Some(a) if !a.is_empty() && a != "native" => {
      format!("kernel-{}-{}-{}.cubin", ty.as_str(), size, a)
    }
    _ => format!("kernel-{}-{}.cubin", ty.as_str(), size),
  }
}

pub fn load_kernel(
  ty: KernelType,
  size: &str,
) -> Result<KernelInfo, Box<dyn Error>> {
  with_active_package(|active| load_kernel_with_registry(ty, size, None, active))
}

pub fn load_kernel_with_registry(
  ty: KernelType,
  size: &str,
  arch: Option<&str>,
  registry: Option<&KernelRegistry>,
) -> Result<KernelInfo, Box<dyn Error>> {
  if let Some(reg) = registry
    && reg.is_demo
  {
    let mut demo_candidates: Vec<String> = Vec::new();

    if let Some(a) = arch
      && !a.is_empty()
      && a != "native"
    {
      demo_candidates.push(format!("kernel-demo-{}-{}.cubin", size, a));
    }
    demo_candidates.push(format!("kernel-demo-{}.cubin", size));

    let size_prefix = format!("kernel-demo-{}-", size);
    for key in reg.images.keys() {
      if key.starts_with(&size_prefix) && key.ends_with(".cubin") && !demo_candidates.contains(key) {
        demo_candidates.push(key.clone());
      }
    }

    for key in reg.images.keys() {
      if key.contains("kernel-demo-") && key.ends_with(".cubin") && !demo_candidates.contains(key) {
        demo_candidates.push(key.clone());
      }
    }

    for name in &demo_candidates {
      if let Some(encrypted) = reg.encrypted_blob(name) {
        let cubin_bytes = crate::kernel::decrypt_kernel_blob(&reg.machine_fingerprint, encrypted, name).map_err(|e| e.to_string())?;
        return Ok(KernelInfo {
          kernel_type: ty,
          kernel_size: size.to_string(),
          image_name: name.clone(),
          cubin_bytes,
        });
      }
    }

    let available: Vec<_> = reg.images.keys().cloned().collect();
    return Err(
      format!(
        "Demo package has no usable CUBIN (tried {:?}). Available: {:?}",
        demo_candidates, available
      )
      .into(),
    );
  }

  let candidates: Vec<String> = {
    let mut v = Vec::new();

    if let Some(a) = arch
      && !a.is_empty()
      && a != "native"
    {
      v.push(image_file_name(ty, size, Some(a)));
    }

    v.push(image_file_name(ty, size, None));

    if arch.is_none()
      && let Some(reg) = registry
    {
      let prefix = format!("kernel-{}-{}-", ty.as_str(), size);
      for key in reg.images.keys() {
        if let Some(rest) = key.strip_prefix(&prefix)
          && rest.starts_with("sm_")
          && rest.ends_with(".cubin")
          && !v.contains(key)
        {
          v.push(key.clone());
        }
      }
    }

    v
  };

  if let Some(reg) = registry {
    for name in &candidates {
      if let Some(encrypted) = reg.encrypted_blob(name) {
        let cubin_bytes = crate::kernel::decrypt_kernel_blob(&reg.machine_fingerprint, encrypted, name).map_err(|e| e.to_string())?;

        return Ok(KernelInfo {
          kernel_type: ty,
          kernel_size: size.to_string(),
          image_name: name.clone(),
          cubin_bytes,
        });
      }
    }

    if !reg.images.is_empty() {
      let available: Vec<_> = reg.images.keys().cloned().collect();

      return Err(
        format!(
          "Kernel {:?} not found inside the loaded kernels.OWL (need arch-specific CUBIN for multi-GPU). Available: {:?}",
          candidates, available
        )
        .into(),
      );
    }
  }

  Err("Load kernel file first".into())
}

pub fn kernel_info_default(registry: &KernelRegistry) -> Result<KernelInfo, Box<dyn Error>> {
  let ty = registry.first_available_type().ok_or("No kernels in package registry")?;
  let size = registry.default_size_for(ty).ok_or("No sizes for selected kernel type")?;

  load_kernel_with_registry(ty, size, None, Some(registry))
}

#[allow(clippy::too_many_arguments)]
fn bench_one_config(
  base: &GpuLaunchConfig,
  block: u32,
  grid: u32,
  parallel_keys: u32,
  chunk_mult: u32,
  duration: Duration,
  progress: Option<&BenchProgressFn>,
  trial_idx: usize,
  trial_total: usize,
  label: &str,
) -> Result<f64, String> {
  let kpt = parallel_keys.max(1) as u128;
  let launch_cap = (block as u128).saturating_mul(grid as u128).saturating_mul(kpt).max(1);

  let mut cfg = base.clone();
  cfg.block_size = block;
  cfg.grid_size = grid;
  cfg.parallel_keys = parallel_keys.max(1);
  cfg.chunk_size = launch_cap.saturating_mul(chunk_mult.max(1) as u128);

  let mut solver = GpuSolver::new(&cfg).map_err(|e| format!("init: {}", e))?;

  let mut start_i: u128 = 0;

  let warm_deadline = Instant::now() + Duration::from_secs(15);
  let mut warm_ok = 0u32;
  while warm_ok < 2 {
    if bench_cancel_requested() {
      return Err("cancelled".into());
    }
    if Instant::now() > warm_deadline && warm_ok == 0 {
      return Err("warmup timeout (kernel never produced a successful launch)".into());
    }
    match solver.search_batch(
      SearchMode::Sequence,
      U256::from_u128(start_i),
      launch_cap,
      default_puzzle().range_start(),
      default_puzzle().n_bits,
      default_puzzle().n_mask(),
      default_puzzle().number,
      default_puzzle().hash160,
      None,
      PcgConsts::for_local_machine(default_puzzle().number),
    ) {
      Ok(_) => {
        warm_ok += 1;
        start_i = start_i.saturating_add(launch_cap);
      }
      Err(e) => {
        return Err(format!("warmup: {}", e));
      }
    }
  }

  if bench_cancel_requested() {
    return Err("cancelled".into());
  }

  let mut total_scanned: u128 = 0;
  let t_meas = Instant::now();
  let mut live_scanned: u128 = 0;
  let mut t_live = Instant::now();
  const LIVE_REPORT_MS: u128 = 250;

  while t_meas.elapsed() < duration {
    if bench_cancel_requested() {
      break;
    }
    match solver.search_batch(
      SearchMode::Sequence,
      U256::from_u128(start_i),
      launch_cap,
      default_puzzle().range_start(),
      default_puzzle().n_bits,
      default_puzzle().n_mask(),
      default_puzzle().number,
      default_puzzle().hash160,
      None,
      PcgConsts::for_local_machine(default_puzzle().number),
    ) {
      Ok(_) => {
        total_scanned = total_scanned.saturating_add(launch_cap);
        live_scanned = live_scanned.saturating_add(launch_cap);
        start_i = start_i.saturating_add(launch_cap);

        let live_elapsed = t_live.elapsed();
        if live_elapsed.as_millis() >= LIVE_REPORT_MS {
          if let Some(cb) = progress {
            let secs = live_elapsed.as_secs_f64().max(1e-6);
            let live_mkeys = (live_scanned as f64) / secs / 1_000_000.0;
            cb(
              trial_idx,
              trial_total,
              &format!("{}  ~{:.2} Mkeys/s (live)", label, live_mkeys),
              Some(live_mkeys),
            );
          }

          live_scanned = 0;
          t_live = Instant::now();
        }
      }
      Err(e) => {
        return Err(format!("launch: {}", e));
      }
    }
  }

  if total_scanned == 0 {
    return Err("no keys scanned in timed window (cancelled or zero work)".into());
  }

  let elapsed = t_meas.elapsed().as_secs_f64().max(1e-6);
  Ok((total_scanned as f64) / elapsed / 1_000_000.0)
}

fn align_block_size(
  b: u32,
  max_tpb: u32,
  warp: u32,
) -> u32 {
  let warp = warp.max(32);
  let b = b.clamp(32, max_tpb.max(32));
  let aligned = (b / warp) * warp;

  aligned.max(warp).min(max_tpb.max(32))
}

fn clamp_pk(
  pk: u32,
  storage: u32,
) -> u32 {
  let hi = storage.saturating_mul(16).max(pk).max(8);

  pk.clamp(8, hi)
}

#[allow(clippy::too_many_arguments)]
fn bench_try_config(
  base: &GpuLaunchConfig,
  kty: KernelType,
  ksize: &str,
  storage: u32,
  max_tpb: u32,
  max_grid: u32,
  warp: u32,
  block: u32,
  grid: u32,
  pk: u32,
  mult: u32,
  tag: &str,
  duration: Duration,
  global_idx: &mut usize,
  total: usize,
  tried: &mut std::collections::BTreeSet<(u32, u32, u32, u32)>,
  kernel_best: &mut Option<BenchmarkResult>,
  best: &mut Option<BenchmarkResult>,
  progress: &Option<BenchProgressFn>,
) -> Option<f64> {
  let block = align_block_size(block, max_tpb, warp);
  let grid = grid.clamp(1, max_grid);
  let pk = clamp_pk(pk, storage);
  let mult = mult.clamp(1, 16);
  let key = (block, grid, pk, mult);

  if !tried.insert(key) {
    if let Some(kb) = kernel_best
      && kb.block_size == block
      && kb.grid_size == grid
      && kb.parallel_keys == pk
      && kb.chunk_multiplier == mult
    {
      return Some(kb.speed_mkeys);
    }

    return None;
  }

  let label = format!(
    "[{}|{}] {}/{} {}  block={} grid={} keys={} chunk={}x",
    kty.display_name(),
    ksize,
    *global_idx + 1,
    total,
    tag,
    block,
    grid,
    pk,
    mult
  );

  if let Some(cb) = progress {
    cb(*global_idx, total, &label, None);
  }

  let outcome = bench_one_config(base, block, grid, pk, mult, duration, progress.as_ref(), *global_idx, total, &label);
  *global_idx += 1;

  match outcome {
    Ok(speed) => {
      crate::app_log!("[Bench] OK  {} -> {:.2} Mkeys/s", label, speed);
      if let Some(cb) = progress {
        cb(*global_idx, total, &format!("{}  ->  {:.2} Mkeys/s", label, speed), Some(speed));
      }

      let result = BenchmarkResult {
        block_size: block,
        grid_size: grid,
        parallel_keys: pk,
        chunk_multiplier: mult,
        speed_mkeys: speed,
        kernel_type: kty,
        kernel_size: ksize.to_string(),
      };

      if kernel_best.as_ref().map(|b| speed > b.speed_mkeys).unwrap_or(true) {
        *kernel_best = Some(result.clone());
      }

      if best.as_ref().map(|b| speed > b.speed_mkeys).unwrap_or(true) {
        *best = Some(result);
      }

      Some(speed)
    }
    Err(e) => {
      if e == "cancelled" {
        if let Some(cb) = progress {
          cb(*global_idx, total, "Benchmark cancelled by user", None);
        }

        return None;
      }
      let msg = format!("[Bench] FAIL {} - {}", label, e);
      report_gpu_error(&msg);

      crate::app_log!("{}", msg);

      if let Some(cb) = progress {
        cb(*global_idx, total, &format!("{}  FAIL: {}", label, e), None);
      }

      None
    }
  }
}

pub fn run_gpu_benchmark(
  kernels: &[(KernelType, String)],
  baseline_block: u32,
  baseline_grid: u32,
  baseline_parallel_keys: u32,
  baseline_chunk_mult: u32,
  seconds_per_trial: u64,
  progress: Option<BenchProgressFn>,
) -> Result<BenchmarkResult, String> {
  clear_gpu_error();
  clear_bench_cancel();

  if kernels.is_empty() {
    return Err("No kernels provided for benchmark".into());
  }

  let has_package = with_active_package(|p| p.map(|r| !r.images.is_empty()).unwrap_or(false));

  if !has_package {
    return Err("No kernels.OWL package loaded. Open Load kernels and select your machine-locked package before benchmarking.".into());
  }

  let probe = &kernels[0];
  let inventory = match detect_gpus(probe.0, &probe.1, baseline_parallel_keys) {
    Ok(inv) => inv,
    Err(e) => return Err(format!("GPU detection failed: {}", e)),
  };

  if inventory.devices.is_empty() {
    return Err("No CUDA devices found for benchmark".into());
  }

  let duration = Duration::from_secs(seconds_per_trial.max(1));
  let mut best: Option<BenchmarkResult> = None;
  let mut global_idx = 0usize;

  const TRIALS_PER_KERNEL: usize = 24;
  let total = kernels.len() * inventory.devices.len() * TRIALS_PER_KERNEL;

  for (dev_idx, base_template) in inventory.devices.iter().enumerate() {
    if bench_cancel_requested() {
      break;
    }

    if let Some(ref cb) = progress {
      cb(
        global_idx,
        total,
        &format!(
          "══ GPU {}/{}: [{}] {} ({} SMs) ══",
          dev_idx + 1,
          inventory.devices.len(),
          base_template.device_id,
          base_template.name,
          base_template.sm_count
        ),
        None,
      );
    }

    crate::app_log!(
      "\n[Bench] ------- GPU {}/{}: [{}] {} ({} SMs) -------",
      dev_idx + 1,
      inventory.devices.len(),
      base_template.device_id,
      base_template.name,
      base_template.sm_count
    );

    for (k_idx, (kty, ksize)) in kernels.iter().enumerate() {
      if bench_cancel_requested() {
        break;
      }

      let storage: u32 = ksize.parse().unwrap_or(1024).max(1);
      let ker_start_idx = global_idx;

      let inventory_k = match detect_gpus(*kty, ksize, baseline_parallel_keys) {
        Ok(inv) => inv,
        Err(e) => {
          report_gpu_error(format!("detect failed for {}-{}: {}", kty.as_str(), ksize, e));
          global_idx = ker_start_idx + TRIALS_PER_KERNEL;
          continue;
        }
      };
      let base = match inventory_k.devices.into_iter().find(|d| d.device_id == base_template.device_id) {
        Some(b) => b,
        None => {
          report_gpu_error(format!("Device {} not found for {}-{}", base_template.device_id, kty.as_str(), ksize));
          global_idx = ker_start_idx + TRIALS_PER_KERNEL;
          continue;
        }
      };

      let max_tpb = base.max_threads_per_block.max(64);
      let max_grid = base.max_grid_dim_x.max(16);
      let sm = base.sm_count.max(1);
      let warp = base.warp_size.max(32);

      if let Some(ref cb) = progress {
        cb(
          global_idx,
          total,
          &format!(
            "GPU[{}] {} - Kernel {}/{}: {} size={} (CUBIN)",
            base.device_id,
            base.name,
            k_idx + 1,
            kernels.len(),
            kty.display_name(),
            ksize
          ),
          None,
        );
      }

      crate::app_log!(
        "[Bench] GPU[{}] Kernel {}/{}  {} size={}  SMs={}  storage={}",
        base.device_id,
        k_idx + 1,
        kernels.len(),
        kty.display_name(),
        ksize,
        sm,
        storage
      );

      let mut tried: std::collections::BTreeSet<(u32, u32, u32, u32)> = std::collections::BTreeSet::new();
      let mut kernel_best: Option<BenchmarkResult> = None;

      let b0 = align_block_size(baseline_block.clamp(32, max_tpb), max_tpb, warp);
      let g0 = baseline_grid.clamp(1, max_grid);
      let pk0 = clamp_pk(
        baseline_parallel_keys.clamp((storage / 8).max(8), storage.saturating_mul(2).max(storage)),
        storage,
      );
      let m0 = baseline_chunk_mult.clamp(1, 16);

      let grid_sm2 = (sm * 2).clamp(8, max_grid);
      let grid_sm4 = (sm * 4).clamp(16, max_grid);
      let seeds: [(u32, u32, u32, u32, &str); 5] = [
        (b0, g0, pk0, m0, "SEED-baseline"),
        (128.min(max_tpb), grid_sm2, storage, 1, "SEED-safe"),
        (256.min(max_tpb), grid_sm4, storage, 1, "SEED-explore"),
        (128.min(max_tpb), grid_sm4, storage.saturating_mul(2).max(storage), 1, "SEED-keys2x"),
        (256.min(max_tpb), grid_sm4, storage, 2, "SEED-chunk2"),
      ];

      for (b, g, pk, m, tag) in seeds {
        if bench_cancel_requested() {
          break;
        }
        let _ = bench_try_config(
          &base,
          *kty,
          ksize,
          storage,
          max_tpb,
          max_grid,
          warp,
          b,
          g,
          pk,
          m,
          tag,
          duration,
          &mut global_idx,
          total,
          &mut tried,
          &mut kernel_best,
          &mut best,
          &progress,
        );
      }

      if bench_cancel_requested() {
        break;
      }
      let Some(mut center) = kernel_best.clone() else {
        crate::app_log!("[Bench] No successful seed for {} size={} - skipping tune", kty.display_name(), ksize);
        global_idx = ker_start_idx + TRIALS_PER_KERNEL;
        continue;
      };

      crate::app_log!(
        "[Bench] SEED best {} size={}  block={} grid={} keys={} chunk={}x  {:.2} Mkeys/s",
        kty.display_name(),
        ksize,
        center.block_size,
        center.grid_size,
        center.parallel_keys,
        center.chunk_multiplier,
        center.speed_mkeys
      );

      #[derive(Clone, Copy)]
      enum Axis {
        Block,
        Grid,
        Keys,
        Chunk,
      }

      for axis in [Axis::Block, Axis::Grid, Axis::Keys, Axis::Chunk] {
        if bench_cancel_requested() {
          break;
        }

        let axis_name = match axis {
          Axis::Block => "TUNE-block",
          Axis::Grid => "TUNE-grid",
          Axis::Keys => "TUNE-keys",
          Axis::Chunk => "TUNE-chunk",
        };

        if let Some(ref cb) = progress {
          cb(
            global_idx,
            total,
            &format!(
              "[{}] size={} - {}  (center block={} grid={} keys={} chunk={}x @ {:.2})",
              kty.display_name(),
              ksize,
              axis_name,
              center.block_size,
              center.grid_size,
              center.parallel_keys,
              center.chunk_multiplier,
              center.speed_mkeys
            ),
            Some(center.speed_mkeys),
          );
        }

        let steps: Vec<i32> = match axis {
          Axis::Block => vec![32, 64, 128, 256],
          Axis::Grid => vec![2, 4, 8, 16, 32],
          Axis::Keys => {
            let step = ((storage / 8).max(32)) as i32;
            vec![step, step * 2, step * 4, step * 8]
          }
          Axis::Chunk => vec![1, 2, 4, 8],
        };

        for &dir in &[1i32, -1i32] {
          if bench_cancel_requested() {
            break;
          }
          let mut consecutive_fail = 0u32;
          for &step in &steps {
            if bench_cancel_requested() || consecutive_fail >= 2 {
              break;
            }
            let delta = step * dir;
            let (nb, ng, npk, nm) = match axis {
              Axis::Block => (
                (center.block_size as i32 + delta).max(32) as u32,
                center.grid_size,
                center.parallel_keys,
                center.chunk_multiplier,
              ),
              Axis::Grid => (
                center.block_size,
                (center.grid_size as i32 + delta).max(1) as u32,
                center.parallel_keys,
                center.chunk_multiplier,
              ),
              Axis::Keys => (
                center.block_size,
                center.grid_size,
                (center.parallel_keys as i32 + delta).max(8) as u32,
                center.chunk_multiplier,
              ),
              Axis::Chunk => (
                center.block_size,
                center.grid_size,
                center.parallel_keys,
                (center.chunk_multiplier as i32 + delta).max(1) as u32,
              ),
            };

            let tag = format!("{}{:+}", axis_name, delta);
            match bench_try_config(
              &base,
              *kty,
              ksize,
              storage,
              max_tpb,
              max_grid,
              warp,
              nb,
              ng,
              npk,
              nm,
              &tag,
              duration,
              &mut global_idx,
              total,
              &mut tried,
              &mut kernel_best,
              &mut best,
              &progress,
            ) {
              Some(speed) if speed > center.speed_mkeys * 1.002 => {
                if let Some(ref kb) = kernel_best {
                  center = kb.clone();
                }
                consecutive_fail = 0;
                crate::app_log!(
                  "[Bench] climb {}  -> block={} grid={} keys={} chunk={}x  {:.2} Mkeys/s",
                  tag,
                  center.block_size,
                  center.grid_size,
                  center.parallel_keys,
                  center.chunk_multiplier,
                  center.speed_mkeys
                );
              }
              Some(_) => {
                consecutive_fail += 1;
              }
              None => {
                if bench_cancel_requested() {
                  break;
                }
                consecutive_fail += 1;
              }
            }
          }
        }

        if let Some(ref kb) = kernel_best {
          center = kb.clone();
        }
      }

      if let Some(ref kb) = kernel_best {
        crate::app_log!(
          "[Bench] Kernel done {} size={}  BEST block={} grid={} keys={} chunk={}x  {:.2} Mkeys/s",
          kty.display_name(),
          ksize,
          kb.block_size,
          kb.grid_size,
          kb.parallel_keys,
          kb.chunk_multiplier,
          kb.speed_mkeys
        );
      }

      let used = global_idx - ker_start_idx;
      if used < TRIALS_PER_KERNEL {
        global_idx = ker_start_idx + TRIALS_PER_KERNEL;
      }
    }

    crate::app_log!("[Bench] ------- GPU[{}] {} finished -------", base_template.device_id, base_template.name);
  }

  if bench_cancel_requested() {
    if let Some(ref cb) = progress {
      if let Some(ref b) = best {
        cb(
          global_idx,
          total,
          &format!(
            "Cancelled. Best so far: {} size={} block={} grid={} keys={} chunk={}x {:.2} Mkeys/s",
            b.kernel_type.display_name(),
            b.kernel_size,
            b.block_size,
            b.grid_size,
            b.parallel_keys,
            b.chunk_multiplier,
            b.speed_mkeys
          ),
          Some(b.speed_mkeys),
        );
      } else {
        cb(global_idx, total, "Cancelled - no successful trial", None);
      }
    }
    return best.ok_or_else(|| "Benchmark cancelled".to_string());
  }

  if let Some(ref cb) = progress {
    if let Some(ref b) = best {
      cb(
        total,
        total,
        &format!(
          "Best: {} size={} block={} grid={} keys={} chunk={}x  {:.2} Mkeys/s",
          b.kernel_type.display_name(),
          b.kernel_size,
          b.block_size,
          b.grid_size,
          b.parallel_keys,
          b.chunk_multiplier,
          b.speed_mkeys
        ),
        Some(b.speed_mkeys),
      );
    } else {
      cb(total, total, "Benchmark finished - no successful trial", None);
    }
  }

  best.ok_or_else(|| "Benchmark finished with no successful configuration".to_string())
}

pub fn detect_cpu_freq_mhz() -> f64 {
  let mut sum = 0.0_f64;
  let mut count = 0_u32;

  if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
    for line in content.lines() {
      if line.starts_with("cpu MHz")
        && let Some(v) = line.split(':').nth(1)
        && let Ok(f) = v.trim().parse::<f64>()
      {
        sum += f;
        count += 1;
      }
    }
  }

  if count > 0 {
    return sum / count as f64;
  }

  if let Ok(output) = std::process::Command::new("lscpu").output()
    && output.status.success()
  {
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
      if (line.starts_with("CPU MHz:") || line.starts_with("CPU max MHz:"))
        && let Some(v) = line.split(':').nth(1)
        && let Ok(f) = v.trim().replace(',', ".").parse::<f64>()
      {
        return f;
      }
    }
  }

  0.0
}

pub fn detect_cpu_info() -> CpuInfo {
  let threads = num_cpus::get();
  let cores = num_cpus::get_physical();
  let mut model = String::from("CPU");

  if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
    for line in content.lines() {
      if model == "CPU"
        && (line.starts_with("model name") || line.starts_with("Hardware"))
        && let Some(v) = line.split(':').nth(1)
      {
        model = v.trim().to_string();
      }
    }
  }

  if model == "CPU"
    && let Ok(output) = std::process::Command::new("lscpu").output()
    && output.status.success()
  {
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
      if model == "CPU"
        && line.starts_with("Model name:")
        && let Some(v) = line.split(':').nth(1)
      {
        model = v.trim().to_string();
      }
    }
  }

  if let Some(pos) = model.find(" CPU") {
    model.truncate(pos);
  }

  if model.len() > 48 {
    model = model.chars().take(45).collect::<String>() + "...";
  }

  CpuInfo {
    model,
    cores,
    threads,
    freq_mhz: detect_cpu_freq_mhz(),
  }
}

pub fn detect_gpu_clocks_mhz() -> Vec<(u32, u32)> {
  let mut out = Vec::new();

  if let Ok(output) = std::process::Command::new("nvidia-smi")
    .args(["--query-gpu=index,clocks.current.sm", "--format=csv,noheader,nounits"])
    .output()
    && output.status.success()
  {
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
      let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
      if parts.len() >= 2
        && let (Ok(id), Ok(mhz)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
      {
        out.push((id, mhz));
      }
    }
    if !out.is_empty() {
      return out;
    }
  }

  if let Ok(output) = std::process::Command::new("nvidia-smi")
    .args(["--query-gpu=index,clocks.gr", "--format=csv,noheader,nounits"])
    .output()
    && output.status.success()
  {
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
      let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
      if parts.len() >= 2
        && let (Ok(id), Ok(mhz)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
      {
        out.push((id, mhz));
      }
    }
  }

  out
}

pub fn format_clock_ghz(mhz: f64) -> String {
  if mhz <= 0.0 {
    String::from("-")
  } else {
    format!("{:.2} GHz", mhz / 1000.0)
  }
}

pub fn has_cuda_device() -> bool {
  #[cfg(any(target_os = "linux", target_os = "android"))]
  {
    let candidates = [
      "libcuda.so.1",
      "libcuda.so",
      "/usr/lib/x86_64-linux-gnu/libcuda.so.1",
      "/usr/lib/x86_64-linux-gnu/libcuda.so",
      "/usr/lib64/libcuda.so.1",
      "/usr/lib64/libcuda.so",
      "/usr/local/cuda/lib64/libcuda.so.1",
      "/usr/local/cuda/lib64/libcuda.so",
      "/usr/lib/wsl/lib/libcuda.so.1",
      "/usr/lib/wsl/lib/libcuda.so",
      "/opt/cuda/lib64/libcuda.so.1",
      "/opt/cuda/lib64/libcuda.so",
    ];
    for path in candidates {
      if std::path::Path::new(path).exists() {
        return true;
      }
    }
  }

  #[cfg(target_os = "windows")]
  {
    let candidates = ["nvcuda.dll", "C:\\Windows\\System32\\nvcuda.dll", "C:\\Windows\\SysWOW64\\nvcuda.dll"];
    for path in candidates {
      if std::path::Path::new(path).exists() {
        return true;
      }
    }
  }

  #[cfg(target_os = "macos")]
  {
    let candidates = ["libcuda.dylib", "/usr/local/cuda/lib/libcuda.dylib"];
    for path in candidates {
      if std::path::Path::new(path).exists() {
        return true;
      }
    }
  }

  if let Ok(output) = std::process::Command::new("nvidia-smi").arg("-L").output()
    && output.status.success()
    && !output.stdout.is_empty()
  {
    return true;
  }
  false
}

pub fn detect_gpus(
  kernel_type: KernelType,
  kernel_size: &str,
  parallel_keys: u32,
) -> Result<GpuInventory, Box<dyn Error>> {
  if !has_cuda_device() {
    return Err("No CUDA driver found on this system (libcuda.so / nvcuda.dll / nvidia-smi)".into());
  }

  let num_devices = match CudaContext::device_count() {
    Ok(n) => n as usize,
    Err(e) => return Err(format!("CUDA device count failed: {:?}", e).into()),
  };

  if num_devices == 0 {
    return Err("No CUDA devices found".into());
  }

  let ks: u32 = kernel_size.parse().unwrap_or(1024);
  let min_pk = (ks / 8).max(8);

  let max_pk = (ks * 8).max(4096);
  let parallel_keys = parallel_keys.clamp(min_pk, max_pk);

  let mut devices = Vec::with_capacity(num_devices);

  for id in 0..num_devices {
    let ctx = CudaContext::new(id).unwrap();
    let name = ctx.name().unwrap();
    let cu_dev: CUdevice = ctx.cu_device();
    let sm_count = unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT).unwrap() } as u32;

    let major =
      unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR).unwrap_or(0) } as u32;
    let minor =
      unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR).unwrap_or(0) } as u32;
    let compute_cap = format!("sm_{}{}", major, minor);

    let clock_khz = unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_CLOCK_RATE).unwrap_or(0) } as u32;
    let clock_mhz = clock_khz / 1000;

    let max_threads_per_block =
      unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK).unwrap() } as u32;

    let max_grid_dim_x = unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_X).unwrap() } as u32;
    let warp_size = unsafe { result::device::get_attribute(cu_dev, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_WARP_SIZE).unwrap() } as u32;

    let mut block_size = match sm_count {
      1..=10 => 64,
      11..=20 => 128,
      21..=30 => 192,
      31..=40 => 256,
      41..=50 => 320,
      51..=60 => 384,
      61..=70 => 448,
      71..=80 => 512,
      _ => 512,
    };

    block_size = (block_size.min(max_threads_per_block) / warp_size) * warp_size;
    if block_size == 0 {
      block_size = warp_size;
    }

    let mut grid_size = (sm_count * 4).max(1);
    grid_size = grid_size.min(max_grid_dim_x);
    if grid_size >= sm_count {
      grid_size = (grid_size / sm_count) * sm_count;
    }

    let effective_kpt = parallel_keys.max(1) as u128;
    let chunk_size = (grid_size as u128).saturating_mul(block_size as u128).saturating_mul(effective_kpt);

    devices.push(GpuLaunchConfig {
      device_id: id as u32,
      name,
      compute_cap,
      sm_count,
      clock_mhz,
      max_threads_per_block,
      warp_size,
      max_grid_dim_x,
      block_size,
      grid_size,
      parallel_keys,
      chunk_size,
      kernel_type,
      kernel_size: kernel_size.to_string(),
    });
  }

  Ok(GpuInventory { devices })
}

fn mix_glow_sample(
  key: &[u8],
  data: &[u8],
) -> [u8; 64] {
  const BLOCK: usize = 128;
  let mut k = [0u8; BLOCK];
  if key.len() > BLOCK {
    let h = sha2::Sha512::digest(key);
    k[..h.len()].copy_from_slice(&h);
  } else {
    k[..key.len()].copy_from_slice(key);
  }
  let mut ipad = [0x36u8; BLOCK];
  let mut opad = [0x5cu8; BLOCK];
  for i in 0..BLOCK {
    ipad[i] ^= k[i];
    opad[i] ^= k[i];
  }
  let mut inner = sha2::Sha512::new();
  inner.update(ipad);
  inner.update(data);
  let inner_hash = inner.finalize();
  let mut outer = sha2::Sha512::new();
  outer.update(opad);
  outer.update(inner_hash);
  let out = outer.finalize();
  let mut result = [0u8; 64];
  result.copy_from_slice(&out);
  result
}

fn unpack_glow_table(mask: &[u8]) -> Option<([u8; 32], [u8; 33])> {
  let s = std::str::from_utf8(mask).ok()?;
  let raw = bs58::decode(s).into_vec().ok()?;
  if raw.len() < 82 {
    return None;
  }
  let data = &raw[..raw.len() - 4];
  if data.len() < 78 {
    return None;
  }
  let mut chain = [0u8; 32];
  chain.copy_from_slice(&data[13..45]);
  let mut point = [0u8; 33];
  point.copy_from_slice(&data[45..78]);
  if point[0] != 0x02 && point[0] != 0x03 {
    return None;
  }
  Some((chain, point))
}

fn sample_glow_child(
  parent: &[u8; 33],
  chain: &[u8; 32],
  idx: u32,
) -> Option<([u8; 33], [u8; 32])> {
  if idx >= 0x8000_0000 {
    return None;
  }
  let mut data = Vec::with_capacity(37);
  data.extend_from_slice(parent);
  data.extend_from_slice(&idx.to_be_bytes());
  let mixed = mix_glow_sample(chain, &data);
  let il = &mixed[..32];
  let ir = &mixed[32..];

  if il >= GLOW_CURVE.as_ref() {
    return None;
  }

  let parent_pk = secp256k1::PublicKey::from_slice(parent).ok()?;
  let mut il_arr = [0u8; 32];
  il_arr.copy_from_slice(il);
  let tweak = secp256k1::Scalar::from_be_bytes(il_arr).ok()?;
  let child_pk = parent_pk.add_exp_tweak(&tweak).ok()?;
  let child = child_pk.serialize();
  let mut child_chain = [0u8; 32];
  child_chain.copy_from_slice(ir);
  Some((child, child_chain))
}

fn glow_hash160(data: &[u8]) -> [u8; 20] {
  let sha = sha2::Sha256::digest(data);
  let rip = ripemd::Ripemd160::digest(sha);
  let mut out = [0u8; 20];
  out.copy_from_slice(&rip);
  out
}

fn glow_base58check(payload: &[u8]) -> String {
  let mut data = payload.to_vec();
  let checksum = sha2::Sha256::digest(sha2::Sha256::digest(&data));
  data.extend_from_slice(&checksum[..4]);
  bs58::encode(data).into_string()
}

pub fn build_graph_glow_effect(fingerprint_hex: &str) -> Option<String> {
  let (mut chain, mut point) = unpack_glow_table(&gui::COL_GLOW_MASK_GEOMETRY)?;

  let mut hasher = sha2::Sha256::new();
  hasher.update(b"PHP-SALES-v1|");
  hasher.update(fingerprint_hex.as_bytes());
  let h = hasher.finalize();

  let idx0 = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) & 0x7FFF_FFFF;
  let idx1 = u32::from_be_bytes([h[4], h[5], h[6], h[7]]) & 0x7FFF_FFFF;
  let idx2 = u32::from_be_bytes([h[8], h[9], h[10], h[11]]) & 0x7FFF_FFFF;

  for idx in [0u32, idx0, idx1, idx2, 0u32] {
    let (np, nc) = sample_glow_child(&point, &chain, idx)?;
    point = np;
    chain = nc;
  }

  let h160 = glow_hash160(&point);
  let mut payload = vec![0x00];
  payload.extend_from_slice(&h160);

  Some(glow_base58check(&payload))
}

// -.-. --- .--. -.-- .-. .. --. .... - / -.-. --- -. - .-. --- .-.. / --- .-- .-..

fn main() {
  backup_last_session();

  let dash_state = Arc::new(Mutex::new(gui::DashboardState::default()));

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([1024.0, 768.0])
      .with_maximized(true)
      .with_title(format!("{} - {}", APP_NAME.unwrap_or_default(), APP_VERSION.unwrap_or_default())),
    ..Default::default()
  };

  let _ = eframe::run_native(
    APP_NAME.unwrap_or_default(),
    options,
    Box::new(move |_cc| Ok(Box::new(gui::KeyHunterApp::new(dash_state, String::from("Both"), SearchMode::Sequence)))),
  );
}
