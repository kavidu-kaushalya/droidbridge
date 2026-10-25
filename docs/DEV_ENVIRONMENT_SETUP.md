# DroidBridge: Windows x64 Development Environment Setup

This guide details the setup, configuration, and verification of the development toolchain required to build, test, and debug DroidBridge natively on Windows 10/11 (x64).

---

## 1. Prerequisites & Verified Toolchain

DroidBridge is built using a modern dual-language architecture: **Rust** for the core memory-safe runtime, parsers, and DEX execution engine, and **C++20 / MSVC** for native Win32 windowing, Direct3D 11 swap chains, and platform integration.

| Tool | Verified Version | Path / Location | Purpose |
| :--- | :--- | :--- | :--- |
| **Rust Toolchain** | `1.99.1` (`x86_64-pc-windows-msvc`) | `C:\Users\<user>\.cargo\bin\rustc.exe` | Core runtime, parser, VM engine |
| **Cargo** | `1.99.1` | `C:\Users\<user>\.cargo\bin\cargo.exe` | Rust package manager & test runner |
| **Visual Studio** | 2022 / 2026 Build Tools | `G:\Visual Studio\Product` | C++20 compiler, Linker, Windows SDK |
| **MSVC Compiler** | `14.51.36231` | `G:\Visual Studio\Product\VC\Tools\MSVC\` | Native C++20 compilation |
| **CMake** | `4.3.1-msvc1` | `G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\` | Build orchestration |
| **Ninja** | `1.11+` | `G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\` | Fast native build driver |
| **Git** | `2.55.0.windows.5` | `G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\TeamFoundation\Team Explorer\Git\cmd\` | Source control |
| **.NET SDK** | .NET 8.0 / 9.0 | `C:\Program Files\dotnet\dotnet.exe` | Windows management UI / Launcher |

---

## 2. Environment Configuration

To ensure seamless compilation across PowerShell and Command Prompt sessions, configure your environment variables or run the Visual Studio Developer Command Prompt.

### 2.1 Setting Up Environment Variables (PowerShell)
To add MSVC, CMake, Git, and Cargo to your current session PATH:

```powershell
# Add Git and CMake to PATH
$env:PATH += ";G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\TeamFoundation\Team Explorer\Git\cmd"
$env:PATH += ";G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
$env:PATH += ";G:\Visual Studio\Product\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"

# Verify toolchain availability
git --version
cmake --version
rustc -vV
cargo --version
```

### 2.2 Using Visual Studio Developer Environment
For building the C++ components with MSVC:

```cmd
call "G:\Visual Studio\Product\VC\Auxiliary\Build\vcvars64.bat"
```

---

## 3. Project Directory Structure

```
droidbridge/
├── Cargo.toml                    # Root Rust workspace definition
├── CMakeLists.txt                # Root CMake project for native C++ & bindings
├── docs/                         # Technical specifications and architecture
│   ├── FEASIBILITY_REPORT.md
│   ├── ARCHITECTURE.md
│   ├── LICENSING_AND_SECURITY.md
│   ├── DEV_ENVIRONMENT_SETUP.md
│   └── ROADMAP.md
├── crates/                       # Rust crates
│   ├── droidbridge_loader/       # APK, AXML, ARSC, DEX parser & validator
│   ├── droidbridge_core/         # DEX virtual machine, opcode interpreter, heap
│   ├── droidbridge_framework/    # Android framework stubs & lifecycle manager
│   └── droidbridge_cli/          # Command-line interface & inspection tool
├── native/                       # Native C++20 Win32/Direct3D components
│   ├── include/                  # C++ API headers
│   └── src/                      # HWND management, D3D11 swap chain
└── tests/                        # Integration and compatibility test suites
    └── fixtures/                 # Test APKs and DEX files
```

---

## 4. Build and Verification Workflow

### 4.1 Compiling the Rust Core
```powershell
Set-Location "C:\Users\nactro\.gemini\antigravity\scratch\droidbridge"
cargo build --workspace
```

### 4.2 Running the Test Suite
```powershell
cargo test --workspace -- --nocapture
```

### 4.3 Inspecting an APK
```powershell
cargo run -p droidbridge_cli -- inspect "path\to\application.apk"
```

### 4.4 Running a Test Bytecode Method
```powershell
cargo run -p droidbridge_cli -- exec-test
```
