# DroidBridge: Implementation Roadmap & Acceptance Criteria

This document defines the engineering milestones, deliverables, and objective acceptance criteria for the development of DroidBridge.

---

## Milestone Matrix Overview

| Phase | Milestone | Primary Objective | Target Artifacts | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 1** | **Foundations & Architecture** | Complete feasibility research, toolchain verification, and architectural design. | Technical specs, Architecture docs, Git repository setup. | All 8 initial deliverables produced and verified. |
| **Phase 2** | **Minimal Proof of Concept** | Memory-safe APK parsing, Binary XML decoding, DEX parsing, and clean-room bytecode execution. | `droidbridge_loader`, `droidbridge_core`, `droidbridge_cli`. | CLI parses arbitrary APK, extracts package metadata, and executes synthetic DEX method without VM. |
| **Phase 3** | **Interactive Window & Basic UI** | Display native Win32 window for an Android Activity with lifecycle and Canvas rendering. | Win32 host bridge, Direct3D 11/Direct2D renderer, basic View tree. | Launch test APK, display Activity window, render TextView, handle mouse click. |
| **Phase 4** | **API & Native Library Expansion** | Expand Android framework APIs; load x86_64 ELF `.so` libraries via user-mode thunking. | ELF loader crate, Bionic libc shim, JNI bridge. | Run an APK with x86_64 native libraries; pass 80%+ of core lifecycle test suite. |
| **Phase 5** | **Developer-Ready Platform** | Windows desktop launcher, sandboxed execution, diagnostics, and developer tools. | C# / WPF Launcher, log viewer, crash reporter, installer. | Graphical launcher installs APK, manages app storage, displays live logs. |

---

## Detailed Phase Breakdown

### Phase 1: Feasibility Research & Engineering Foundations
- [x] **Task 1.1:** Inspect development environment and verify toolchains (Rust, MSVC, CMake, Git).
- [x] **Task 1.2:** Produce deep-dive Feasibility Report analyzing ART reuse vs custom runtime vs emulation.
- [x] **Task 1.3:** Author System Architecture and Component Dependency Diagrams.
- [x] **Task 1.4:** Author Licensing, Trademark, and Security Assessment.
- [x] **Task 1.5:** Document Windows x64 Development Setup Guide.
- [x] **Task 1.6:** Define Phased Roadmap with measurable acceptance criteria.
- [x] **Task 1.7:** Initialize Git repository and establish modular workspace structure.
- [x] **Task 1.8:** Implement First Compilable Milestone (`droidbridge_loader` and `droidbridge_core`).

---

### Phase 2: Minimal Proof of Concept
**Focus:** Pure user-mode execution of Android bytecode directly on Windows.

- **Milestone 2.1: APK Ingestion Engine**
  - Implement memory-safe ZIP archive reader with Zip-Slip protection.
  - Implement full Android Binary XML (`AXML`) decoder:
    - String pool table parsing (UTF-8 & UTF-16).
    - Resource map parsing (`RES_XML_RESOURCE_MAP_TYPE`).
    - Element and attribute extraction (Package name, Min/Target SDK, Activities, Permissions).
  - *Acceptance Criterion:* CLI successfully inspects arbitrary real-world APK files and outputs formatted JSON metadata.

- **Milestone 2.2: DEX Parser & Bytecode Disassembler**
  - Parse DEX headers (`magic`, `checksum`, `signature`, table offsets).
  - Parse `class_defs`, `method_ids`, `string_ids`, `type_ids`, and `code_item`.
  - Decode Dalvik instruction streams into structured Rust enums.
  - *Acceptance Criterion:* Disassembles method bytecodes from real-world DEX files with 100% fidelity to standard `dexdump`.

- **Milestone 2.3: Register-Based DEX Virtual Machine**
  - Allocate call frames with typed virtual registers ($v_0 \dots v_N$).
  - Implement core opcodes: `const`, `move`, arithmetic (`add`, `sub`, `mul`, `div`), bitwise, branching (`goto`, `if-eq`, `if-nez`), and method returns.
  - Implement synthetic class loader and method dispatch table.
  - *Acceptance Criterion:* Executes a standalone DEX test method performing arithmetic and branching, returning the correct value to the host without any emulator.

---

### Phase 3: Interactive Window & Basic UI
**Focus:** Connecting the DEX virtual machine to a native Windows desktop window.

- **Milestone 3.1: Activity Lifecycle & Host Window Bridge**
  - Implement `android.app.Activity` state machine (`onCreate`, `onStart`, `onResume`, `onPause`, `onDestroy`).
  - Register Win32 window class and create top-level `HWND` for the main Activity.
  - Map Win32 messages (`WM_CLOSE`, `WM_ACTIVATE`) to Android lifecycle events.
  - *Acceptance Criterion:* Launching the APK opens a native Windows desktop window displaying the application's title.

- **Milestone 3.2: View Hierarchy & Canvas Rendering**
  - Implement core View classes: `View`, `ViewGroup`, `TextView`, `Button`, `LinearLayout`.
  - Implement two-pass layout: `measure(widthSpec, heightSpec)` and `layout(l, t, r, b)`.
  - Implement Direct2D / Direct3D 11 hardware-accelerated canvas rasterizer.
  - *Acceptance Criterion:* A test application with a `TextView` and `Button` renders sharp text and interactive widgets inside the Win32 window.

- **Milestone 3.3: Input Dispatch Pipeline**
  - Translate Win32 mouse events (`WM_LBUTTONDOWN`, `WM_MOUSEMOVE`, `WM_LBUTTONUP`) to `android.view.MotionEvent`.
  - Dispatch events down the View hierarchy with hit-testing.
  - Handle `View.OnClickListener` callbacks inside the DEX engine.
  - *Acceptance Criterion:* Clicking the rendered button triggers the application's Kotlin/Java click listener and updates UI text.

---

### Phase 4: Compatibility Expansion & Native Libraries
**Focus:** Broader framework coverage and loading Linux x86_64 ELF libraries on Windows.

- **Milestone 4.1: Framework API Expansion**
  - Implement `android.content.SharedPreferences` backed by isolated JSON storage in `%LOCALAPPDATA%`.
  - Implement `android.content.Intent` explicit resolution.
  - Implement basic `android.os.Handler` and `android.os.Looper` message queues.
  - *Acceptance Criterion:* Applications can persist state across restarts and dispatch delayed tasks on the UI thread.

- **Milestone 4.2: User-Mode x86_64 ELF Loader & Bionic Shim**
  - Parse ELF headers, load segments with `VirtualAlloc`, apply relocations (`R_X86_64_RELATIVE`, `R_X86_64_GLOB_DAT`).
  - Implement ABI thunking between System V AMD64 and Microsoft x64 calling conventions.
  - Provide Bionic libc shim DLL exporting `__android_log_print`, `malloc`, `free`, and standard POSIX memory/string APIs.
  - Implement JNI `JNIEnv*` dispatch table for native methods (`Java_*`).
  - *Acceptance Criterion:* Loads an x86_64 `.so` library, binds a native JNI method, and executes it from DEX bytecode.

---

### Phase 5: Developer-Ready Platform & Tooling
**Focus:** Usability, sandboxing, diagnostics, and deployment.

- **Milestone 5.1: Windows Launcher & Diagnostic Console**
  - Modern Windows desktop launcher (WPF / .NET or native WinUI 3).
  - Drag-and-drop APK installation.
  - Live Logcat console capturing `android.util.Log` output.
  - Real-time compatibility warning inspector (flags unsupported APIs and missing native ABIs).
  - *Acceptance Criterion:* End-user can drag an APK onto the launcher, view its metadata, launch it, and monitor live runtime logs.

- **Milestone 5.2: Security Sandboxing & Distribution**
  - Windows AppContainer isolation for worker execution.
  - Scoped storage isolation with file access quotas.
  - Automated installer (MSI / Inno Setup) bundling precompiled binaries.
  - *Acceptance Criterion:* Passing comprehensive security test verifying that sandboxed APK cannot read files outside its designated data folder.
