use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, env, fs, path::Path, process::Command};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct UpstreamLock {
    engrammer: String,
    engrammer_commit: String,
    enthium: String,
    enthium_commit: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

fn command(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("run {command:?}"))?;
    ensure!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

fn github(path: &str) -> Result<Value> {
    Ok(serde_json::from_str(&command(
        Command::new("gh").args(["api", path]),
    )?)?)
}

fn release(repository: &str) -> Result<String> {
    let release: Release = serde_json::from_value(github(&format!(
        "repos/sunaku/{repository}/releases/latest"
    ))?)?;
    ensure!(
        !release.draft && !release.prerelease,
        "expected stable release for {repository}"
    );
    ensure!(
        release.tag_name.starts_with('v')
            && release.tag_name[1..].chars().all(|c| c.is_ascii_digit()),
        "unsupported release tag {}",
        release.tag_name
    );
    Ok(release.tag_name)
}

fn revision(repository: &str, tag: &str) -> Result<String> {
    github(&format!("repos/sunaku/{repository}/commits/{tag}"))?["sha"]
        .as_str()
        .context("release commit missing")
        .map(str::to_owned)
}

fn checkout(repository: &str, commit: &str, path: &Path) -> Result<()> {
    ensure!(
        commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
        "invalid commit"
    );
    command(Command::new("git").args(["init", "--quiet"]).arg(path))?;
    command(Command::new("git").arg("-C").arg(path).args([
        "fetch",
        "--quiet",
        "--depth=1",
        &format!("https://github.com/sunaku/{repository}.git"),
        commit,
    ]))?;
    command(Command::new("git").arg("-C").arg(path).args([
        "checkout",
        "--quiet",
        "--detach",
        "FETCH_HEAD",
    ]))?;
    Ok(())
}

// Read the stable release's machine-readable XKB mapping; reject unsupported
// keysyms and missing physical positions instead of interpreting documentation.
fn enthium_keys(source: &str) -> Result<BTreeMap<String, String>> {
    ensure!(
        source.contains("xkb_symbols \"enthium\""),
        "unsupported Enthium XKB structure"
    );
    let pattern =
        Regex::new(r"key\s+<([A-Z0-9]+)>\s*\{\s*\[\s*([A-Za-z_]+)(?:\s*,\s*([A-Za-z_]+))?\s*\]")?;
    let mut mapped = BTreeMap::new();
    for capture in pattern.captures_iter(source) {
        let zmk = match &capture[2] {
            "apostrophe" => "SQT",
            "comma" => "COMMA",
            "period" => "DOT",
            "semicolon" => "SEMI",
            "slash" => "FSLH",
            "equal" => "EQUAL",
            "minus" => "MINUS",
            "Alt_R" | "Control_R" | "Caps_Lock" => continue,
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_lowercase() => key,
            key => anyhow::bail!("unsupported Enthium keysym {key}"),
        };
        let shifted = match &capture[2] {
            "apostrophe" => "quotedbl".to_owned(),
            "comma" => "less".to_owned(),
            "period" => "greater".to_owned(),
            "semicolon" => "colon".to_owned(),
            "slash" => "question".to_owned(),
            "equal" => "plus".to_owned(),
            "minus" => "underscore".to_owned(),
            letter => letter.to_uppercase(),
        };
        ensure!(
            capture.get(3).map(|value| value.as_str()) == Some(shifted.as_str()),
            "unsupported shifted Enthium symbol at {}",
            &capture[1]
        );
        ensure!(
            mapped
                .insert(capture[1].to_owned(), zmk.to_uppercase())
                .is_none(),
            "duplicate Enthium physical key"
        );
    }

    ensure!(mapped.len() == 33, "unsupported Enthium physical layout");
    Ok(mapped)
}

fn render_binding(binding: &Value, names: &[Value]) -> Result<String> {
    if let Some(value) = binding["value"].as_u64() {
        return Ok(value.to_string());
    }

    let value = binding["value"].as_str().context("binding value missing")?;
    if value == "&magic" {
        return Ok("&magic LAYER_Magic 0".to_owned());
    }

    if value == "&reset" {
        return Ok("&sys_reset".to_owned());
    }

    if value == "&tog" {
        let index = binding["params"][0]["value"]
            .as_u64()
            .context("toggle layer missing")? as usize;
        let name = names
            .get(index)
            .and_then(Value::as_str)
            .context("toggle layer invalid")?;
        return Ok(format!("&tog LAYER_{name}"));
    }

    let params = binding["params"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if value == "Custom" {
        ensure!(params.len() == 1, "unsupported custom binding");
        return Ok(params[0]["value"]
            .as_str()
            .context("custom binding missing")?
            .to_owned());
    }

    let mut text = value.to_owned();
    for param in params {
        let param = render_binding(param, names)?;
        if value.starts_with('&') {
            text.push(' ');
            text.push_str(&param);
        } else {
            text.push('(');
            text.push_str(&param);
            text.push(')');
        }
    }

    Ok(text)
}

fn regenerate(root: &Path, upstream: &Path, enthium: &Path) -> Result<(Value, String)> {
    let original: Value = serde_json::from_str(&fs::read_to_string(upstream.join("keymap.json"))?)?;
    let local: Value = serde_json::from_str(&fs::read_to_string(root.join("config/keymap.json"))?)?;
    let mut layout = original.clone();
    // User metadata survives; generated content and configuration come from upstream.
    for field in [
        "uuid",
        "parent_uuid",
        "unlisted",
        "creator",
        "title",
        "notes",
    ] {
        layout[field] = local[field].clone();
    }

    let keys = enthium_keys(&fs::read_to_string(
        enthium.join("linux/usr-share-X11-xkb-symbols-us"),
    )?)?;
    let positions = [
        (23, "AD01"),
        (24, "AD02"),
        (25, "AD03"),
        (26, "AD04"),
        (27, "AD05"),
        (28, "AD06"),
        (29, "AD07"),
        (30, "AD08"),
        (31, "AD09"),
        (32, "AD10"),
        (34, "CAPS"),
        (35, "AC01"),
        (36, "AC02"),
        (37, "AC03"),
        (38, "AC04"),
        (39, "AC05"),
        (40, "AC06"),
        (41, "AC07"),
        (42, "AC08"),
        (43, "AC09"),
        (44, "AC10"),
        (45, "AC11"),
        (47, "AB01"),
        (48, "AB02"),
        (49, "AB03"),
        (50, "AB04"),
        (51, "AB05"),
        (58, "AB06"),
        (59, "AB07"),
        (60, "AB08"),
        (61, "AB09"),
        (62, "AB10"),
        (74, "RALT"),
    ];
    ensure!(
        layout["layer_names"][0] == "Enthium",
        "Enthium must remain the base layer"
    );
    ensure!(
        layout["layers"][0]
            .as_array()
            .context("base layer missing")?
            .len()
            == 80,
        "unsupported key count"
    );
    let home_key = Regex::new(r"(\(\s*)([A-Z][A-Z0-9_]*)(\s*,\s*LAYER_Enthium\))$")?;
    let thumb_key = Regex::new(r"(\s)([A-Z][A-Z0-9_]*)(\s*)$")?;
    for (position, physical) in positions {
        let key = keys
            .get(physical)
            .with_context(|| format!("unsupported Enthium key at {physical}"))?;
        let binding = &mut layout["layers"][0][position];
        let previous = render_binding(binding, &[])?;
        if binding["value"] == "&kp" {
            binding["params"][0]["value"] = Value::String(key.clone());
        } else {
            ensure!(
                binding["value"] == "Custom",
                "unsupported binding at {position}"
            );
            let text = binding["params"][0]["value"]
                .as_str()
                .context("custom key missing")?;
            let pattern = if text.contains('(') {
                &home_key
            } else {
                &thumb_key
            };
            ensure!(
                pattern.is_match(text),
                "unsupported custom key at {position}"
            );
            binding["params"][0]["value"] = Value::String(
                pattern
                    .replace(text, |caps: &regex::Captures<'_>| {
                        format!("{}{key}{}", &caps[1], &caps[3])
                    })
                    .into_owned(),
            );
        }

        if previous != render_binding(binding, &[])?
            && let Some(decoration) = binding.get_mut("decoration")
        {
            decoration["label"] = Value::String(key.clone());
        }
    }

    fs::write(
        upstream.join("keymap.json"),
        serde_json::to_string_pretty(&layout)?,
    )?;
    // The read-only generation job has no publishing credential. Do not pass
    // even its API token into upstream generation code.
    command(
        Command::new("rake")
            .current_dir(upstream)
            .args(["-B", "keymap.dtsi"])
            .env_remove("GH_TOKEN")
            .env_remove("GITHUB_TOKEN"),
    )?;
    let behaviors = fs::read_to_string(upstream.join("keymap.dtsi"))?;
    let linux = "#define OPERATING_SYSTEM 'L' // choose 'L'inux, 'M'acOS, or 'W'indows";
    ensure!(
        behaviors.matches(linux).count() == 1,
        "upstream OS override changed"
    );
    layout["custom_defined_behaviors"] = Value::String(behaviors.replacen(
        linux,
        "#define OPERATING_SYSTEM 'M' // macOS shortcuts and CAGS home-row modifiers",
        1,
    ));
    layout["custom_devicetree"] = Value::String(fs::read_to_string(upstream.join("device.dtsi"))?);
    ensure!(
        layout["config_parameters"] == local["config_parameters"],
        "upstream configuration changed; review HID pointing configuration"
    );
    let mut firmware = fs::read_to_string(upstream.join("keymap.zmk"))?;
    for field in ["custom_defined_behaviors", "custom_devicetree"] {
        let old = original[field]
            .as_str()
            .context("missing upstream snippet")?;
        ensure!(
            firmware.matches(old).count() == 1,
            "upstream firmware/snippet mismatch: {field}"
        );
        firmware = firmware.replacen(
            old,
            layout[field]
                .as_str()
                .context("missing generated snippet")?,
            1,
        );
    }

    let names = layout["layer_names"]
        .as_array()
        .context("layer names missing")?;
    let layers = layout["layers"].as_array().context("layers missing")?;
    ensure!(names.len() == layers.len(), "layer count mismatch");
    for (name, layer) in names.iter().zip(layers) {
        let name = name.as_str().context("invalid layer name")?;
        ensure!(
            name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "invalid layer name"
        );
        let pattern = Regex::new(&format!(
            r"(?s)(layer_{}\s*\{{\s*bindings\s*=\s*<).*?(>;)",
            regex::escape(name)
        ))?;
        ensure!(
            pattern.find_iter(&firmware).count() == 1,
            "unsupported exported layer {name}"
        );
        let bindings = layer.as_array().context("invalid layer")?;
        ensure!(bindings.len() == 80, "unsupported layer size for {name}");
        let rendered = bindings
            .iter()
            .map(|binding| render_binding(binding, names))
            .collect::<Result<Vec<_>>>()?
            .join("\n                ");
        firmware = pattern
            .replace(&firmware, |caps: &regex::Captures<'_>| {
                format!(
                    "{}\n                {rendered}\n            {}",
                    &caps[1], &caps[2]
                )
            })
            .into_owned();
    }

    Ok((layout, firmware))
}

fn tokens(source: &str) -> String {
    let comments = Regex::new(r"(?s)/\*.*?\*/|//[^\n]*").expect("constant regex");
    comments
        .replace_all(source, "")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn main() -> Result<()> {
    let mode = env::args().nth(1).unwrap_or_else(|| "update".to_owned());
    ensure!(
        mode == "update" || mode == "check",
        "usage: upstream-keymaps [update|check]"
    );
    let root = env::current_dir()?;
    let lock_path = root.join("upstream-lock.json");
    let old: UpstreamLock = serde_json::from_str(&fs::read_to_string(&lock_path)?)?;
    let mut next = UpstreamLock {
        engrammer: old.engrammer.clone(),
        engrammer_commit: old.engrammer_commit.clone(),
        enthium: old.enthium.clone(),
        enthium_commit: old.enthium_commit.clone(),
    };
    if mode == "update" {
        next.engrammer = release("glove80-keymaps")?;
        next.enthium = release("enthium")?;
        if next.engrammer != old.engrammer {
            next.engrammer_commit = revision("glove80-keymaps", &next.engrammer)?;
        }

        if next.enthium != old.enthium {
            next.enthium_commit = revision("enthium", &next.enthium)?;
        }

        ensure!(
            next.engrammer[1..].parse::<u64>()? >= old.engrammer[1..].parse::<u64>()?
                && next.enthium[1..].parse::<u64>()? >= old.enthium[1..].parse::<u64>()?,
            "latest upstream release would downgrade locked versions"
        );
        if next == old {
            println!("No new stable upstream releases.");
            return Ok(());
        }
    }

    if mode == "update" {
        command(Command::new(env::current_exe()?).arg("check"))
            .context("local configuration diverged; reconcile customizations before updating")?;
    }

    let temporary = tempfile::tempdir()?;
    let upstream = temporary.path().join("engrammer");
    let enthium = temporary.path().join("enthium");
    checkout("glove80-keymaps", &next.engrammer_commit, &upstream)?;
    checkout("enthium", &next.enthium_commit, &enthium)?;
    let (mut layout, firmware) = regenerate(&root, &upstream, &enthium)?;
    if mode == "check" {
        let local: Value =
            serde_json::from_str(&fs::read_to_string(root.join("config/keymap.json"))?)?;
        for field in ["custom_defined_behaviors", "custom_devicetree"] {
            ensure!(
                tokens(
                    layout[field]
                        .as_str()
                        .context("generated snippet missing")?
                ) == tokens(local[field].as_str().context("local snippet missing")?),
                "{field} differs from locked regeneration"
            );
            layout[field] = local[field].clone();
        }

        ensure!(layout == local, "JSON differs from locked regeneration");
        ensure!(
            tokens(&firmware) == tokens(&fs::read_to_string(root.join("config/glove80.keymap"))?),
            "firmware differs from locked regeneration"
        );
        let xkb_path = enthium.join("linux/usr-share-X11-xkb-symbols-us");
        let xkb = fs::read_to_string(&xkb_path)?;
        let replacement = if enthium_keys(&xkb)?.get("AD01").map(String::as_str) == Some("Z") {
            "q"
        } else {
            "z"
        };
        let expected_key = replacement.to_uppercase();
        let expected_define = format!("#define KEY_LH_C5R3 {expected_key}");
        let pattern = Regex::new(r"(key\s+<AD01>\s*\{\s*\[\s*)[a-z]+(\s*,\s*)[A-Z]+")?;
        ensure!(
            pattern.is_match(&xkb),
            "independent-update fixture requires an AD01 letter"
        );
        fs::write(
            &xkb_path,
            pattern
                .replace(&xkb, format!("${{1}}{replacement}${{2}}{expected_key}"))
                .as_ref(),
        )?;
        let (changed, changed_firmware) = regenerate(&root, &upstream, &enthium)?;
        ensure!(
            render_binding(&changed["layers"][0][23], &[])? == format!("&kp {expected_key}"),
            "independent Enthium update did not change binding"
        );
        ensure!(
            changed["custom_defined_behaviors"]
                .as_str()
                .context("missing behavior")?
                .contains(&expected_define),
            "independent update did not regenerate upstream key definitions"
        );
        ensure!(
            changed_firmware.contains(&expected_define)
                && changed_firmware.contains("#define OPERATING_SYSTEM 'M'"),
            "independent update lost generated definitions or macOS override"
        );
        println!(
            "Locked JSON/firmware match; independent Enthium mutation regenerates bindings and key definitions with macOS preserved."
        );
        return Ok(());
    }

    layout["title"] = Value::String(format!(
        "Enthium {} for macOS (Glorious Engrammer {})",
        next.enthium, next.engrammer
    ));
    layout["notes"] = Value::String(format!(
        "Glorious Engrammer {} / Enthium {}, configured for macOS. See upstream-lock.json for exact source commits and README.md for build instructions.",
        next.engrammer, next.enthium
    ));
    // All generation/validation completes before any tracked output is written.
    fs::write(
        root.join("config/keymap.json"),
        format!("{}\n", serde_json::to_string_pretty(&layout)?),
    )?;
    fs::write(root.join("config/glove80.keymap"), firmware)?;
    fs::write(
        lock_path,
        format!("{}\n", serde_json::to_string_pretty(&next)?),
    )?;
    println!("Prepared {} / {} for review.", next.engrammer, next.enthium);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_shifted_symbol() {
        let error =
            enthium_keys("xkb_symbols \"enthium\" { key <AD01> { [ q, X ] }; };").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported shifted Enthium symbol")
        );
    }

    #[test]
    fn renders_nested_modifier_and_custom_binding() -> Result<()> {
        let modifier: Value = serde_json::from_str(
            r#"{"value":"&kp","params":[{"value":"LG","params":[{"value":"A"}]}]}"#,
        )?;
        assert_eq!(render_binding(&modifier, &[])?, "&kp LG(A)");
        let custom: Value = serde_json::from_str(
            r#"{"value":"Custom","params":[{"value":"&thumb LAYER_Symbol R"}]}"#,
        )?;
        assert_eq!(render_binding(&custom, &[])?, "&thumb LAYER_Symbol R");
        Ok(())
    }
}
