use anyhow::Result;
#[cfg(windows)]
use anyhow::anyhow;


#[cfg(windows)]
pub fn encrypt_bytes(data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let success = unsafe {
        CryptProtectData(
            &mut input_blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output_blob,
        )
    };

    if !success.as_bool() {
        return Err(anyhow!("Windows DPAPI 加密失败: {:?}", std::io::Error::last_os_error()));
    }

    let encrypted = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };

    unsafe {
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output_blob.pbData as _));
    }

    Ok(encrypted)
}

#[cfg(windows)]
pub fn decrypt_bytes(encrypted_data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: encrypted_data.len() as u32,
        pbData: encrypted_data.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let success = unsafe {
        CryptUnprotectData(
            &mut input_blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output_blob,
        )
    };

    if !success.as_bool() {
        return Err(anyhow!("Windows DPAPI 解密失败 (可能已跨机迁移或凭据损坏): {:?}", std::io::Error::last_os_error()));
    }

    let decrypted = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };

    unsafe {
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output_blob.pbData as _));
    }

    Ok(decrypted)
}

#[cfg(not(windows))]
pub fn encrypt_bytes(data: &[u8]) -> Result<Vec<u8>> {
    // 非 Windows 平台回退实现（主要用于跨平台单元测试）
    Ok(data.iter().map(|b| b ^ 0x5A).collect())
}

#[cfg(not(windows))]
pub fn decrypt_bytes(encrypted_data: &[u8]) -> Result<Vec<u8>> {
    Ok(encrypted_data.iter().map(|b| b ^ 0x5A).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpapi_roundtrip() -> Result<()> {
        let raw_secret = b"MyStrongPassword!#@123";
        let encrypted = encrypt_bytes(raw_secret)?;
        assert_ne!(raw_secret.as_slice(), encrypted.as_slice());

        let decrypted = decrypt_bytes(&encrypted)?;
        assert_eq!(raw_secret.as_slice(), decrypted.as_slice());
        Ok(())
    }
}
