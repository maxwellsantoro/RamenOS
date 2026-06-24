# Getting Started with RamenOS

A comprehensive guide for new contributors to set up their development environment and start working with RamenOS.

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Development Environment Setup](#development-environment-setup)
4. [Building RamenOS](#building-ramenos)
5. [Running in QEMU](#running-in-qemu)
6. [Running Foundry Gates](#running-foundry-gates)
7. [Architecture-Specific Notes](#architecture-specific-notes)
8. [Common Issues and Troubleshooting](#common-issues-and-troubleshooting)
9. [Next Steps](#next-steps)

---

## Introduction

### What is RamenOS?

RamenOS is a reliability-first, post-Unix operating system built around:

- **Typed Harnesses + Portals**: No ioctl-style escape hatches in native interfaces
- **Quarantined Compatibility Domains**: Isolated environments for Linux/Flatpak and GPU blobs
- **Unified Foundry Pipeline**: Trace, Replay, Fuzz, Minimize, Gate workflow for drivers and app ports

The project is organized as three pillars:
1. **OS Core** (kernel + services + runtimes)
2. **Foundry** (tooling + CI gates)
3. **Store Platform** (Run Now, Vote/Port, Publish)

### Who This Guide Is For

This guide is for developers who want to:
- Contribute to the RamenOS codebase
- Understand the build system and architecture
- Run and test the OS in QEMU
- Work on kernel, services, or tooling components

### What You'll Learn

By the end of this guide, you will be able to:
- Set up a complete development environment
- Build the kernel and services for multiple architectures
- Run RamenOS in QEMU with UEFI boot
- Execute Foundry gates to verify functionality
- Troubleshoot common issues

---

## Prerequisites

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | nightly-2026-02-08 | Kernel and services development |
| QEMU | 7.0+ | x86_64 and aarch64 emulation |
| OVMF | UEFI firmware | UEFI boot support for x86_64 |
| just | 1.0+ | Command runner for justfile |
| Git | 2.0+ | Version control |
| Python | 3.8+ | Build scripts and tooling |

### Rust Toolchain

RamenOS uses a pinned nightly Rust toolchain for bare-metal development. The toolchain is defined in [`rust-toolchain.toml`](../rust-toolchain.toml):

```toml
[toolchain]
channel = "nightly-2026-02-08"
components = ["rust-src", "llvm-tools", "rustfmt", "clippy"]
targets = [
  "x86_64-unknown-none",
  "aarch64-unknown-none",
  "x86_64-unknown-uefi",
  "aarch64-unknown-uefi",
]
```

The toolchain includes:
- `rust-src`: Source code for building bare-metal targets
- `llvm-tools`: Linker tools for bare-metal work
- `rustfmt`: Code formatting
- `clippy`: Linting

### QEMU Installation

RamenOS requires QEMU for both x86_64 and aarch64 architectures:

- **qemu-system-x86_64**: For UEFI boot testing on x86_64
- **qemu-system-aarch64**: For direct kernel boot on aarch64

### OVMF Firmware

OVMF (Open Virtual Machine Firmware) provides UEFI boot support for QEMU x86_64. You need:
- `OVMF_CODE.fd`: UEFI firmware code (read-only)
- `OVMF_VARS.fd`: UEFI variable store (writable copy)

### just Command Runner

The project uses `just` as a command runner (similar to `make` but with simpler syntax). All build commands are defined in the [`justfile`](../justfile).

---

## Development Environment Setup

### macOS Setup (Primary Platform)

#### 1. Install Homebrew Packages

```bash
# Install QEMU for both architectures
brew install qemu

# Install OVMF firmware for UEFI boot
brew install edk2-ovmf

# Install just command runner
brew install just
```

#### 2. Install Rust

```bash
# Install rustup if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# The project will automatically use the pinned toolchain from rust-toolchain.toml
# when you run cargo commands in the project directory
```

#### 3. Verify Installation

```bash
# Check Rust version (should show nightly-2026-02-08 in project directory)
cd RamenOS
cargo --version

# Check QEMU
qemu-system-x86_64 --version
qemu-system-aarch64 --version

# Check just
just --version

# Verify OVMF location
ls /opt/homebrew/share/edk2-ovmf/x64/
# Should show: OVMF_CODE.fd OVMF_VARS.fd
```

### Linux Setup

#### 1. Install Packages (Ubuntu/Debian)

```bash
# Install QEMU
sudo apt-get install qemu-system-x86 qemu-system-arm

# Install OVMF firmware
sudo apt-get install ovmf

# Install just (from release)
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin
# Add ~/bin to your PATH

# Install build essentials
sudo apt-get install build-essential python3 python3-pip
```

#### 2. Install Rust

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 3. Verify Installation

```bash
# Check OVMF location
ls /usr/share/OVMF/
# Should show: OVMF_CODE.fd OVMF_VARS.fd (or OVMF_CODE_4M.fd OVMF_VARS_4M.fd)
```

### Rust Toolchain Configuration

The project uses a `rust-toolchain.toml` file to automatically select the correct toolchain. No manual configuration is needed when working within the project directory.

To verify the toolchain is correct:

```bash
cd RamenOS
rustup show
# Should show: nightly-2026-02-08 (with required components and targets)
```

If components are missing, install them:

```bash
rustup component add rust-src llvm-tools rustfmt clippy
rustup target add x86_64-unknown-none aarch64-unknown-none x86_64-unknown-uefi aarch64-unknown-uefi
```

### Clippy and rustfmt Configuration

The project uses specific formatting settings defined in [`rustfmt.toml`](../rustfmt.toml):

```toml
style_edition = "2024"
```

Run formatting before committing:

```bash
# Format all code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all --check
```

Run clippy for linting:

```bash
# Run baseline clippy (host workspace)
just clippy

# Run strict lint tranches
just clippy-strict
```

---

## Building RamenOS

### Cloning the Repository

```bash
git clone https://github.com/maxwellsantoro/RamenOS.git
cd RamenOS
```

### Building the Kernel

The kernel can be built for multiple targets:

```bash
# Build kernel for x86_64 (bare-metal)
cargo build -p kernel --target x86_64-unknown-none

# Build kernel for aarch64 (bare-metal)
cargo build -p kernel --target aarch64-unknown-none

# Build UEFI kernel for x86_64
cargo build -p kernel_uefi --target x86_64-unknown-uefi

# Build aarch64 kernel (standalone)
cargo build -p kernel_aarch64 --target aarch64-unknown-none --release
```

### Building Services

Host-side services run on the development machine:

```bash
# Build all host-side components
just build-host

# This excludes kernel_uefi and kernel_aarch64 which are target-specific
```

### Running IDL Code Generation

RamenOS uses IDL (Interface Definition Language) files to generate typed contracts:

```bash
# Generate all IDL code
just codegen
```

This generates:
- Rust code in `kernel_api/src/generated/`
- C headers in `tools/capsule/generated/`

### Building All Targets

```bash
# Build all target-specific components
just build-targets
```

### Running Tests

```bash
# Run host workspace tests
cargo test --workspace --exclude kernel_uefi --exclude kernel_aarch64

# Or use the preflight command which includes tests
just preflight
```

---

## Running in QEMU

### x86_64 QEMU Setup

The x86_64 boot uses UEFI firmware (OVMF) and loads the kernel from a FAT disk image.

#### Manual QEMU Command

```bash
# Build the UEFI kernel
cargo build -p kernel_uefi --target x86_64-unknown-uefi

# Create boot directory structure
mkdir -p out/uefi/x86_64/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/kernel_uefi.efi out/uefi/x86_64/EFI/BOOT/BOOTX64.EFI

# Run QEMU (macOS paths)
qemu-system-x86_64 \
  -machine q35 \
  -m 512M \
  -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/edk2-ovmf/x64/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=/opt/homebrew/share/edk2-ovmf/x64/OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:out/uefi/x86_64 \
  -nographic \
  -serial stdio
```

#### Linux Paths

```bash
# Use these paths on Linux
OVMF_CODE=/usr/share/OVMF/OVMF_CODE_4M.fd
OVMF_VARS=/usr/share/OVMF/OVMF_VARS_4M.fd
```

### aarch64 QEMU Setup

The aarch64 boot uses direct kernel loading (no UEFI).

#### Manual QEMU Command

```bash
# Build the aarch64 kernel
cargo build -p kernel_aarch64 --target aarch64-unknown-none --release

# Run QEMU
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a57 \
  -m 512M \
  -kernel target/aarch64-unknown-none/release/kernel_aarch64 \
  -nographic \
  -serial stdio
```

### Expected Output

When RamenOS boots successfully, you should see:

```
RAMEN OS S0 boot
mm: allocator ready
init: hello
init: ping/pong ok
init: ipc badlen small ok
init: ipc badlen large ok
init: ipc unknown proto ok
init: trace ok
```

These messages indicate:
- **RAMEN OS S0 boot**: Kernel entry point reached
- **mm: allocator ready**: Memory management initialized
- **init: hello**: Init component started
- **init: ping/pong ok**: IPC mechanism working
- **init: ipc badlen tests**: IPC error handling working
- **init: trace ok**: Tracing system working

### Using the Foundry Gate

The easiest way to run QEMU is through the Foundry gate:

```bash
# Run the S0 boot gate (builds and tests both architectures)
just foundry-s0
```

---

## Running Foundry Gates

### What Are Foundry Gates?

Foundry gates are automated test scripts that verify specific functionality. They:
- Build the necessary components
- Run QEMU with appropriate configuration
- Assert expected output in logs
- Report pass/fail status

Foundry gates are located in `tools/ci/` and named `foundry_*.sh`.

### Running Individual Gates

```bash
# S0: Boot gate (UEFI + QEMU for both architectures)
just foundry-s0

# S1: Artifact store gate
just foundry-artifact-s1

# S2: Compatibility gate
just foundry-compat-s2

# S3: Trace gate
just foundry-trace-s3

# S4: Store gate
just foundry-store-s4

# S5: POSIX runner gate
just foundry-posix-s5

# S6: Domain manager gate
just foundry-domain-manager-s6
```

### Running All Gates

```bash
# Run S0 through S5 gates
just foundry-all-s0-s1-s2-s3-s4-s5

# Run S0 through S6 gates (full umbrella)
just foundry-all-s0-s1-s2-s3-s4-s5-s6
```

### Running Preflight

The preflight command runs a comprehensive check before pushing:

```bash
just preflight
```

Preflight runs:
1. Format check (`cargo fmt --all --check`)
2. IDL codegen (`just codegen`)
3. Strict lint baseline + tranches
4. Host workspace tests
5. Foundry umbrella gate (S0-S6)

### Interpreting Results

A successful gate run ends with:

```
FOUNDRY_S0: ok
```

If a gate fails:
1. Check the log files in `out/logs/`
2. Look for error messages or missing assertions
3. Verify QEMU and OVMF are correctly installed

---

## Architecture-Specific Notes

### x86_64 Specifics

- **Boot Method**: UEFI via OVMF firmware
- **Machine Type**: Q35 (modern chipset)
- **Memory**: 512MB default
- **Firmware Location**:
  - macOS: `/opt/homebrew/share/edk2-ovmf/x64/`
  - Linux: `/usr/share/OVMF/`
- **Entry Point**: `EFI/BOOT/BOOTX64.EFI` on FAT disk image

#### x86_64 Build Targets

| Target | Purpose |
|--------|---------|
| `x86_64-unknown-none` | Bare-metal kernel (no std) |
| `x86_64-unknown-uefi` | UEFI application kernel |

### aarch64 Specifics

- **Boot Method**: Direct kernel loading (no UEFI)
- **Machine Type**: virt
- **CPU**: Cortex-A57
- **Memory**: 512MB default
- **Entry Point**: Kernel loaded at default address

#### aarch64 Build Targets

| Target | Purpose |
|--------|---------|
| `aarch64-unknown-none` | Bare-metal kernel (no std) |
| `aarch64-unknown-uefi` | UEFI application kernel (future) |

### UEFI Boot Specifics

The UEFI boot process:
1. QEMU loads OVMF firmware
2. OVMF initializes UEFI environment
3. OVMF looks for `EFI/BOOT/BOOTX64.EFI` on the FAT disk
4. The kernel EFI binary is loaded and executed
5. Kernel initializes and starts the init component

#### UEFI Disk Structure

```
out/uefi/x86_64/
  EFI/
    BOOT/
      BOOTX64.EFI    # Kernel UEFI binary
  init.img          # Init component image
```

---

## Common Issues and Troubleshooting

### QEMU Not Found

**Symptom**: `command not found: qemu-system-x86_64`

**Solution**:
```bash
# macOS
brew install qemu

# Linux (Debian/Ubuntu)
sudo apt-get install qemu-system-x86 qemu-system-arm
```

### OVMF Firmware Issues

**Symptom**: `OVMF_CODE not found` or `WARN: OVMF_VARS not found`

**Solution**:

The Foundry gate searches multiple locations. If your OVMF is in a different location, set environment variables:

```bash
# macOS
export OVMF_CODE=/opt/homebrew/share/edk2-ovmf/x64/OVMF_CODE.fd
export OVMF_VARS=/opt/homebrew/share/edk2-ovmf/x64/OVMF_VARS.fd

# Linux
export OVMF_CODE=/usr/share/OVMF/OVMF_CODE_4M.fd
export OVMF_VARS=/usr/share/OVMF/OVMF_VARS_4M.fd
```

### Rust Nightly Version Mismatch

**Symptom**: `error: rustc version mismatch`

**Solution**:

The project pins the toolchain in `rust-toolchain.toml`. Ensure you're running cargo from within the project directory:

```bash
cd RamenOS
rustup show  # Should show nightly-2026-02-08
```

If the toolchain isn't being selected automatically:
```bash
rustup override set nightly-2026-02-08
```

### Linker Errors

**Symptom**: `linker 'rust-lld' not found` or undefined symbol errors

**Solution**:

Ensure you have the required components:
```bash
rustup component add rust-src llvm-tools
```

For bare-metal targets, the project uses `rust-lld` as the linker, which is provided by `llvm-tools`.

### Common Build Failures

#### Missing rust-src

**Symptom**: `can't find crate for std` when building target

**Solution**:
```bash
rustup component add rust-src
rustup target add x86_64-unknown-none aarch64-unknown-none
```

#### Clippy Warnings as Errors

**Symptom**: Build fails with clippy warnings

**Solution**:

The project enforces `-D warnings` for clippy. Fix the warnings or check `docs/LINT_DEBT.md` for allowed exceptions.

For local development with warnings allowed:
```bash
just clippy-baseline-soft
```

#### IDL Codegen Out of Sync

**Symptom**: `use of undeclared type or module` for generated types

**Solution**:
```bash
just codegen
```

### QEMU Boot Hangs

**Symptom**: QEMU starts but no output appears

**Solution**:

1. Check if the kernel was built:
   ```bash
   ls -la target/x86_64-unknown-uefi/debug/kernel_uefi.efi
   ```

2. Check the log file:
   ```bash
   cat out/logs/qemu_x86_64.log
   ```

3. Try running QEMU interactively:
   ```bash
   qemu-system-x86_64 \
     -machine q35 -m 512M \
     -drive if=pflash,format=raw,readonly=on,file=$OVMF_CODE \
     -drive format=raw,file=fat:rw:out/uefi/x86_64 \
     -nographic -serial stdio
   ```

---

## Next Steps

### Where to Learn More

| Document | Purpose |
|----------|---------|
| [`PLATFORM_OVERVIEW.md`](../PLATFORM_OVERVIEW.md) | Architecture and component overview |
| [`ROADMAP.md`](../ROADMAP.md) | Development roadmap and milestones |
| [`SLICES.md`](../SLICES.md) | Vertical slice definitions |
| [`CURRENT_STATUS.md`](../CURRENT_STATUS.md) | Current development status |
| [`CONSTITUTION.md`](../CONSTITUTION.md) | Core design principles |
| [`DECISIONS.md`](../DECISIONS.md) | Design decisions log |

### How to Contribute

See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for:
- Toolchain and formatting requirements
- Lint policy
- Required preflight checks
- Lint debt discipline

### Key Files to Explore

| File | Purpose |
|------|---------|
| [`kernel/src/lib.rs`](../kernel/src/lib.rs) | Kernel core |
| [`kernel/src/init.rs`](../kernel/src/init.rs) | Init component |
| [`kernel/src/ipc_v0.rs`](../kernel/src/ipc_v0.rs) | IPC implementation |
| [`kernel_api/src/lib.rs`](../kernel_api/src/lib.rs) | Kernel API types |
| [`idl/`](../idl/) | Interface definitions |
| [`justfile`](../justfile) | Build commands |

### Development Workflow

1. **Before starting work**: Run `just preflight` to ensure your environment is working
2. **During development**: Run `cargo fmt --all` and `just clippy` frequently
3. **Before pushing**: Run `just preflight` to catch issues early
4. **After pushing**: CI will run the same checks

### Getting Help

- Check [`docs/LINT_DEBT.md`](LINT_DEBT.md) for known lint issues
- Check [`RISKS.md`](../RISKS.md) for known risks and mitigations
- Check [`NEXT_TASKS.md`](../NEXT_TASKS.md) for current priorities
