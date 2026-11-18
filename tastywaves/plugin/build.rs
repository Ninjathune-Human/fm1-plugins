//! Compile le firmware de TastyWaves et son hôte (tastywaves/vendor, tastywaves/core) en bibliothèque dynamique,
//! que le plugin embarque et charge en une copie par instance (le firmware garde son état dans des variables
//! statiques : deux instances dans un même processus ne peuvent pas partager une même bibliothèque).
//! Compilateur : TW_CC, sinon `cc` (gcc ou clang), sinon `gcc` sous Windows. Un compilateur de type GNU est requis.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let racine = dir.parent().unwrap().to_path_buf(); // tastywaves/
    let (vendor, core) = (racine.join("vendor"), racine.join("core"));
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("noyau.bin");
    let cible = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let cc = env::var("TW_CC").unwrap_or_else(|_| if cible == "windows" { "gcc".into() } else { "cc".into() });

    let mut cmd = Command::new(&cc);
    cmd.current_dir(&vendor)
        .args(["-O2", "-w", "-fPIC", if cible == "macos" { "-dynamiclib" } else { "-shared" }])
        .args(["-DDINGHY_HOST", "-DAPP_VER=\"1.0\"", "-DDH_CV_ROWS=64u"])
        .arg(format!("-I{}", core.join("shim").display()))
        .args(["-Ifirmware/src", "-Ifirmware/gen", "firmware/src/tastywaves.c"])
        .arg(core.join("runtime.c"))
        .arg("-o")
        .arg(&out);
    let etat = cmd.status().unwrap_or_else(|e| panic!("compilateur C `{cc}` introuvable : {e} (variable TW_CC)"));
    assert!(etat.success(), "la compilation du noyau a échoué");

    println!("cargo:rerun-if-changed={}", vendor.display());
    println!("cargo:rerun-if-changed={}", core.display());
    println!("cargo:rerun-if-env-changed=TW_CC");
}
