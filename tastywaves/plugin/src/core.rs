//! Le noyau : le firmware de TastyWaves, chargé depuis sa bibliothèque dynamique embarquée.
//! Une copie de la bibliothèque par instance, car le firmware garde son état dans des variables statiques.

use libloading::{Library, Symbol};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, path::PathBuf};

static NOYAU: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/noyau.bin"));
static COMPTEUR: AtomicU32 = AtomicU32::new(0);

/// Fréquence d'échantillonnage du firmware, et taille de ses blocs.
pub const FS: f64 = 44_100.0;
pub const BLOC: usize = 256;

type Avance = unsafe extern "C" fn(f64);
type Audio = unsafe extern "C" fn() -> *const f32;
type Midi = unsafe extern "C" fn(u32, u32, u32);

pub struct Noyau {
    avance: Avance,
    audio: Audio,
    midi: Midi,
    lib: Option<Library>, // gardée tant que les pointeurs de fonction servent ; déchargée dans Drop
    chemin: PathBuf,
}

impl Noyau {
    pub fn nouveau() -> Result<Self, String> {
        let n = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let ext = if cfg!(windows) { "dll" } else if cfg!(target_os = "macos") { "dylib" } else { "so" };
        let chemin = std::env::temp_dir().join(format!("tastywaves-{}-{n}.{ext}", std::process::id()));
        fs::write(&chemin, NOYAU).map_err(|e| format!("écriture de {}: {e}", chemin.display()))?;
        let lib = unsafe { Library::new(&chemin) }.map_err(|e| format!("chargement du noyau: {e}"))?;
        let (avance, audio, midi) = unsafe {
            let a: Symbol<Avance> = lib.get(b"tw_advance\0").map_err(|e| e.to_string())?;
            let b: Symbol<Audio> = lib.get(b"web_audio\0").map_err(|e| e.to_string())?;
            let c: Symbol<Midi> = lib.get(b"web_midi\0").map_err(|e| e.to_string())?;
            (*a, *b, *c)
        };
        #[cfg(not(windows))]
        let _ = fs::remove_file(&chemin); // déjà chargée : le fichier peut disparaître
        Ok(Self { avance, audio, midi, lib: Some(lib), chemin })
    }

    /// Un message MIDI (état, données 1 et 2), pris en compte au prochain bloc.
    pub fn midi(&mut self, etat: u8, d1: u8, d2: u8) {
        unsafe { (self.midi)(etat as u32, d1 as u32, d2 as u32) }
    }

    /// Fait avancer le firmware d'un bloc (256 images à 44,1 kHz) et rend ses 256 images stéréo, entrelacées.
    pub fn bloc(&mut self) -> &[f32] {
        unsafe {
            (self.avance)(BLOC as f64 * 1000.0 / FS);
            std::slice::from_raw_parts((self.audio)(), BLOC * 2)
        }
    }
}

impl Drop for Noyau {
    fn drop(&mut self) {
        drop(self.lib.take()); // sous Windows, la DLL doit être déchargée avant d'effacer son fichier
        let _ = fs::remove_file(&self.chemin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Même scénario que le banc (tastywaves/banc) : 100 blocs de démarrage, note 60, 344 blocs, somme FNV des bits des flottants.
    fn somme(note: u8) -> u64 {
        let mut n = Noyau::nouveau().unwrap();
        let mut s: u64 = 0;
        for b in 0..(100 + 44100 * 2 / 256) {
            if b == 100 {
                n.midi(0x90, note, 100);
            }
            for x in n.bloc() {
                s = s.wrapping_mul(1099511628211).wrapping_add(x.to_bits() as u64);
            }
        }
        s
    }

    #[test]
    fn identique_au_banc() {
        assert_eq!(format!("{:016x}", somme(60)), "4350bbfe0b9c0000");
    }

    #[test]
    fn instances_isolees() {
        let seule = somme(64);
        let (mut a, mut b) = (Noyau::nouveau().unwrap(), Noyau::nouveau().unwrap());
        a.midi(0x90, 60, 100);
        for _ in 0..50 {
            a.bloc();
            b.bloc();
        }
        drop(a);
        drop(b);
        assert_eq!(seule, somme(64));
    }
}
