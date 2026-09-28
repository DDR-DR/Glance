//! Protects the LLM API key at rest.
//!
//! On Windows the value is protected with the DPAPI
//! (`CryptProtectData`/`CryptUnprotectData`), so it is only readable by the
//! current Windows user account. Encrypted values are stored as
//! `enc:v1:<base64>`. Values without the prefix are treated as legacy
//! plaintext and returned unchanged (they get encrypted on the next save).
//! On macOS the secret is stored in the user's login Keychain and the settings
//! file only contains the `keychain:v1` marker. Linux keeps the legacy
//! plaintext behavior until a platform secret store is added.

#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_MARKER: &str = "keychain:v1";
#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_SERVICE: &str = "com.harukaon.glance.llm-api-key";
#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_ACCOUNT: &str = "default";

/// Protect a plaintext secret for storage. Windows uses DPAPI and macOS uses
/// the login Keychain. If the platform store is unavailable, the value stays
/// in the legacy plaintext format so saving settings does not destroy it.
pub fn encrypt(value: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        if value.is_empty() {
            delete_macos_keychain();
            return String::new();
        }
        if store_macos_keychain(value) {
            return MACOS_KEYCHAIN_MARKER.to_string();
        }
        tracing::warn!("macOS Keychain write failed; keeping legacy settings storage");
    }

    #[cfg(target_os = "windows")]
    {
        if !value.is_empty() {
            if let Some(enc) = encrypt_windows(value) {
                return format!("enc:v1:{enc}");
            }
        }
    }
    value.to_string()
}

/// Load a stored secret. Values without a platform marker/prefix are returned
/// as-is for backward compatibility with legacy plaintext settings.
pub fn decrypt(stored: &str) -> String {
    #[cfg(target_os = "macos")]
    if stored == MACOS_KEYCHAIN_MARKER {
        return load_macos_keychain().unwrap_or_else(|| {
            tracing::warn!("macOS Keychain item is unavailable");
            String::new()
        });
    }

    if stored.starts_with("enc:v1:") {
        #[cfg(target_os = "windows")]
        {
            let b64 = stored.trim_start_matches("enc:v1:");
            if let Some(plain) = decrypt_windows(b64) {
                return plain;
            }
        }
        // Encrypted value but no decryptor available: return as-is rather
        // than corrupting the secret.
        return stored.to_string();
    }
    stored.to_string()
}

#[cfg(target_os = "macos")]
fn store_macos_keychain(value: &str) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-U",
            "-a",
            MACOS_KEYCHAIN_ACCOUNT,
            "-s",
            MACOS_KEYCHAIN_SERVICE,
            "-w",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    // `security ... -w` prompts twice when creating and accepts the same input
    // when updating. Stdin keeps the secret out of the process argument list.
    if let Some(mut stdin) = child.stdin.take() {
        if writeln!(stdin, "{value}").is_err() || writeln!(stdin, "{value}").is_err() {
            let _ = child.kill();
            return false;
        }
    }
    child.wait().map(|status| status.success()).unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn load_macos_keychain() -> Option<String> {
    let output = std::process::Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-a",
            MACOS_KEYCHAIN_ACCOUNT,
            "-s",
            MACOS_KEYCHAIN_SERVICE,
            "-w",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim_end_matches(&['\r', '\n'][..]).to_string())
}

#[cfg(target_os = "macos")]
fn delete_macos_keychain() {
    let _ = std::process::Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-a",
            MACOS_KEYCHAIN_ACCOUNT,
            "-s",
            MACOS_KEYCHAIN_SERVICE,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(target_os = "windows")]
fn encrypt_windows(plain: &str) -> Option<String> {
    use base64::Engine;
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let bytes = plain.as_bytes();
    let mut in_blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let ok = unsafe {
        CryptProtectData(
            &mut in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return None;
    }
    let data = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    unsafe {
        let _ = LocalFree(out_blob.pbData as HLOCAL);
    }
    Some(encoded)
}

#[cfg(target_os = "windows")]
fn decrypt_windows(b64: &str) -> Option<String> {
    use base64::Engine;
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let mut in_blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &mut in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return None;
    }
    let data = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let text = String::from_utf8(data.to_vec()).ok();
    unsafe {
        let _ = LocalFree(out_blob.pbData as HLOCAL);
    }
    text
}
