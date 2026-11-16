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

use super::PuzzleInfo;

#[derive(Clone, Copy, Debug)]
pub struct TestVector {
  pub puzzle: PuzzleInfo,

  pub _private_key_hex: &'static str,
}

pub static TEST_PUZZLES: &[TestVector] = &[
  TestVector {
    puzzle: PuzzleInfo {
      number: 71,
      n_bits: 70,
      range_start_hex: "400000000000000000",
      address: "1GyeWCp2u3vEBqxfuKHrSwYwt32WWoY4MS",
      hash160: [
        0xaf, 0x40, 0x7c, 0xaa, 0x25, 0xc5, 0xfc, 0x1e, 0xdd, 0xa6, 0xde, 0x8a, 0x79, 0xce, 0xcc, 0x27, 0x4e, 0x35, 0x4f, 0xbc,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000000400000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 72,
      n_bits: 71,
      range_start_hex: "800000000000000000",
      address: "15hUUtVP8UfM5cWxae2qvNWVUKmEAcHJpP",
      hash160: [
        0x33, 0x88, 0x14, 0xd9, 0x6d, 0x41, 0x8d, 0xe0, 0xfa, 0x3c, 0xe9, 0xa0, 0x1d, 0xf7, 0x79, 0xd3, 0xc7, 0xbf, 0xb1, 0x8d,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000000800000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 73,
      n_bits: 72,
      range_start_hex: "1000000000000000000",
      address: "1Lf7rQ15jmoDDnwUV76NFfnDy27L75mbJb",
      hash160: [
        0xd7, 0x9f, 0xea, 0x79, 0xd9, 0x47, 0x7c, 0x1b, 0xd2, 0x4b, 0xab, 0xf6, 0xaa, 0xf0, 0x6f, 0xfe, 0xec, 0x0f, 0xeb, 0xfb,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000001000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 74,
      n_bits: 73,
      range_start_hex: "2000000000000000000",
      address: "1KQxEGzr7nQX7XuwVt5ncLTf2QyNW1L7Y4",
      hash160: [
        0xc9, 0xf9, 0xef, 0x19, 0xd0, 0x40, 0xa0, 0xb8, 0x70, 0x43, 0x44, 0xee, 0x1d, 0x10, 0x4c, 0x96, 0xe2, 0xf2, 0x81, 0x87,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000002000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 76,
      n_bits: 75,
      range_start_hex: "8000000000000000000",
      address: "1MNTEHLWQjwFwpZQQMhiNDj5Xce4Ggn45L",
      hash160: [
        0xdf, 0x71, 0x25, 0x4e, 0xe0, 0xaf, 0xdb, 0x54, 0x15, 0x1c, 0xd0, 0xca, 0x1b, 0x52, 0x1a, 0xe5, 0x2a, 0x90, 0x11, 0x5c,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000008000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 77,
      n_bits: 76,
      range_start_hex: "10000000000000000000",
      address: "1E1QUdBxLB52rSzCmhwpSq8CnWTKfvb76r",
      hash160: [
        0x8e, 0xad, 0x3b, 0x71, 0x3b, 0x32, 0x34, 0x77, 0x23, 0xa8, 0x1d, 0x8d, 0x5a, 0x4f, 0x0d, 0x46, 0xe1, 0x03, 0xaa, 0x3e,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000010000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 78,
      n_bits: 77,
      range_start_hex: "20000000000000000000",
      address: "1JmCoEjGhKogz9jGv8eKxNosjDw2cgTz4H",
      hash160: [
        0xc2, 0xd6, 0x49, 0xd6, 0xfc, 0x5c, 0x25, 0xf2, 0x80, 0xb8, 0x43, 0x9e, 0xdf, 0x64, 0x00, 0x7c, 0xc1, 0x0c, 0x90, 0x74,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000020000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 79,
      n_bits: 78,
      range_start_hex: "40000000000000000000",
      address: "1G8JG84BACjJXwVn3XUpfrdLXxbaxfFHpg",
      hash160: [
        0xa5, 0xeb, 0x34, 0xfe, 0xa9, 0x2f, 0x4c, 0x6a, 0x5f, 0xe5, 0xbb, 0xa0, 0xe2, 0x1f, 0x71, 0x09, 0xf7, 0x81, 0x76, 0xaf,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000040000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 81,
      n_bits: 80,
      range_start_hex: "100000000000000000000",
      address: "1Jkb2UGPvRhdzgUrwM99kJqJjozF6brxh5",
      hash160: [
        0xc2, 0xb8, 0x6d, 0x77, 0xd9, 0x74, 0xb4, 0x39, 0xe2, 0x85, 0xd4, 0x2c, 0xe7, 0xe9, 0x36, 0xee, 0xcf, 0xfe, 0x43, 0xc8,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000100000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 82,
      n_bits: 81,
      range_start_hex: "200000000000000000000",
      address: "15rRwBVXiK4KA4r1tXB6vctfTUBi7rq4CX",
      hash160: [
        0x35, 0x39, 0xb3, 0x01, 0xc6, 0x2d, 0x88, 0xee, 0x09, 0xd1, 0x40, 0x18, 0xfe, 0x1b, 0x2c, 0xde, 0xca, 0xf3, 0x4c, 0x5b,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000200000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 83,
      n_bits: 82,
      range_start_hex: "400000000000000000000",
      address: "1KpGp662sH6hhKLT2P1DsWAepPUxCh63iW",
      hash160: [
        0xce, 0x63, 0x03, 0x6d, 0x3c, 0x97, 0x59, 0x8f, 0x64, 0xc5, 0xb1, 0xe3, 0x5a, 0xc4, 0x61, 0xb4, 0xba, 0x0a, 0x2e, 0x66,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000400000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 84,
      n_bits: 83,
      range_start_hex: "800000000000000000000",
      address: "19dRsD5WVt3bJvbRGzQEWFxWfghDHZvVa3",
      hash160: [
        0x5e, 0xa4, 0xbc, 0xda, 0x04, 0x59, 0xd6, 0x6a, 0xf7, 0xad, 0xa0, 0x51, 0x2f, 0x4a, 0x0e, 0x5a, 0xd3, 0xc1, 0x86, 0x84,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000000800000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 86,
      n_bits: 85,
      range_start_hex: "2000000000000000000000",
      address: "146KvJQ38QwDqw9ybH2KmZq5rZbokoWYYN",
      hash160: [
        0x21, 0xea, 0x3f, 0xfa, 0x8a, 0x10, 0xc9, 0x1d, 0xbb, 0xc3, 0x58, 0x02, 0xdf, 0x95, 0x70, 0x4e, 0x18, 0x21, 0xbc, 0x87,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000002000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 87,
      n_bits: 86,
      range_start_hex: "4000000000000000000000",
      address: "1LJuLbVzMFMvYco3ijvvKRJZHX1Rd8bdAD",
      hash160: [
        0xd3, 0xcd, 0x26, 0x92, 0x76, 0x67, 0x17, 0x4f, 0x16, 0x14, 0x2a, 0x37, 0x96, 0x46, 0x20, 0xcb, 0x0f, 0x27, 0x45, 0x0e,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000004000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 88,
      n_bits: 87,
      range_start_hex: "8000000000000000000000",
      address: "18HpfLg9s5Z8WEHhVCTk3ppMbhpZYitiuh",
      hash160: [
        0x4f, 0xf7, 0x52, 0x40, 0x2f, 0xff, 0x0f, 0x08, 0xde, 0x4d, 0x62, 0x80, 0xf1, 0xa5, 0x1c, 0x3d, 0xa7, 0x7b, 0x0c, 0x19,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000008000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 89,
      n_bits: 88,
      range_start_hex: "10000000000000000000000",
      address: "1Fpu1aZAxwpAePfZxLBbVAXifkbhDgLwfT",
      hash160: [
        0xa2, 0xa0, 0xba, 0x17, 0x42, 0x65, 0xdf, 0x05, 0xc1, 0x24, 0xbd, 0xa0, 0x99, 0x42, 0x22, 0x51, 0xfd, 0xde, 0xdb, 0x1f,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000010000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 91,
      n_bits: 90,
      range_start_hex: "40000000000000000000000",
      address: "1BYHiE8K72THdzsfBpJydymYk7rrxVb7BB",
      hash160: [
        0x73, 0x9c, 0x1a, 0x24, 0xe2, 0x8e, 0xea, 0x9f, 0x7f, 0x8e, 0xd8, 0x88, 0xdf, 0x55, 0x80, 0x3a, 0x68, 0x88, 0x21, 0xd4,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000040000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 92,
      n_bits: 91,
      range_start_hex: "80000000000000000000000",
      address: "1GvaKzKJSyKR5QXRy3kVJKYFP3B4uHRkL9",
      hash160: [
        0xae, 0xab, 0xc0, 0xc6, 0x8c, 0x00, 0xbd, 0xc3, 0x22, 0x99, 0xfe, 0xfd, 0x72, 0x89, 0x37, 0x26, 0x8a, 0x65, 0x79, 0x0b,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000080000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 93,
      n_bits: 92,
      range_start_hex: "100000000000000000000000",
      address: "19MDLu39eeP8k8271j3AddGjwEKcB4djmX",
      hash160: [
        0x5b, 0x93, 0xa1, 0x04, 0x24, 0x51, 0xb8, 0x8f, 0x63, 0x20, 0xd1, 0x14, 0x7d, 0xba, 0x1a, 0xf1, 0x59, 0x28, 0xf9, 0xdf,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000100000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 94,
      n_bits: 93,
      range_start_hex: "200000000000000000000000",
      address: "1KhaFJhdE5edGTfUQ8j71xAopC9FLfNAi4",
      hash160: [
        0xcd, 0x1e, 0xa7, 0xcd, 0x65, 0xdd, 0x22, 0xf2, 0x06, 0xe1, 0xed, 0xcb, 0x82, 0xf0, 0xa8, 0x08, 0x02, 0xe7, 0x7d, 0xc8,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000200000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 96,
      n_bits: 95,
      range_start_hex: "800000000000000000000000",
      address: "14AZCGxmWiwMCUSa1zwDrpnuMHrYFjMW6C",
      hash160: [
        0x22, 0xb6, 0xfe, 0xdb, 0xa7, 0x36, 0x9b, 0xec, 0x55, 0x9b, 0x39, 0x32, 0x10, 0x62, 0xa1, 0xdd, 0xee, 0xf2, 0x19, 0xa1,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000000800000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 97,
      n_bits: 96,
      range_start_hex: "1000000000000000000000000",
      address: "12mEShCECo29ZXbg1VNYdhNFwRTMJ9uUYC",
      hash160: [
        0x13, 0x55, 0xa6, 0x55, 0x50, 0x6b, 0x95, 0x0a, 0x19, 0x63, 0xcc, 0x2c, 0xd8, 0x1e, 0xe1, 0x25, 0x25, 0x01, 0xc5, 0xac,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000001000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 98,
      n_bits: 97,
      range_start_hex: "2000000000000000000000000",
      address: "13gmNiVwxHkB8qdxhkRyv6Yw3VLmdfGFY5",
      hash160: [
        0x1d, 0x75, 0x84, 0x16, 0x4e, 0xa8, 0x2e, 0x3a, 0xce, 0xc2, 0xc1, 0xe7, 0x5f, 0x33, 0xf9, 0xf5, 0xe2, 0x36, 0xaf, 0x01,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000002000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 99,
      n_bits: 98,
      range_start_hex: "4000000000000000000000000",
      address: "1F7s4R47FpJXoABSDcMjYa3yzffiNL6brU",
      hash160: [
        0x9a, 0xde, 0x0b, 0x35, 0xaa, 0x4a, 0x80, 0x23, 0xf0, 0x7d, 0x50, 0xa8, 0x38, 0xa8, 0xcd, 0x97, 0x1c, 0xe8, 0xee, 0x96,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000004000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 101,
      n_bits: 100,
      range_start_hex: "10000000000000000000000000",
      address: "1Fby6MF2qD7DpAZG9iSm5m2erVnHDih4HP",
      hash160: [
        0xa0, 0x2e, 0xba, 0x23, 0x83, 0x9a, 0x7c, 0x0e, 0xd5, 0x81, 0x3c, 0x9d, 0xc7, 0x0c, 0x09, 0x64, 0x1f, 0x7f, 0x46, 0xfd,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000010000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 102,
      n_bits: 101,
      range_start_hex: "20000000000000000000000000",
      address: "17Ms8cUYXuwv4ziJoqYJUoWkj7YPHsfrak",
      hash160: [
        0x45, 0xc2, 0xeb, 0xdf, 0xf5, 0x82, 0x90, 0x8a, 0xf7, 0xeb, 0x22, 0xd0, 0x1a, 0xac, 0x82, 0x14, 0xe0, 0xcf, 0x30, 0xe8,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000020000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 103,
      n_bits: 102,
      range_start_hex: "40000000000000000000000000",
      address: "1C8vFopAsZFApR77GobipXFt9VeKRzo3BS",
      hash160: [
        0x7a, 0x28, 0xc0, 0x29, 0xab, 0x37, 0x24, 0xbe, 0x1c, 0x9d, 0x38, 0x50, 0x8f, 0xa3, 0xcf, 0xd6, 0x8e, 0x82, 0x6f, 0x3f,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000040000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 104,
      n_bits: 103,
      range_start_hex: "80000000000000000000000000",
      address: "139iJ6UJurRPXE9wx3oBgvS4w3Bxkorwp6",
      hash160: [
        0x17, 0x96, 0x0c, 0xe4, 0xce, 0x57, 0x40, 0x20, 0x59, 0x16, 0x35, 0x11, 0x53, 0x15, 0xb3, 0x14, 0xc5, 0x5b, 0xb9, 0x64,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000080000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 106,
      n_bits: 105,
      range_start_hex: "200000000000000000000000000",
      address: "18w7dyTuFxacvybUH5gfRun3gwNNShf11K",
      hash160: [
        0x57, 0x04, 0xe2, 0x13, 0x57, 0xc4, 0xaa, 0xc1, 0x17, 0xa2, 0xd8, 0x94, 0xc0, 0x58, 0x9b, 0x6e, 0x65, 0x04, 0xbf, 0x1a,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000200000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 107,
      n_bits: 106,
      range_start_hex: "400000000000000000000000000",
      address: "1GT2d8vTaSnUQXuyaGTChLcpEdvSe6xhfU",
      hash160: [
        0xa9, 0x76, 0x0e, 0x7e, 0x28, 0x30, 0xfc, 0xeb, 0x8a, 0xae, 0xc7, 0x13, 0x1d, 0x3e, 0x64, 0xa1, 0x7f, 0x56, 0xba, 0x46,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000400000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 108,
      n_bits: 107,
      range_start_hex: "800000000000000000000000000",
      address: "1UGVb71xaYQHX8zVRLV7g4yTEAJezmSWs",
      hash160: [
        0x05, 0x28, 0x29, 0xed, 0xf1, 0xc3, 0x7e, 0xd6, 0xe0, 0xee, 0xae, 0x4f, 0x6e, 0x4d, 0x2d, 0xcc, 0x21, 0x47, 0xbd, 0x54,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000000800000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 109,
      n_bits: 108,
      range_start_hex: "1000000000000000000000000000",
      address: "19FaEA4CmngG4RR2uwnjFy5PCbcLKCqYFb",
      hash160: [
        0x5a, 0x82, 0x90, 0xeb, 0xa9, 0xd8, 0x30, 0x6d, 0xc4, 0x35, 0x13, 0x8c, 0xc6, 0x88, 0x0b, 0xe1, 0x8c, 0x35, 0x35, 0x00,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000001000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 111,
      n_bits: 110,
      range_start_hex: "4000000000000000000000000000",
      address: "19vTqSu8kBFmuQ8NNxQ9YceoJwqgUBR4b9",
      hash160: [
        0x61, 0xdd, 0x74, 0x05, 0xc2, 0x9c, 0x60, 0xa6, 0x31, 0x40, 0x71, 0x1f, 0x6c, 0x79, 0x5f, 0x7e, 0x99, 0x7d, 0xf1, 0x67,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000004000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 112,
      n_bits: 111,
      range_start_hex: "8000000000000000000000000000",
      address: "1rAxcm6Jt6jAoowVFNUaEp1wQbJweDydb",
      hash160: [
        0x09, 0x4c, 0xb2, 0x05, 0xfc, 0xa2, 0xe7, 0x6e, 0x4e, 0xdf, 0x62, 0x13, 0x1a, 0x4b, 0xf1, 0x93, 0x6b, 0x6b, 0x48, 0xac,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000008000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 113,
      n_bits: 112,
      range_start_hex: "10000000000000000000000000000",
      address: "1KtQqZKRttZCHTaEWC2ZXZ8SP6AJ66WBhW",
      hash160: [
        0xcf, 0x2b, 0x60, 0x5e, 0xc4, 0x27, 0xd6, 0xdd, 0xc7, 0x4e, 0xca, 0x14, 0x76, 0xfb, 0xec, 0xf0, 0x3e, 0x21, 0x7e, 0x8b,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000010000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 114,
      n_bits: 113,
      range_start_hex: "20000000000000000000000000000",
      address: "1AZP6dJ5tPhkXqnRiTDKED3BQqVc3xyg7G",
      hash160: [
        0x68, 0xd8, 0xe3, 0x73, 0x12, 0x52, 0x9b, 0x98, 0xa8, 0x28, 0xde, 0xdf, 0x3d, 0xfa, 0xd3, 0x03, 0x7e, 0x12, 0x72, 0x22,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000020000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 116,
      n_bits: 115,
      range_start_hex: "80000000000000000000000000000",
      address: "1A9KBvMD3eNDhPJwJwBGZkEDjQoGBRsumW",
      hash160: [
        0x64, 0x4b, 0xa4, 0x7e, 0x63, 0x48, 0x4f, 0x31, 0xad, 0x76, 0x49, 0x0a, 0x39, 0xbb, 0xf4, 0x85, 0xca, 0xb6, 0x27, 0xd2,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000080000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 117,
      n_bits: 116,
      range_start_hex: "100000000000000000000000000000",
      address: "1JEJtC9gpCHjjPQnkamzBz6g16mXWZ7Jse",
      hash160: [
        0xbc, 0xfe, 0x79, 0x36, 0xb3, 0x09, 0xa2, 0xdf, 0xfc, 0x8f, 0x7d, 0xbc, 0xdc, 0x58, 0xa8, 0xf7, 0xe9, 0x67, 0xc9, 0x78,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000100000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 118,
      n_bits: 117,
      range_start_hex: "200000000000000000000000000000",
      address: "1MWp1ctVYW8kqNynayneg8CrkepiQjeuSg",
      hash160: [
        0xe1, 0x05, 0xd2, 0x33, 0x69, 0xd9, 0x21, 0x7e, 0x6a, 0x46, 0xb7, 0x56, 0x8d, 0x16, 0xb5, 0xc0, 0xc7, 0x00, 0xc8, 0x14,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000200000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 119,
      n_bits: 118,
      range_start_hex: "400000000000000000000000000000",
      address: "176PtRLzj9SJiYSg8Dp9sttT4J3H63PunL",
      hash160: [
        0x42, 0xd5, 0xf0, 0x71, 0xf9, 0x3c, 0x35, 0x31, 0xe6, 0xf7, 0x8f, 0xcd, 0xa0, 0xfd, 0x2a, 0xc0, 0xe1, 0x50, 0x3f, 0xc5,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000000400000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 121,
      n_bits: 120,
      range_start_hex: "1000000000000000000000000000000",
      address: "18Kdtm79iPmiZJeiuuwME6TujQ6q7m2BsS",
      hash160: [
        0x50, 0x4f, 0x29, 0xfe, 0x9b, 0xa0, 0x11, 0xe5, 0xbb, 0x68, 0x7e, 0x74, 0xe8, 0xc0, 0xc6, 0x8e, 0x1c, 0x62, 0xd6, 0x5d,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000001000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 122,
      n_bits: 121,
      range_start_hex: "2000000000000000000000000000000",
      address: "1M7MiPGbVpuCC9t28Pwybq9RbCTvbLgz4f",
      hash160: [
        0xdc, 0x96, 0x4d, 0x5e, 0x38, 0x94, 0x1a, 0xfe, 0x90, 0x1f, 0x73, 0xb8, 0x8f, 0x84, 0x77, 0x56, 0xec, 0x11, 0x93, 0x60,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000002000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 123,
      n_bits: 122,
      range_start_hex: "4000000000000000000000000000000",
      address: "1ACLecsZNixJg5rjKbjqjYqf2Z2SA5sVaC",
      hash160: [
        0x64, 0xde, 0x1c, 0x05, 0x32, 0xe8, 0x84, 0x88, 0x13, 0x81, 0xc3, 0x1e, 0xb6, 0xce, 0x4c, 0x1e, 0x56, 0x4e, 0x8a, 0xde,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000004000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 124,
      n_bits: 123,
      range_start_hex: "8000000000000000000000000000000",
      address: "1ArjXniaV1RmRiZg8RSoxvHYBYRkANEys7",
      hash160: [
        0x6c, 0x21, 0x04, 0x62, 0xc0, 0x26, 0xee, 0x0e, 0x5b, 0x39, 0x0a, 0x63, 0x91, 0x24, 0x22, 0xe6, 0x62, 0x9e, 0x89, 0x8f,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000008000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 126,
      n_bits: 125,
      range_start_hex: "20000000000000000000000000000000",
      address: "1GAx8f19KQRZVfmy44FAwehPrPAZj9HevP",
      hash160: [
        0xa6, 0x6b, 0xa6, 0xfb, 0xa7, 0x3f, 0xcb, 0x30, 0x18, 0x7c, 0xb7, 0xf3, 0x96, 0xd0, 0xe6, 0x59, 0x91, 0xf7, 0x75, 0x31,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000020000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 127,
      n_bits: 126,
      range_start_hex: "40000000000000000000000000000000",
      address: "14YWNoZZwvkumo4Loxfusz2vUA3hcBkhjn",
      hash160: [
        0x26, 0xdd, 0xcb, 0x3f, 0xc2, 0x34, 0xd1, 0x51, 0xef, 0xfe, 0x3c, 0xf8, 0x8a, 0xe4, 0xf1, 0xc7, 0xe7, 0x46, 0x4b, 0x59,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000040000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 128,
      n_bits: 127,
      range_start_hex: "80000000000000000000000000000000",
      address: "17HgPfJvZ2DD2RgU6b9HmV3omgNBCcQNpB",
      hash160: [
        0x44, 0xf8, 0x4a, 0xb2, 0xc5, 0x0c, 0xee, 0x74, 0x96, 0x63, 0x43, 0x22, 0x1f, 0x03, 0xa6, 0xdd, 0x1c, 0x56, 0x77, 0x18,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000080000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 129,
      n_bits: 128,
      range_start_hex: "100000000000000000000000000000000",
      address: "1FnAC82yRwegzvH99ZRHFVt2UUq5FtCkMD",
      hash160: [
        0xa2, 0x1c, 0x26, 0xf2, 0x6d, 0xe8, 0xb2, 0x4a, 0xd5, 0xc0, 0xb1, 0xf5, 0x79, 0xc8, 0x05, 0x0f, 0x4a, 0xb4, 0xb8, 0x1e,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000100000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 131,
      n_bits: 130,
      range_start_hex: "400000000000000000000000000000000",
      address: "13v4G7futDfbi2pNwM2q9BFMLCqj2jRqRv",
      hash160: [
        0x1f, 0xf9, 0x04, 0xf4, 0x99, 0x8d, 0xaa, 0x40, 0x2e, 0x84, 0x6d, 0xe0, 0xd1, 0xd4, 0x0a, 0x7c, 0xa7, 0xc6, 0xe0, 0xe5,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000400000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 132,
      n_bits: 131,
      range_start_hex: "800000000000000000000000000000000",
      address: "18o6t6n5SyeRWXZ6ECEThAHMixUcC81bdH",
      hash160: [
        0x55, 0x80, 0xec, 0x88, 0xde, 0x52, 0x5b, 0x8e, 0xea, 0xf9, 0x0e, 0x72, 0x78, 0x9e, 0xe5, 0x54, 0x74, 0xd5, 0x7f, 0x5d,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000000800000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 133,
      n_bits: 132,
      range_start_hex: "1000000000000000000000000000000000",
      address: "15Nw2xQ3RpxGJEWkj68KqBRvQSSXZsqNQt",
      hash160: [
        0x30, 0x06, 0x57, 0xe9, 0x29, 0xad, 0x60, 0xa1, 0x5f, 0xde, 0x8a, 0x84, 0xf3, 0x46, 0x54, 0xe4, 0x1c, 0xf5, 0x6a, 0x8b,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000001000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 134,
      n_bits: 133,
      range_start_hex: "2000000000000000000000000000000000",
      address: "1Ai5njfWJ7aE6VyZC8H9NFP28yaA5dPc2e",
      hash160: [
        0x6a, 0x7e, 0x2f, 0x0b, 0x18, 0x67, 0x0e, 0xde, 0x65, 0xfc, 0xf1, 0x04, 0x0b, 0xa6, 0xf1, 0x3a, 0x03, 0x5e, 0x53, 0xf7,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000002000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 136,
      n_bits: 135,
      range_start_hex: "8000000000000000000000000000000000",
      address: "19A6dTKtC5iAYHbsqs39VF9sqg9uCW9LXd",
      hash160: [
        0x59, 0x79, 0x72, 0xc8, 0xa8, 0x9b, 0xef, 0xa4, 0xcb, 0x4d, 0xe7, 0xd7, 0x59, 0x2a, 0x6a, 0x65, 0x90, 0x20, 0xb0, 0xf1,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000008000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 137,
      n_bits: 136,
      range_start_hex: "10000000000000000000000000000000000",
      address: "1GrdBhQd1at8m3dCLaLjh92URqQvT22maG",
      hash160: [
        0xad, 0xec, 0x79, 0x60, 0x08, 0x51, 0xa9, 0x4d, 0x27, 0x5b, 0xcd, 0x42, 0x66, 0x57, 0x8b, 0x49, 0x95, 0xdf, 0x92, 0xc0,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000010000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 138,
      n_bits: 137,
      range_start_hex: "20000000000000000000000000000000000",
      address: "1DTbCTdtMic44AnFyuKQg9kFmvYuQy6iaM",
      hash160: [
        0x88, 0xa8, 0xdf, 0x00, 0xf3, 0xe2, 0x7e, 0x68, 0x0b, 0x28, 0x74, 0xd9, 0xb9, 0x17, 0xb2, 0x79, 0x8b, 0x78, 0x23, 0x27,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000020000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 139,
      n_bits: 138,
      range_start_hex: "40000000000000000000000000000000000",
      address: "1C45DgjcqNJ6HJNjtpuM8B15kvGxqLxusC",
      hash160: [
        0x79, 0x3e, 0x27, 0x32, 0x09, 0xec, 0xb2, 0xfa, 0x8c, 0x9e, 0xfc, 0x52, 0xa1, 0x6e, 0x94, 0x00, 0x74, 0x0d, 0x21, 0xe6,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000040000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 140,
      n_bits: 139,
      range_start_hex: "80000000000000000000000000000000000",
      address: "158prKvuk7hKxwmSRZ28pFjquUoY2qxN4Q",
      hash160: [
        0x2d, 0x5b, 0x5b, 0xc1, 0x20, 0xa2, 0x86, 0x57, 0x13, 0x1d, 0x69, 0x11, 0x8a, 0x6c, 0xdd, 0xf3, 0xa1, 0xd2, 0x0c, 0x28,
      ],
      pubkey_hex: Some("02113cfbd0c8c402b6862f4665c69b63d04ac2aa85979389a3f8776ec1a3d2f5cb"),
    },
    _private_key_hex: "0000000000000000000000000000080000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 141,
      n_bits: 140,
      range_start_hex: "100000000000000000000000000000000000",
      address: "1AvK3FTc3qANQ51M8Vmc9ZeR9oXKgmaeB7",
      hash160: [
        0x6c, 0xce, 0x3c, 0x6d, 0x6d, 0xb0, 0x97, 0x21, 0x03, 0xbb, 0xf6, 0x45, 0x6f, 0xcd, 0x6a, 0x6a, 0x89, 0xce, 0x09, 0x0c,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000100000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 142,
      n_bits: 141,
      range_start_hex: "200000000000000000000000000000000000",
      address: "1PLPxrWDZNrArWzYeDrGsaF3X1vewtTQA2",
      hash160: [
        0xf4, 0xfd, 0xd6, 0xcc, 0x73, 0x58, 0xea, 0x2b, 0x81, 0x31, 0x22, 0x11, 0xde, 0x9a, 0x9c, 0xb1, 0x76, 0x39, 0x79, 0x9d,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000200000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 143,
      n_bits: 142,
      range_start_hex: "400000000000000000000000000000000000",
      address: "1KX2edXtS3sPKcdUB67aJFLkFtp6eMbpLj",
      hash160: [
        0xcb, 0x20, 0x1e, 0x8a, 0xa4, 0x12, 0x0c, 0x7f, 0x2a, 0xff, 0x86, 0x65, 0xdb, 0xb2, 0x88, 0xd5, 0x25, 0xdc, 0x04, 0x9b,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000400000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 144,
      n_bits: 143,
      range_start_hex: "800000000000000000000000000000000000",
      address: "1cPoq52z74e6x1xgTCYSKaUAbSvqDu6zi",
      hash160: [
        0x06, 0xb1, 0x98, 0xe9, 0xa9, 0x72, 0xd9, 0xee, 0x56, 0x6c, 0xfc, 0x5c, 0x13, 0xe5, 0x4b, 0x19, 0x26, 0x87, 0x7e, 0x41,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000000800000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 145,
      n_bits: 144,
      range_start_hex: "1000000000000000000000000000000000000",
      address: "1PqJUq7aukpoLehYVdnGDEK7SbevvStUGn",
      hash160: [
        0xfa, 0x75, 0x53, 0x27, 0xe8, 0x0b, 0x6b, 0x5f, 0x3f, 0x69, 0xdc, 0x2a, 0x85, 0xde, 0x97, 0x0f, 0x88, 0x29, 0xea, 0x16,
      ],
      pubkey_hex: Some("027e43a8f1a10928fe3ba29b85b41510b62a58744c3476c337076e26f05746552a"),
    },
    _private_key_hex: "0000000000000000000000000001000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 146,
      n_bits: 145,
      range_start_hex: "2000000000000000000000000000000000000",
      address: "1HJAegzSGzQm6zDszKeB1aT4vDNEk6vhYq",
      hash160: [
        0xb2, 0xc1, 0x23, 0x9f, 0x96, 0xe3, 0x53, 0x80, 0xd0, 0x6a, 0xb4, 0x9f, 0xc8, 0xe4, 0xf5, 0xcc, 0xfc, 0x9e, 0x38, 0xc4,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000002000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 147,
      n_bits: 146,
      range_start_hex: "4000000000000000000000000000000000000",
      address: "16WVojTw2eYn5esLtRQ4RTdE44zrgLH6d5",
      hash160: [
        0x3c, 0x6c, 0xbc, 0xd4, 0xb1, 0x12, 0x4a, 0x51, 0x9b, 0x7e, 0x66, 0x01, 0x96, 0x70, 0xdb, 0x42, 0x34, 0xac, 0x48, 0xaa,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000004000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 148,
      n_bits: 147,
      range_start_hex: "8000000000000000000000000000000000000",
      address: "17rr2v7GLzzMRQ1e54QWH3vZWnsYCLu3hP",
      hash160: [
        0x4b, 0x3e, 0x11, 0x4a, 0x63, 0x22, 0x60, 0x37, 0xd0, 0x5a, 0xf8, 0xbb, 0x9d, 0xb6, 0x3d, 0x94, 0xff, 0x9a, 0xc4, 0x58,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000008000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 149,
      n_bits: 148,
      range_start_hex: "10000000000000000000000000000000000000",
      address: "1MX7UogmGmwzGksmsk9TRYjEBg3fhvqgUz",
      hash160: [
        0xe1, 0x14, 0x67, 0x3b, 0xc8, 0xe1, 0x91, 0x11, 0x09, 0x7f, 0xd9, 0x8e, 0x4d, 0x53, 0x01, 0x01, 0x78, 0xec, 0x7b, 0x61,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000010000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 150,
      n_bits: 149,
      range_start_hex: "20000000000000000000000000000000000000",
      address: "1D5q4jtAx1W5wsqrumK3n7Hb2VKhzbtH1S",
      hash160: [
        0x84, 0x8b, 0x4b, 0xa2, 0xcb, 0x4f, 0x44, 0xbf, 0xb6, 0xcb, 0x33, 0xc9, 0x28, 0x75, 0x35, 0x01, 0x52, 0x5e, 0xc0, 0xf5,
      ],
      pubkey_hex: Some("02d3c025607428286721f998c7b999712e59ad89e0e0b7ffee2f9db05bae89fb6c"),
    },
    _private_key_hex: "0000000000000000000000000020000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 151,
      n_bits: 150,
      range_start_hex: "40000000000000000000000000000000000000",
      address: "1GSqXJ7JuRd9JkaGy1HMaKQWLBZKSKeMAD",
      hash160: [
        0xa9, 0x6c, 0xca, 0x4a, 0x92, 0x55, 0xca, 0xa3, 0x68, 0xdf, 0x63, 0x4b, 0xe5, 0x37, 0xc9, 0xb2, 0xde, 0x20, 0x67, 0x42,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000040000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 152,
      n_bits: 151,
      range_start_hex: "80000000000000000000000000000000000000",
      address: "1NETL44XXbQhsMYYC7uU9pRzCoLxpEwnkr",
      hash160: [
        0xe8, 0xe6, 0x06, 0xe1, 0xe3, 0xf2, 0x2b, 0xa7, 0xc4, 0x10, 0xa0, 0x11, 0xea, 0x87, 0x82, 0xc0, 0x5a, 0x51, 0xf5, 0x4d,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000080000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 153,
      n_bits: 152,
      range_start_hex: "100000000000000000000000000000000000000",
      address: "1QERyWtrFdA6EPKWQ9XSarHDcjq4J5AuBG",
      hash160: [
        0xfe, 0xd5, 0x25, 0xea, 0xb2, 0xa1, 0xb1, 0x2c, 0x3b, 0xaf, 0xd1, 0x4e, 0x08, 0x16, 0xe8, 0x31, 0xc3, 0x24, 0xe0, 0xe6,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000100000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 154,
      n_bits: 153,
      range_start_hex: "200000000000000000000000000000000000000",
      address: "134gNuPBAVRDeWyMRVutakGcvMUSx3tqPp",
      hash160: [
        0x16, 0xa2, 0x5e, 0xc8, 0xd1, 0xfa, 0xf0, 0x39, 0x0c, 0x8c, 0xd4, 0xbc, 0xe0, 0x7f, 0xa7, 0x54, 0x00, 0x3c, 0x08, 0xd7,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000200000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 155,
      n_bits: 154,
      range_start_hex: "400000000000000000000000000000000000000",
      address: "1MXrx35cGoXgLyhBY4VSiZEd5dLgyZHQVh",
      hash160: [
        0xe1, 0x38, 0xb0, 0x94, 0x84, 0x68, 0x56, 0xcd, 0x6d, 0x2f, 0xcc, 0x9d, 0xe6, 0x21, 0x17, 0x73, 0xbe, 0xdd, 0xd5, 0x2a,
      ],
      pubkey_hex: Some("02ca88d2b2f7c5e9ec2f413e5879bac7879e86e23dfa8b16bc32d1159331f13260"),
    },
    _private_key_hex: "0000000000000000000000000400000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 156,
      n_bits: 155,
      range_start_hex: "800000000000000000000000000000000000000",
      address: "1F9Xtf4x8fUupDhNKzL9btvHA7BNexkzqr",
      hash160: [
        0x9b, 0x2e, 0xdf, 0xf7, 0x12, 0x45, 0x8c, 0xfb, 0x54, 0x7b, 0x44, 0x51, 0x1d, 0xae, 0xbc, 0x44, 0x51, 0x37, 0x61, 0x86,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000000800000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 157,
      n_bits: 156,
      range_start_hex: "1000000000000000000000000000000000000000",
      address: "15bXcA3Z1GZS9L7pkkLQ6Sgiw4BoeyUjsY",
      hash160: [
        0x32, 0x68, 0x31, 0xce, 0x2d, 0x80, 0x74, 0xa1, 0x20, 0xa8, 0xda, 0xf2, 0x99, 0xdc, 0xfd, 0x7c, 0x1e, 0xa9, 0x92, 0x68,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000001000000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 158,
      n_bits: 157,
      range_start_hex: "2000000000000000000000000000000000000000",
      address: "1De2k4tSrv2ZVTLhdYZypsuq6AZTbyFyDr",
      hash160: [
        0x8a, 0xa2, 0x5a, 0xc6, 0x2b, 0x33, 0xf2, 0x8e, 0xe9, 0xdf, 0x45, 0x30, 0x36, 0x14, 0x07, 0x2f, 0xce, 0xb6, 0x13, 0x8f,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000002000000000000000000000000000000050000000",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 159,
      n_bits: 158,
      range_start_hex: "4000000000000000000000000000000000000000",
      address: "1MkxiTN8h6F4nPVmgvcrp4TYDjmkWmccx7",
      hash160: [
        0xe3, 0xb2, 0xe9, 0x0c, 0xb4, 0x10, 0x17, 0xa6, 0x58, 0x6c, 0x5b, 0x4a, 0xe0, 0x78, 0x32, 0x5c, 0x19, 0xbe, 0xb3, 0x07,
      ],
      pubkey_hex: None,
    },
    _private_key_hex: "0000000000000000000000004000000000000000000000000000000050000001",
  },
  TestVector {
    puzzle: PuzzleInfo {
      number: 160,
      n_bits: 159,
      range_start_hex: "8000000000000000000000000000000000000000",
      address: "1PM4oThUHVVb2dowg9FJcDAYKZ4UYNLjXw",
      hash160: [
        0xf5, 0x1e, 0x42, 0x69, 0xd0, 0x7b, 0x87, 0x44, 0x86, 0xcb, 0xb7, 0xd7, 0x93, 0x0c, 0xb5, 0x1a, 0xc2, 0x04, 0x12, 0xeb,
      ],
      pubkey_hex: Some("02faa4ac25a1f14e119ef645fccb8e5b2d6c56572e9a0b6feef7018f2b2957b0bb"),
    },
    _private_key_hex: "0000000000000000000000008000000000000000000000000000000050000000",
  },
];

pub fn test_puzzle_infos() -> &'static [PuzzleInfo] {
  use std::sync::OnceLock;
  static INFOS: OnceLock<&'static [PuzzleInfo]> = OnceLock::new();
  INFOS.get_or_init(|| {
    let v: Vec<PuzzleInfo> = TEST_PUZZLES.iter().map(|tv| tv.puzzle).collect();
    Box::leak(v.into_boxed_slice())
  })
}

#[cfg(test)]
mod tests {
  use super::super::{PcgConsts, SearchMode, U256, permute_index_u256, resolve_key_from_index};
  use super::*;

  fn parse_priv_hex(hex: &str) -> [u8; 32] {
    let h = hex.trim().trim_start_matches("0x");
    assert_eq!(h.len(), 64, "private key must be 32 bytes (64 hex chars)");
    let mut out = [0u8; 32];
    for i in 0..32 {
      out[i] = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).expect("hex");
    }
    out
  }

  fn priv_to_u256(priv_be: &[u8; 32]) -> U256 {
    let mut hi_bytes = [0u8; 16];
    let mut lo_bytes = [0u8; 16];
    hi_bytes.copy_from_slice(&priv_be[..16]);
    lo_bytes.copy_from_slice(&priv_be[16..]);
    U256 {
      hi: u128::from_be_bytes(hi_bytes),
      lo: u128::from_be_bytes(lo_bytes),
    }
  }

  #[test]
  fn test_vector_table_not_empty() {
    assert!(!TEST_PUZZLES.is_empty());
    assert_eq!(test_puzzle_infos().len(), TEST_PUZZLES.len());
  }

  #[test]
  fn every_test_key_in_range_and_offset_pattern() {
    for (i, tv) in TEST_PUZZLES.iter().enumerate() {
      let priv_be = parse_priv_hex(tv._private_key_hex);
      let key = priv_to_u256(&priv_be);
      let start = tv.puzzle.range_start();
      let end = tv.puzzle.range_end();
      assert!(
        key >= start && key <= end,
        "#{} key {} outside [{}, {}]",
        tv.puzzle.number,
        key.to_hex(),
        start.to_hex(),
        end.to_hex()
      );
      let delta = key.saturating_sub(start);
      let expect = if i % 2 == 0 { 0x5000_0000u128 } else { 0x5000_0001u128 };
      assert_eq!(
        delta.as_u128(),
        Some(expect),
        "#{} expected offset {:#x}, got {}",
        tv.puzzle.number,
        expect,
        delta.to_hex()
      );
    }
  }

  #[test]
  fn sequence_roundtrip_all_vectors() {
    for tv in TEST_PUZZLES {
      let priv_be = parse_priv_hex(tv._private_key_hex);
      let key = priv_to_u256(&priv_be);
      let start = tv.puzzle.range_start();
      let index = key.saturating_sub(start);
      let pcg = PcgConsts::for_local_machine(tv.puzzle.number);
      let resolved = resolve_key_from_index(SearchMode::Sequence, index, start, tv.puzzle.n_bits, tv.puzzle.n_mask(), &pcg).expect("resolve");
      assert_eq!(resolved.to_hex(), key.to_hex(), "#{}", tv.puzzle.number);
    }
  }

  #[test]
  fn pcg_odd_multiplier_sample() {
    let tv = &TEST_PUZZLES[0];
    let pcg = PcgConsts::for_local_machine(tv.puzzle.number);
    assert_eq!(pcg.a.lo & 1, 1, "PCG a must be odd");
    let n_mask = tv.puzzle.n_mask();
    let mut seen = std::collections::HashSet::new();
    for i in 0u128..128 {
      let p = permute_index_u256(U256::from_u128(i), n_mask, &pcg);
      assert!(seen.insert(p.to_hex()));
    }
  }
}
