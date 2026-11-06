#[cfg(test)]
mod tests {
    use std::path::Path;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root exists")
            .to_path_buf()
    }

    #[test]
    fn cargo_release_profile_av_safe() {
        let cargo_toml =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
                .expect("read Cargo.toml");

        // Windows Defender ML heuristics flag stripped, panic-abort, opt-level=z Rust binaries
        // as generic Trojan:Win32/Wacatac.B!ml.
        assert!(
            !cargo_toml.contains("strip = true"),
            "CRITICAL: Cargo.toml must NOT enable 'strip = true'. This triggers Windows Defender ML false positives."
        );
        assert!(
            !cargo_toml.contains("panic = \"abort\""),
            "CRITICAL: Cargo.toml must NOT enable 'panic = \"abort\"'. Unwind tables are required for safe heuristic analysis."
        );
        assert!(
            !cargo_toml.contains("opt-level = \"z\""),
            "CRITICAL: Cargo.toml must NOT use 'opt-level = \"z\"'. Section compression triggers antivirus packer heuristics."
        );

        let cargo_config = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(".cargo/config.toml"),
        )
        .expect("read .cargo/config.toml");

        assert!(
            !cargo_config.contains("/OPT:ICF"),
            "CRITICAL: Linker flag /OPT:ICF alters PE section alignment, triggering heuristics."
        );
        assert!(
            !cargo_config.contains("+crt-static"),
            "CRITICAL: .cargo/config.toml must NOT use '+crt-static'. Dynamic CRT avoids Bitdefender Gen.Variant.Fugrafa false positives."
        );
        assert!(
            cargo_config.contains("/RELEASE"),
            "CRITICAL: .cargo/config.toml must specify '/RELEASE' linker flag to populate PE checksum."
        );

        let hooks_rs =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hooks.rs"))
                .expect("read src/hooks.rs");
        for banned in [
            "SetWindowsHookExW",
            "UnhookWindowsHookEx",
            "CallNextHookEx",
            "WH_KEYBOARD_LL",
            "WH_MOUSE_LL",
            "keybd_event",
            "GetAsyncKeyState",
        ] {
            assert!(
                !hooks_rs.contains(banned),
                "CRITICAL: hooks.rs must NOT use '{banned}' (triggers Bitdefender Gen.Variant.Fugrafa keylogger heuristic)."
            );
        }

        let native_rs =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/native.rs"))
                .expect("read src/native.rs");
        assert!(
            !native_rs.contains("GetAsyncKeyState"),
            "CRITICAL: native.rs must use GetKeyState instead of GetAsyncKeyState."
        );

        let rc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("glazebar.rc"))
            .expect("read glazebar.rc");
        assert!(
            rc.contains("glazebar.manifest"),
            "glazebar.rc must embed glazebar.manifest."
        );
        let manifest = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("glazebar.manifest"),
        )
        .expect("read glazebar.manifest");
        assert!(
            manifest.contains("asInvoker"),
            "glazebar.manifest must specify asInvoker."
        );
    }

    #[test]
    fn installer_scripts_av_heuristics_safe() {
        let root = repo_root();

        // 1. Install.ps1
        let install_ps1 = std::fs::read_to_string(root.join("distribution/Install.ps1"))
            .expect("read Install.ps1");
        assert!(
            !install_ps1.contains("-ExecutionPolicy Bypass"),
            "Install.ps1 must not use '-ExecutionPolicy Bypass' (known AV heuristic trigger)."
        );
        assert!(
            !install_ps1.contains("Invoke-WebRequest")
                || !install_ps1.contains("standalone-glazewm"),
            "Install.ps1 must NOT download GlazeWM MSI dynamically via Invoke-WebRequest at install time (causes AMSI TrojanDownloader heuristic block)."
        );
        assert!(
            install_ps1.contains("$manifest.glazewmMsi"),
            "Install.ps1 must use local bundled $manifest.glazewmMsi."
        );

        // 2. GlazeBar-Setup.iss
        let setup_iss = std::fs::read_to_string(root.join("distribution/GlazeBar-Setup.iss"))
            .expect("read GlazeBar-Setup.iss");
        assert!(
            setup_iss.contains("-ExecutionPolicy RemoteSigned"),
            "GlazeBar-Setup.iss must use '-ExecutionPolicy RemoteSigned' instead of Bypass."
        );
        assert!(
            !setup_iss.contains("-ExecutionPolicy Bypass"),
            "GlazeBar-Setup.iss must not contain '-ExecutionPolicy Bypass'."
        );
        assert!(
            !setup_iss.contains("LZMAUseSeparateProcess=yes"),
            "GlazeBar-Setup.iss must not use separate process LZMA unpacking."
        );
        assert!(
            !setup_iss.contains("LZMANumBlockThreads=1"),
            "GlazeBar-Setup.iss must not restrict LZMA block threads."
        );

        // 3. Setup-Bridge.ps1
        let bridge_ps1 = std::fs::read_to_string(root.join("distribution/Setup-Bridge.ps1"))
            .expect("read Setup-Bridge.ps1");
        assert!(
            !bridge_ps1.contains("-ExecutionPolicy Bypass"),
            "Setup-Bridge.ps1 must not use '-ExecutionPolicy Bypass'."
        );

        // 4. Repackage-Bundle.py
        let repackage_py = std::fs::read_to_string(root.join("distribution/Repackage-Bundle.py"))
            .expect("read Repackage-Bundle.py");
        assert!(
            !repackage_py.contains("shutil.rmtree(upstream_dir"),
            "Repackage-Bundle.py must NOT delete upstream_dir. Upstream MSI must remain in the bundle to prevent runtime download heuristics."
        );
        assert!(
            repackage_py.contains("normalize_lockfile"),
            "Repackage-Bundle.py must normalize Cargo.lock version when comparing lockfiles."
        );
        assert!(
            repackage_py.contains("(folder / 'payload/GlazeTheme.exe').unlink(missing_ok=True)"),
            "Repackage-Bundle.py must remove the duplicate 5MB GlazeTheme.exe binary."
        );
    }

    #[test]
    fn codeql_cryptographic_literals_safe() {
        let root = repo_root();
        let wifi_rs =
            std::fs::read_to_string(root.join("native-rust/src/wifi.rs")).expect("read wifi.rs");

        // CodeQL rust/hard-coded-cryptographic-value flags literal strings passed to profile(...)
        assert!(
            !wifi_rs.contains("profile(&n, \"\")"),
            "wifi.rs tests must NOT pass literal empty string \"\" to profile (triggers CodeQL hard-coded cryptographic value alert)."
        );
        assert!(
            !wifi_rs.contains("profile(&network(), &\""),
            "wifi.rs tests must NOT pass string literals to profile(...) directly."
        );
    }
}
