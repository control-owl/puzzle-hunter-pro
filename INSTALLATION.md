This tutorial covers building **puzzle-hunter-pro** from source on Ubuntu, Arch, Fedora, and Windows. 

On every system you need:

1. **Rust 1.85 or newer** (required by edition 2024)
2. **NVIDIA driver**, and the **CUDA 11.0 or 12.0 or 13.0 toolkit** depending on your card.
3. **GnuPG (`gpg`)** - required to generate license requests and to verify/load signed `.OWL` kernel packages. Without it the application will refuse to load any commercial kernel package.

In this tutorial we are assuming that you are running latest OS with latest drivers.

| System | NVIDIA driver | CUDA toolkit | Build dependencies |
|---|---|---|---|
| Ubuntu 22.04/24.04 | `nvidia-driver-580` from the Ubuntu repos | `cuda-toolkit-13-0` from NVIDIA's apt repo | `build-essential`, `clang`, GTK3 and X11/Wayland dev headers |
| Arch Linux | `nvidia` and `nvidia-utils` from the official repos | `cuda` from the official repos (installs to `/opt/cuda`) | `base-devel`, `clang`, `gtk3`, `libxkbcommon`, `wayland` |
| Fedora | `akmod-nvidia` from RPM Fusion | `cuda-toolkit-13-0` from NVIDIA's dnf repo | `gcc-c++`, `clang`, GTK3 and X11/Wayland dev headers |
| Windows 10/11 | NVIDIA driver 580 or newer | CUDA 13.0 `.exe` installer | Visual Studio Build Tools with the C++ workload |

---

## Prerequisites on every system

Install Rust with rustup on all platforms. On Linux, run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`, then restart your shell. On Windows, download `rustup-init.exe` from rustup.rs. Confirm the install with `rustc --version`, which should report 1.85 or newer.

**GnuPG is mandatory at runtime.** The host uses `gpg` to encrypt the license request to the developer and to verify the detached signature on every `.OWL` package. Install it with your package manager (`gnupg` on Debian/Ubuntu/Fedora, `gnupg` on Arch, Gpg4win or `winget install GnuPG.GnuPG` on Windows).

The build does not invoke `nvcc`. The `cudarc` crate loads the driver (`libcuda.so` or `nvcuda.dll`) and NVRTC (the runtime compiler for GPU kernels) at runtime, so the toolkit matters mainly for its runtime libraries. Install it anyway, because the program needs NVRTC when it runs.

---

## Ubuntu (22.04 and 24.04)

Ubuntu needs the build tools, the GUI libraries used by `eframe` and `rfd`, and the NVIDIA driver and toolkit. The commands below assume Ubuntu 24.04. For 22.04, replace `ubuntu2404` with `ubuntu2204` in the keyring URL.

### Install build dependencies

```bash
sudo apt update
sudo apt install -y build-essential pkg-config clang git curl
```

### Install the NVIDIA driver and CUDA toolkit

Install the driver first and reboot, then add NVIDIA's repository and install the toolkit:

```bash
sudo apt install -y nvidia-driver-580
sudo reboot
```

```bash
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt update
sudo apt install -y cuda-toolkit-13-0
```

Add the toolkit to your environment:

```bash
echo 'export PATH=/usr/local/cuda-13.0/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/usr/local/cuda-13.0/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc
```

Run `nvidia-smi` to confirm the driver sees your GPU, and `nvcc --version` to confirm the toolkit.

---

## Arch Linux

Arch packages the driver and toolkit in its official repositories, so you don't need an external repo. The `cuda` package is usually the newest 13.x release, which should work with the `cuda-13040` feature. If `cargo` reports a version mismatch, check the toolkit version with `nvcc --version`.

### Install build dependencies

```bash
sudo pacman -Syu
sudo pacman -S --needed base-devel git curl pkgconf clang
```

### Install the NVIDIA driver and CUDA toolkit

```bash
sudo pacman -S --needed nvidia nvidia-utils cuda
sudo reboot
```

If you use a custom or LTS kernel, install `nvidia-dkms` plus the matching headers (for example `linux-lts-headers`) instead of `nvidia`.

The `cuda` package installs to `/opt/cuda`. Add it to your environment:

```bash
echo 'export PATH=/opt/cuda/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/opt/cuda/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc
```

Verify with `nvidia-smi` and `nvcc --version`.

---

## Fedora

Fedora does not ship proprietary NVIDIA drivers, so you need RPM Fusion for the driver and NVIDIA's repository for the toolkit. Fedora's kernel modules are built by `akmods`, which takes a few minutes after install.

### Install build dependencies

```bash
sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config clang git curl
```

### Install the NVIDIA driver from RPM Fusion

```bash
sudo dnf install -y \
  https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm \
  https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
sudo dnf install -y akmod-nvidia xorg-x11-drv-nvidia-cuda
```

Wait for the module to build before rebooting. You can check progress with `sudo akmods --force`, then confirm with `modinfo -F version nvidia`. If Secure Boot is enabled, you must enroll the Machine Owner Key when prompted, or the module will not load. Reboot after the build finishes.

### Install the CUDA toolkit

Add NVIDIA's repo for your Fedora release. The URL uses the release number, so replace `fedora41` with the directory matching your version if NVIDIA's listing differs:

```bash
sudo dnf config-manager addrepo \
  --from-repofile=https://developer.download.nvidia.com/compute/cuda/repos/fedora41/x86_64/cuda-fedora41.repo
sudo dnf install -y cuda-toolkit-13-0
```

Add the toolkit to your environment:

```bash
echo 'export PATH=/usr/local/cuda-13.0/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/usr/local/cuda-13.0/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc
```

Verify with `nvidia-smi` and `nvcc --version`.

---

## Windows

On Windows, the NVIDIA driver and CUDA toolkit come as installers, and the C++ build tools are needed for Rust's MSVC toolchain and for the C code in some dependencies.

### Install the prerequisites

1. Install **Git for Windows** from git-scm.com.
2. Install **Visual Studio Build Tools** from visualstudio.microsoft.com. In the installer, select the **Desktop development with C++** workload, which includes the MSVC compiler and the Windows SDK.
3. Run **rustup-init.exe** and accept the default `x86_64-pc-windows-msvc` toolchain.
4. Install an **NVIDIA driver version 580 or newer** from nvidia.com/drivers or through the NVIDIA app.
5. Download the **CUDA 13.0 toolkit** `.exe` (local installer) from developer.nvidia.com/cuda-downloads. In the installer, choose Custom and leave the driver component unchecked if your driver is already newer. The installer sets `CUDA_PATH` and adds the toolkit's `bin` folder to PATH.

Restart PowerShell after installing, then check the setup:

```powershell
rustc --version
nvidia-smi
nvcc --version
```

The GUI uses the native Windows backend, so it needs no extra libraries. The program loads `nvcuda.dll` from the driver and the NVRTC DLL from the toolkit at runtime.

---

## Clone, build, and run

These steps are the same on all platforms. Run them in a terminal (PowerShell on Windows).

```bash
git clone -b stable --single-branch https://github.com/control-owl/puzzle-hunter-pro.git
cd puzzle-hunter-pro
cargo build --release
```

The first build takes several minutes because it compiles the GUI stack and the C code in `secp256k1` and `zstd`. Choose the feature set that matches your GPU:

| Goal | Command |
|---|---|
| Default, CUDA 13.0 (sm_50 to sm_121) | `cargo build --release` |
| CUDA 12.3 (sm_50 to sm_120) | `cargo build --release --no-default-features --features cuda-12030` |
| Legacy CUDA 11.8 (sm_30 to sm_37) | `cargo build --release --no-default-features --features cuda-11080` |

---

## SM Architecture List

| Compute Capability | SM Name | Architecture | Key GPU Examples                            | Optimal kernel size | cuda-11080 | cuda-12030 | cuda-13040 |
| ------------------ | ------- | ------------ | ------------------------------------------- | ------------------- | ---------- | ---------- | ---------- |
| 3.0                | sm_30   | Kepler       | Tesla K20, K40                              | 128 / 256           | +          |            |            |
| 3.2                | sm_32   | Kepler       | Tesla K20X                                  | 128 / 256           | +          |            |            |
| 3.5                | sm_35   | Kepler       | Tesla K40m, GTX 750 Ti                      | 128 / 256           | +          |            |            |
| 3.7                | sm_37   | Kepler       | Tesla K80                                   | 128 / 256           | +          |            |            |
| 5.0                | sm_50   | Maxwell      | Tesla M40, GTX 750                          | 256 / 512           |            | +          | +          |
| 5.2                | sm_52   | Maxwell      | GTX 960, GTX 970, GTX 980                   | 256 / 512           |            | +          | +          |
| 5.3                | sm_53   | Maxwell      | Jetson TX1                                  | 128 / 256           |            | +          | +          |
| 6.0                | sm_60   | Pascal       | Tesla P100                                  | 512 / 1024          |            | +          | +          |
| 6.1                | sm_61   | Pascal       | GTX 1080, GTX 1070, GTX 1060                | 512 / 1024          |            | +          | +          |
| 6.2                | sm_62   | Pascal       | Jetson TX2                                  | 128 / 256           |            | +          | +          |
| 7.0                | sm_70   | Volta        | Tesla V100                                  | 1024 / 2048         |            | +          | +          |
| 7.2                | sm_72   | Volta        | Jetson AGX Xavier                           | 256 / 512           |            | +          | +          |
| 7.5                | sm_75   | Turing       | Tesla T4, RTX 2060-2080 Ti, TITAN RTX, 1650 | 1024 / 2048         |            | +          | +          |
| 8.0                | sm_80   | Ampere       | A100, A30                                   | 1024 / 2048         |            | +          | +          |
| 8.6                | sm_86   | Ampere       | RTX 3050-3090 Ti, RTX A-series, A40, A10    | 1024 / 2048         |            | +          | +          |
| 8.7                | sm_87   | Ampere       | Jetson AGX Orin, Orin NX, Orin Nano         | 256 / 512           |            | +          | +          |
| 8.9                | sm_89   | Ada Lovelace | RTX 40-series, L4, L40, L40S                | 1024 / 2048         |            | +          | +          |
| 9.0                | sm_90   | Hopper       | H100, H200, GH200                           | 2048 / 4096         |            | +          | +          |
| 9.0a               | sm_90a  | Hopper       | H100 (with feature enhancements)            | 2048 / 4096         |            | +          | +          |
| 10.0               | sm_100  | Blackwell    | GB200, B200                                 | 2048 / 4096         |            | +          | +          |
| 10.3               | sm_103  | Blackwell    | GB300                                       | 2048 / 4096         |            | +          | +          |
| 11.0               | sm_110  | Blackwell    | Jetson T5000, T4000                         | 512 / 1024          |            | +          | +          |
| 12.0               | sm_120  | Blackwell    | RTX 5090, 5080, 5070, 5060, RTX PRO series  | 2048 / 4096         |            | +          | +          |
| 12.1               | sm_121  | Blackwell    | RTX Spark, DGX Spark                        | 1024 / 2048         |            |            | +          |
