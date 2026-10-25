# DroidBridge: Licensing, Trademarks, and Security Architecture

## 1. Legal and Licensing Assessment

### 1.1 Open Source Licensing Compliance
- **Android Open Source Project (AOSP):** The vast majority of user-space Android components, including the Android Runtime (ART), the Android Framework, and Bionic libc, are licensed under the **Apache License, Version 2.0**.
  - Permissive license: Allows redistribution, modification, and incorporation into commercial or proprietary projects without viral copyleft obligations.
  - Requires preservation of copyright notices, Apache 2.0 license text, and statement of significant modifications.
- **Clean-Room Implementation Strategy:**
  - DroidBridge implements its core parser, virtual machine, and Win32 host integration as an independent, clean-room codebase written from scratch in Rust and C++20.
  - Clean-room design avoids copyright disputes and ensures zero accidental inclusion of GPLv2-licensed Linux kernel code.
- **Third-Party Dependencies:**
  - Standard Rust crates (`zip`, `byteorder`, `thiserror`, `windows-sys` / `windows`) are permissively licensed (MIT / Apache-2.0).

### 1.2 Trademark & Brand Usage
- **"Android" Trademark:** "Android" is a registered trademark of Google LLC.
- **Compliance Rules:**
  - DroidBridge must clearly state that it is an independent project and is neither endorsed by, affiliated with, nor certified by Google LLC.
  - The software must not claim "Android Compatibility Certification" (which requires passing the official Android Compatibility Test Suite - CTS and obtaining a Google license).
  - Marketing, documentation, and user interfaces must state: *"DroidBridge is an independent Windows runtime for running Android-formatted applications. Android is a trademark of Google LLC."*

---

## 2. Threat Modeling and Security Architecture

Treating arbitrary APK files as untrusted input is a paramount requirement. Running an untrusted Android application directly on a Windows host without a hypervisor introduces security vectors that must be actively mitigated.

### 2.1 Threat Vectors & Mitigation Matrix

```
+---------------------------------------------------------------------------------+
|                               Untrusted APK Input                               |
+---------------------------------------------------------------------------------+
        |                                 |                                 |
        v                                 v                                 v
[Malicious Archive/AXML]        [Bytecode Exploits]             [Host System Compromise]
- Path Traversal (Zip Slip)     - Type confusion                - Arbitrary filesystem read
- Decompression Bombs           - Out-of-bounds register read   - Arbitrary registry write
- Malformed String Pools        - Stack overflow                - Unauthorized network use
        |                                 |                                 |
        v                                 v                                 v
+-----------------------+       +-----------------------+       +-----------------------+
|  Memory-Safe Parsers  |       |   Rigid DEX Verifier  |       |  Windows AppContainer |
|  - Rust strict bounds |       |   - Bounds validation |       |  - Low Integrity token|
|  - Zip-Slip rejection |       |   - Typed values      |       |  - Scoped storage map |
|  - Allocation quotas  |       |   - Gas / step limits |       |  - Blocked raw syscall|
+-----------------------+       +-----------------------+       +-----------------------+
```

### 2.2 Ingestion-Phase Hardening
1. **Archive Path Traversal ("Zip Slip"):**
   - When extracting or reading files from an APK archive, relative paths containing `..` or leading slashes (`/` or `\`) are strictly rejected.
   - Output paths are canonicalized and verified to reside strictly within the allocated target directory.
2. **Decompression Bomb Protection:**
   - File extraction enforces a maximum uncompressed size limit (e.g. 500 MB maximum cumulative size per APK) and a maximum compression ratio (e.g. 100:1).
3. **Integer Overflow and Bounds Validation:**
   - All DEX offsets, table sizes, string counts, and method bytecode lengths are strictly checked against the total DEX file buffer size before any slicing occurs.
   - Implemented in Rust with debug assertions and safe arithmetic (`checked_add`, `checked_mul`).

### 2.3 Runtime Execution Hardening (DEX Engine)
1. **Virtual Register Safety:**
   - Virtual registers are strictly indexed within the method's declared `registers_size`. Any bytecode instruction referencing an index $\ge \text{registers\_size}$ triggers a `VerifyError` and immediate termination of the offending thread.
2. **Type Enforcement:**
   - Registers are strongly typed at runtime (`Integer`, `Long`, `Float`, `Double`, `Reference`, `Null`). Arithmetic instructions on invalid register types are caught before evaluation.
3. **Execution Quotas (Gas / Step Limits):**
   - During evaluation of untrusted or diagnostic test apps, execution steps can be metered to prevent infinite loops or denial-of-service hangs on the main UI thread.

### 2.4 Host Isolation & Windows Sandboxing
1. **Virtual Scoped Storage:**
   - The application does not receive access to `%USERPROFILE%`, `C:\Windows`, or arbitrary drive roots.
   - All Android file APIs (`Context.getFilesDir()`, `Context.getCacheDir()`, `Environment.getExternalStorageDirectory()`) are virtualized to:
     `%LOCALAPPDATA%\DroidBridge\Apps\<package_name>\files`
     `%LOCALAPPDATA%\DroidBridge\Apps\<package_name>\cache`
     `%LOCALAPPDATA%\DroidBridge\Apps\<package_name>\storage`
2. **Windows AppContainer / Low Integrity Execution:**
   - DroidBridge application worker processes run with a restricted Windows Token (`SECURITY_MANDATORY_LOW_RID` or `AppContainer`).
   - AppContainers restrict network capabilities unless `android.permission.INTERNET` is explicitly granted by the user or manifest.
   - Direct access to sensitive Windows APIs (e.g. `OpenProcess`, raw sockets, registry modifications outside HKCU AppContainer key) is blocked by the Windows NT kernel security monitor.
