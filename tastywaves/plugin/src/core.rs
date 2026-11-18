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
type Store = unsafe extern "C" fn() -> *mut u8;
type StoreN = unsafe extern "C" fn() -> u32;
type Dirty = unsafe extern "C" fn() -> u32;

pub struct Noyau {
    avance: Avance,
    audio: Audio,
    midi: Midi,
    store: Store,
    store_n: StoreN,
    dirty: Dirty,
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
        let (avance, audio, midi, store, store_n, dirty) = unsafe {
            let a: Symbol<Avance> = lib.get(b"tw_advance\0").map_err(|e| e.to_string())?;
            let b: Symbol<Audio> = lib.get(b"web_audio\0").map_err(|e| e.to_string())?;
            let c: Symbol<Midi> = lib.get(b"web_midi\0").map_err(|e| e.to_string())?;
            let d: Symbol<Store> = lib.get(b"web_store\0").map_err(|e| e.to_string())?;
            let e: Symbol<StoreN> = lib.get(b"web_store_n\0").map_err(|e| e.to_string())?;
            let f: Symbol<Dirty> = lib.get(b"web_store_take_dirty\0").map_err(|e| e.to_string())?;
            (*a, *b, *c, *d, *e, *f)
        };
        #[cfg(not(windows))]
        let _ = fs::remove_file(&chemin); // déjà chargée : le fichier peut disparaître
        Ok(Self { avance, audio, midi, store, store_n, dirty, lib: Some(lib), chemin })
    }

    /// Les réglages et la bibliothèque de patchs du firmware (son bloc `keep`), tels quels.
    pub fn exporter(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts((self.store)(), (self.store_n)() as usize).to_vec() }
    }

    /// Remet un bloc exporté plus tôt ; à faire avant le premier bloc, le firmware le lit au démarrage.
    /// Refuse un bloc d'une autre taille (version différente du firmware).
    pub fn importer(&mut self, d: &[u8]) -> bool {
        let n = unsafe { (self.store_n)() } as usize;
        if d.len() != n {
            return false;
        }
        unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), (self.store)(), n) };
        true
    }

    /// Vrai si le firmware a écrit ses réglages ou un patch depuis le dernier appel.
    pub fn modifie(&mut self) -> bool {
        unsafe { (self.dirty)() != 0 }
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
    fn etat_aller_retour() {
        let mut a = Noyau::nouveau().unwrap();
        for _ in 0..20 {
            a.bloc();
        }
        let mut e = a.exporter();
        assert!(!e.is_empty());
        e[8] ^= 0x5A; // une valeur reconnaissable dans la bibliothèque
        let mut b = Noyau::nouveau().unwrap();
        assert!(b.importer(&e));
        assert_eq!(b.exporter(), e);
        assert!(!b.importer(&e[1..]));
    }

    /// Le contrôleur 74 (coupure) du tableau des paramètres change bien le son.
    #[test]
    fn coupure_audible() {
        let jouer = |cc: Option<u8>| {
            let mut n = Noyau::nouveau().unwrap();
            for _ in 0..100 {
                n.bloc();
            }
            if let Some(v) = cc {
                n.midi(0xB0, 74, v);
            }
            n.midi(0x90, 60, 100);
            let mut e = 0.0f64;
            for _ in 0..200 {
                e += n.bloc().iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>();
            }
            e
        };
        let (defaut, ferme) = (jouer(None), jouer(Some(10)));
        assert!(defaut > 0.0 && ferme < defaut * 0.8, "défaut {defaut}, fermé {ferme}");
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
