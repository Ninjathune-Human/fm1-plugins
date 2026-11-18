//! Du rythme du firmware (blocs de 256 images stéréo à 44,1 kHz) à celui de l'hôte, par interpolation linéaire.
//! À 44,1 kHz, la sortie est celle du firmware, retardée d'une image.

use crate::core::{BLOC, FS};

pub struct Rateur {
    pas: f64,
    t: f64,                    // position dans `images`, en images du firmware
    images: [[f32; 2]; BLOC + 1], // [0] : dernière image du bloc précédent ; [1..] : le bloc courant
}

impl Rateur {
    pub fn new(fs_hote: f64) -> Self {
        Self { pas: FS / fs_hote, t: BLOC as f64, images: [[0.0; 2]; BLOC + 1] }
    }

    pub fn reset(&mut self) {
        self.t = BLOC as f64;
        self.images = [[0.0; 2]; BLOC + 1];
    }

    /// L'image suivante pour l'hôte ; `bloc` donne le bloc suivant du firmware quand il faut le tirer.
    pub fn suivant<'a>(&mut self, bloc: impl FnOnce() -> &'a [f32]) -> (f32, f32) {
        if self.t >= BLOC as f64 {
            self.t -= BLOC as f64;
            self.images[0] = self.images[BLOC];
            for (i, p) in bloc().chunks_exact(2).enumerate() {
                self.images[i + 1] = [p[0], p[1]];
            }
        }
        let i = self.t as usize;
        let f = (self.t - i as f64) as f32;
        let (a, b) = (self.images[i], self.images[i + 1]);
        self.t += self.pas;
        (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f)
    }
}
