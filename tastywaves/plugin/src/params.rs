//! Les paramètres automatisables : chacun est un contrôleur MIDI de la table du firmware (patch.h, patch_cc).
//! Le sens est unique : un changement de paramètre part en contrôleur MIDI vers le firmware.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{Arc, RwLock};

/// (champ, nom, contrôleur, minimum, maximum, défaut du firmware). Un minimum négatif : valeur envoyée = valeur + 64.
macro_rules! table {
    ($( $champ:ident, $nom:literal, $cc:literal, $min:literal, $max:literal, $def:literal; )*) => {
        #[derive(Params)]
        pub struct Parametres {
            $( #[id = $nom] pub $champ: IntParam, )*
            #[persist = "editor-state"]
            pub fenetre: Arc<EguiState>,
            /// Réglages et patchs du firmware, en hexadécimal ; mis à jour quand le firmware les écrit.
            #[persist = "keep"]
            pub keep: RwLock<String>,
        }

        impl Default for Parametres {
            fn default() -> Self {
                Self {
                    fenetre: EguiState::from_size(520, 800),
                    $( $champ: IntParam::new($nom, $def, IntRange::Linear { min: $min, max: $max }), )*
                    keep: RwLock::new(String::new()),
                }
            }
        }

        impl Parametres {
            /// Contrôleur, valeur courante, défaut, pour chaque paramètre.
            pub const N: usize = [$( $cc ),*].len();
            pub fn liste(&self) -> [(u8, &IntParam, i32); Self::N] {
                [$( ($cc, &self.$champ, $min) ),*]
            }
        }
    };
}

table! {
    cutoff, "cutoff", 74, 0, 127, 110;
    reso, "reso", 71, 0, 127, 20;
    keytrack, "keytrack", 83, 0, 127, 64;
    filter_env, "filter_env", 28, -64, 63, 0;
    pos1, "pos1", 70, 0, 127, 0;
    level1, "level1", 86, 0, 127, 100;
    pos2, "pos2", 22, 0, 127, 0;
    level2, "level2", 23, 0, 127, 80;
    sub, "sub", 24, 0, 127, 0;
    noise, "noise", 25, 0, 127, 0;
    attack, "attack", 73, 0, 127, 10;
    decay, "decay", 75, 0, 127, 80;
    sustain, "sustain", 76, 0, 127, 100;
    release, "release", 72, 0, 127, 70;
    lfo_rate, "lfo_rate", 26, 0, 127, 60;
    lfo_pos, "lfo_pos", 27, 0, 127, 0;
    lfo_pitch, "lfo_pitch", 17, 0, 127, 0;
    lfo_cut, "lfo_cut", 18, 0, 127, 0;
    lfo_amp, "lfo_amp", 19, 0, 127, 0;
    glide, "glide", 5, 0, 127, 0;
    drive, "drive", 29, 0, 127, 0;
    volume, "volume", 7, 0, 127, 100;
}

pub fn en_hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn de_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_aller_retour() {
        let d: Vec<u8> = (0..=255).collect();
        assert_eq!(de_hex(&en_hex(&d)), Some(d));
        assert_eq!(de_hex("abc"), None);
        assert_eq!(de_hex("zz"), None);
    }

    #[test]
    fn controleurs_distincts() {
        let p = Parametres::default();
        let mut cc: Vec<u8> = p.liste().iter().map(|(c, _, _)| *c).collect();
        cc.sort();
        cc.dedup();
        assert_eq!(cc.len(), Parametres::N);
    }
}
