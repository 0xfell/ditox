# Task 004: Windows Installer & Distribution

**Status**: `in-progress`
**Started**: 2025-12-07
**Completed**: -

## Overview

Implement a complete Windows distribution pipeline for `ditox-gui`:
1. ✅ MSI installer with optional "Run on startup" checkbox
2. ✅ Startup-on-boot registration via Windows Registry
3. ⏳ Code signing for trusted distribution (requires purchase)

---

## What Was Implemented

### ✅ Phase 1: Prerequisites & Assets (DONE)

1. **Windows Icon (.ico)** - Auto-generated from `ditox.png` via `build.rs`
   - Multi-resolution: 256x256, 48x48, 32x32, 16x16
   - Generated at: `ditox-gui/ditox.ico`

2. **Windows Resource Manifest** - Embeds icon + version info into `.exe`
   - File: `ditox-gui/build.rs`
   - Dependencies: `winres`, `image`

### ✅ Phase 2: Startup-on-Boot Feature (DONE)

1. **auto-launch crate** - Cross-platform startup registration
   - File: `ditox-gui/src/startup.rs`
   - Registry key: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`

2. **Tray Menu Toggle** - "Run on Startup" checkbox in system tray
   - File: `ditox-gui/src/app.rs` (updated `setup_tray_icon`)

### ✅ Phase 3: WiX Installer Setup (DONE)

1. **cargo-wix installed** globally
2. **WiX configuration created:**
   - `ditox-gui/wix/main.wxs` - Custom installer config
   - `ditox-gui/wix/License.rtf` - MIT license
   - `ditox-gui/wix/ditox.ico` - Icon for installer

**Installer Features:**
- Installs to `%LocalAppData%\Ditox` (no admin required)
- Creates Start Menu shortcut
- Optional Desktop shortcut
- Optional "Run on Startup" (checked by default)
- Shows in Add/Remove Programs with icon

---

## Remaining Steps (Manual)

### Install WiX Toolset

WiX is required to build the `.msi` installer. Choose one method:

```powershell
# Option 1: Via .NET tool (if dotnet is installed)
dotnet tool install --global wix

# Option 2: Via Chocolatey
choco install wixtoolset

# Option 3: Manual download from https://wixtoolset.org/
```

### Build the Installer

```powershell
# Build release first
cargo build --release -p ditox-gui

# Build MSI (after WiX is installed)
cargo wix -p ditox-gui
```

Output: `target/wix/ditox-gui-{version}-x86_64.msi`

### Code Signing (Optional)

See "Phase 4" section below for signing options.

---

## Original Plan (Reference)

### 1.1 Create Windows Icon (.ico)

The WiX installer and Windows executable require an `.ico` file (not just `.png`).

**Files to create:**
- `ditox-gui/ditox.ico` - Multi-resolution icon (16x16, 32x32, 48x48, 256x256)

**Tools:**
- Use ImageMagick: `magick ditox.png -define icon:auto-resize=256,48,32,16 ditox.ico`
- Or use an online converter like [icoconvert.com](https://icoconvert.com/)

### 1.2 Add Windows Resource Manifest

Embed icon and version info into the `.exe`.

**File:** `ditox-gui/build.rs`
```rust
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("ditox.ico");
        res.set("ProductName", "Ditox Clipboard Manager");
        res.set("FileDescription", "Ditox Clipboard Manager");
        res.set("LegalCopyright", "Copyright (c) 2024");
        res.compile().unwrap();
    }
}
```

**Cargo.toml change:**
```toml
[build-dependencies]
winres = "0.1"  # Uncomment existing line
```

---

## Phase 2: Startup-on-Boot Feature

### 2.1 Add `auto-launch` Dependency

**File:** `ditox-gui/Cargo.toml`
```toml
[target.'cfg(windows)'.dependencies]
auto-launch = "0.5"
```

### 2.2 Add Startup Toggle to Config

**File:** `ditox-core/src/config.rs` - Add field:
```rust
pub struct General {
    // existing fields...
    #[serde(default)]
    pub start_on_boot: bool,
}
```

### 2.3 Implement Startup Registration

**File:** `ditox-gui/src/startup.rs` (new file)
```rust
use auto_launch::AutoLaunchBuilder;

pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?;

    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name("Ditox")
        .set_app_path(&exe_path.to_string_lossy())
        .set_use_launch_agent(false)  // Use registry on Windows
        .build()
        .map_err(|e| e.to_string())?;

    if enabled {
        auto_launch.enable().map_err(|e| e.to_string())
    } else {
        auto_launch.disable().map_err(|e| e.to_string())
    }
}

pub fn is_startup_enabled() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    AutoLaunchBuilder::new()
        .set_app_name("Ditox")
        .set_app_path(&exe_path.to_string_lossy())
        .build()
        .map(|al| al.is_enabled().unwrap_or(false))
        .unwrap_or(false)
}
```

### 2.4 Add Settings UI (Optional)

Add a context menu item in the system tray or a settings dialog to toggle startup behavior.

---

## Phase 3: WiX Installer Setup

### 3.1 Install Prerequisites

1. **Install WiX Toolset** (one of):
   ```powershell
   # Via .NET tool (recommended)
   dotnet tool install --global wix

   # Or via Chocolatey
   choco install wixtoolset
   ```

2. **Install cargo-wix:**
   ```powershell
   cargo install cargo-wix
   ```

### 3.2 Initialize WiX for Project

```powershell
cd ditox-gui
cargo wix init
```

This creates `wix/main.wxs` - the installer definition.

### 3.3 Customize Installer (wix/main.wxs)

Key customizations needed:

```xml
<?xml version='1.0' encoding='windows-1252'?>
<Wix xmlns='http://schemas.microsoft.com/wix/2006/wi'>
    <Product
        Id='*'
        Name='Ditox Clipboard Manager'
        UpgradeCode='GENERATE-NEW-GUID-HERE'
        Manufacturer='Ditox'
        Language='1033'
        Version='$(var.Version)'>

        <Package
            Keywords='Installer'
            Description='Ditox Clipboard Manager Installer'
            Manufacturer='Ditox'
            InstallerVersion='450'
            Compressed='yes'
            InstallScope='perUser'/>  <!-- perUser = no admin required -->

        <!-- ... existing components ... -->

        <!-- Startup checkbox feature -->
        <Feature Id='StartupFeature' Title='Run on Startup' Level='1000'>
            <ComponentRef Id='StartupShortcut'/>
        </Feature>

        <!-- Registry entry for startup -->
        <DirectoryRef Id='TARGETDIR'>
            <Component Id='StartupShortcut' Guid='GENERATE-GUID'>
                <RegistryValue
                    Root='HKCU'
                    Key='Software\Microsoft\Windows\CurrentVersion\Run'
                    Name='Ditox'
                    Type='string'
                    Value='"[INSTALLDIR]bin\ditox-gui.exe"'
                    KeyPath='yes'/>
            </Component>
        </DirectoryRef>

        <!-- Custom UI for startup checkbox -->
        <UI>
            <UIRef Id='WixUI_InstallDir'/>
            <Property Id='WIXUI_INSTALLDIR' Value='INSTALLDIR'/>
        </UI>

    </Product>
</Wix>
```

### 3.4 Add License File

WiX requires a license file for the installer:

**File:** `ditox-gui/wix/License.rtf`
- Convert LICENSE (MIT) to RTF format
- Or create a simple RTF with license text

### 3.5 Build the Installer

```powershell
# Build release binary first
cargo build --release -p ditox-gui

# Build MSI installer
cargo wix -p ditox-gui
```

Output: `target/wix/ditox-gui-{version}-x86_64.msi`

---

## Phase 4: Code Signing

### Option A: Microsoft Trusted Signing (Recommended)

**Cost:** $9.99/month (Basic) - 5,000 signatures included

**Pros:**
- Immediate SmartScreen reputation (no warning dialogs)
- No hardware token required
- Cloud-based, works in CI/CD
- Cheaper than traditional EV certificates

**Setup Steps:**

1. **Create Azure Account** at [azure.microsoft.com](https://azure.microsoft.com)

2. **Create Trusted Signing Resource:**
   ```
   Azure Portal → Create Resource → "Trusted Signing"
   ```

3. **Identity Verification:**
   - Individual: Government ID + address verification
   - Takes 1-3 business days

4. **Create Certificate Profile:**
   - Choose "Public Trust" for public distribution
   - Download signing credentials

5. **Install SignTool (Windows SDK):**
   ```powershell
   winget install Microsoft.WindowsSDK
   ```

6. **Sign the Installer:**
   ```powershell
   # Sign MSI
   signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 ^
       /azure-key-vault-url "YOUR_VAULT_URL" ^
       target/wix/ditox-gui-*.msi

   # Sign EXE
   signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 ^
       /azure-key-vault-url "YOUR_VAULT_URL" ^
       target/release/ditox-gui.exe
   ```

### Option B: Traditional Code Signing Certificate

**Cost:** $200-400/year (Individual/OV certificate)

**Providers:**
- [Sectigo](https://sectigo.com) - ~$215/year
- [DigiCert](https://digicert.com) - ~$400/year
- [SSL.com](https://ssl.com) - ~$250/year

**Cons:**
- Requires USB hardware token (shipped physically)
- SmartScreen reputation builds over time (users see warnings initially)
- Can't easily use in CI/CD

**Signing Command:**
```powershell
signtool sign /f certificate.pfx /p PASSWORD /fd SHA256 ^
    /tr http://timestamp.digicert.com /td SHA256 ^
    target/wix/ditox-gui-*.msi
```

### Option C: Self-Signed (Development Only)

**Cost:** Free

**For testing only** - Windows will show "Unknown Publisher" warnings.

```powershell
# Create self-signed certificate
$cert = New-SelfSignedCertificate -Type CodeSigning -Subject "CN=Ditox Dev" -CertStoreLocation Cert:\CurrentUser\My

# Export to PFX
$pwd = ConvertTo-SecureString -String "password" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath ditox-dev.pfx -Password $pwd

# Sign
signtool sign /f ditox-dev.pfx /p password /fd SHA256 target/wix/*.msi
```

---

## Phase 5: GitHub Actions CI/CD (Optional)

### 5.1 Create Workflow File

**File:** `.github/workflows/release-windows.yml`

```yaml
name: Release Windows

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: x86_64-pc-windows-msvc

      - name: Install WiX
        run: dotnet tool install --global wix

      - name: Install cargo-wix
        run: cargo install cargo-wix

      - name: Build Release
        run: cargo build --release -p ditox-gui

      - name: Build Installer
        run: cargo wix -p ditox-gui --no-build

      # Signing step (requires secrets)
      - name: Sign Installer
        if: ${{ secrets.AZURE_TENANT_ID }}
        run: |
          signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 `
            /azure-key-vault-url "${{ secrets.AZURE_VAULT_URL }}" `
            target/wix/*.msi

      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ditox-windows-installer
          path: target/wix/*.msi

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: target/wix/*.msi
```

---

## Summary: Recommended Path

| Component | Recommendation | Cost |
|-----------|---------------|------|
| Installer | cargo-wix + WiX Toolset | Free |
| Startup | `auto-launch` crate | Free |
| Code Signing | Microsoft Trusted Signing | $9.99/mo |
| CI/CD | GitHub Actions | Free (public repos) |

**Total minimum cost:** ~$120/year (for signing)

**Without signing:** Free, but users will see SmartScreen warnings

---

## Implementation Order

1. **Create .ico file** from existing ditox.png
2. **Add winres build script** for exe metadata
3. **Add auto-launch** dependency and startup code
4. **Install WiX + cargo-wix** locally
5. **Run `cargo wix init`** and customize main.wxs
6. **Build and test installer** locally
7. **(Optional)** Set up Azure Trusted Signing
8. **(Optional)** Add GitHub Actions workflow

---

## Work Log

- 2025-12-07: Created implementation plan

## References

- [cargo-wix GitHub](https://github.com/volks73/cargo-wix)
- [WiX Toolset](https://wixtoolset.org/)
- [auto-launch crate](https://crates.io/crates/auto-launch)
- [Microsoft Trusted Signing](https://azure.microsoft.com/en-us/products/trusted-signing)
- [Azure Trusted Signing Pricing](https://azure.microsoft.com/en-us/pricing/details/trusted-signing/)
