use alvr_filesystem as afs;
use std::{env, fs, path::Path};

pub fn load_android_env() {
    let env_path = afs::workspace_dir().join(".env");

    if env_path.exists() {
        let env_contents = fs::read_to_string(&env_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", env_path.display()));

        let mut loaded_any = false;

        for (line_no, raw_line) in env_contents.lines().enumerate() {
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, raw_value) = line.split_once('=').unwrap_or_else(|| {
                panic!(
                    "Invalid env entry in {} at line {}: {raw_line}",
                    env_path.display(),
                    line_no + 1
                )
            });

            let key = key.trim();
            if key.is_empty() {
                panic!(
                    "Invalid env key in {} at line {}",
                    env_path.display(),
                    line_no + 1
                );
            }

            if env::var_os(key).is_some() {
                continue;
            }

            let value = parse_value(raw_value.trim(), &env_path, line_no + 1);
            unsafe { env::set_var(key, value) };
            loaded_any = true;
        }

        if loaded_any {
            println!("Loaded Android environment from {}", env_path.display());
        }
    }

    sync_alias("ANDROID_HOME", "ANDROID_SDK_ROOT");
    sync_alias("ANDROID_SDK_ROOT", "ANDROID_HOME");
    sync_alias("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT");
    sync_alias("ANDROID_NDK_ROOT", "ANDROID_NDK_HOME");
}

fn parse_value(raw_value: &str, env_path: &Path, line_no: usize) -> String {
    if raw_value.is_empty() {
        return String::new();
    }

    if raw_value.starts_with('"') {
        return parse_quoted_value(raw_value, env_path, line_no, '"');
    }

    if raw_value.starts_with('\'') {
        return parse_quoted_value(raw_value, env_path, line_no, '\'');
    }

    raw_value.to_owned()
}

fn parse_quoted_value(raw_value: &str, env_path: &Path, line_no: usize, quote: char) -> String {
    if !raw_value.ends_with(quote) || raw_value.len() < 2 {
        panic!(
            "Unterminated quoted value in {} at line {}",
            env_path.display(),
            line_no
        );
    }

    let inner = &raw_value[1..raw_value.len() - 1];

    if quote == '"' {
        inner
            .replace(r#"\""#, "\"")
            .replace(r#"\n"#, "\n")
            .replace(r#"\r"#, "\r")
            .replace(r#"\t"#, "\t")
    } else {
        inner.to_owned()
    }
}

fn sync_alias(target: &str, source: &str) {
    if env::var_os(target).is_none() {
        if let Some(value) = env::var_os(source) {
            unsafe { env::set_var(target, value) };
        }
    }
}
