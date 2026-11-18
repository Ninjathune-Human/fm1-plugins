//! TastyWaves : plugin VST3 et CLAP du synthé à tables d'ondes du firmware de marksweiss/tastywaves.

pub mod core;
mod params;
mod rate;

use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::core::{Noyau, BLOC, FS};
use crate::params::{de_hex, en_hex, Parametres};
use crate::rate::Rateur;

const MAX_MIDI: usize = 64; // la file MIDI du firmware tient 64 messages par bloc

pub struct TastyWaves {
    params: Arc<Parametres>,
    envoyes: [i32; Parametres::N], // dernière valeur de chaque paramètre transmise au firmware
    noyau: Option<Noyau>,
    rateur: Rateur,
    en_attente: [[u8; 3]; MAX_MIDI],
    nb_attente: usize,
}

impl Default for TastyWaves {
    fn default() -> Self {
        Self { params: Arc::new(Parametres::default()), envoyes: valeurs_defaut(), noyau: None, rateur: Rateur::new(FS), en_attente: [[0; 3]; MAX_MIDI], nb_attente: 0 }
    }
}

fn valeurs_defaut() -> [i32; Parametres::N] {
    let p = Parametres::default();
    let mut v = [0; Parametres::N];
    for (i, (_, par, _)) in p.liste().iter().enumerate() {
        v[i] = par.default_plain_value();
    }
    v
}

impl TastyWaves {
    /// Un noyau neuf, avec les réglages et patchs du projet s'il y en a.
    fn nouveau_noyau(&mut self) -> Result<Noyau, String> {
        let mut n = Noyau::nouveau()?;
        if let Some(d) = de_hex(&self.params.keep.read().unwrap()) {
            if !d.is_empty() && !n.importer(&d) {
                nih_warn!("TastyWaves : état d'une autre version du firmware, ignoré");
            }
        }
        self.envoyes = valeurs_defaut(); // le firmware repart de ses défauts : tout écart sera retransmis
        Ok(n)
    }

    /// Met dans l'état du projet ce que le firmware vient d'écrire (réglages, patchs).
    fn garder(&mut self) {
        let Some(n) = self.noyau.as_mut() else { return };
        if n.modifie() {
            if let Ok(mut k) = self.params.keep.try_write() {
                *k = en_hex(&n.exporter());
            }
        }
    }

    /// Les paramètres changés depuis le dernier bloc partent en contrôleurs MIDI.
    fn envoyer_parametres(&mut self) {
        for (i, (cc, par, min)) in self.params.liste().iter().enumerate() {
            let v = par.value();
            if v != self.envoyes[i] && self.nb_attente < MAX_MIDI - 8 {
                let x = if *min < 0 { v - *min } else { v }; // un paramètre signé part décalé (0..127)
                self.en_attente[self.nb_attente] = [0xB0, *cc, x as u8];
                self.nb_attente += 1;
                self.envoyes[i] = v;
            }
        }
    }
}

/// Un événement de l'hôte en message MIDI à trois octets (None pour ce que le firmware n'écoute pas).
fn en_midi(e: &NoteEvent<()>) -> Option<[u8; 3]> {
    let v7 = |x: f32| (x.clamp(0.0, 1.0) * 127.0).round() as u8;
    Some(match *e {
        NoteEvent::NoteOn { channel, note, velocity, .. } => [0x90 | channel.min(15), note, v7(velocity).max(1)],
        NoteEvent::NoteOff { channel, note, velocity, .. } => [0x80 | channel.min(15), note, v7(velocity)],
        NoteEvent::PolyPressure { channel, note, pressure, .. } => [0xA0 | channel.min(15), note, v7(pressure)],
        NoteEvent::MidiCC { channel, cc, value, .. } => [0xB0 | channel.min(15), cc, v7(value)],
        NoteEvent::MidiProgramChange { channel, program, .. } => [0xC0 | channel.min(15), program, 0],
        NoteEvent::MidiChannelPressure { channel, pressure, .. } => [0xD0 | channel.min(15), v7(pressure), 0],
        NoteEvent::MidiPitchBend { channel, value, .. } => {
            let v = (value.clamp(0.0, 1.0) * 16383.0).round() as u16;
            [0xE0 | channel.min(15), (v & 0x7F) as u8, (v >> 7) as u8]
        }
        _ => return None,
    })
}

impl Plugin for TastyWaves {
    const NAME: &'static str = "TastyWaves";
    const VENDOR: &'static str = "Ninjathune";
    const URL: &'static str = "https://github.com/Ninjathune-Human/fm1-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(&mut self, _: &AudioIOLayout, cfg: &BufferConfig, ctx: &mut impl InitContext<Self>) -> bool {
        match self.nouveau_noyau() {
            Ok(n) => self.noyau = Some(n),
            Err(e) => {
                nih_error!("TastyWaves : {e}");
                return false;
            }
        }
        self.rateur = Rateur::new(cfg.sample_rate as f64);
        self.nb_attente = 0;
        // un bloc du firmware (256 images à 44,1 kHz) plus une image d'interpolation
        ctx.set_latency_samples(((BLOC + 1) as f64 * cfg.sample_rate as f64 / FS).ceil() as u32);
        true
    }

    fn reset(&mut self) {
        // le firmware n'a pas de remise à zéro : on repart d'une nouvelle instance
        if self.noyau.is_some() {
            self.noyau = self.nouveau_noyau().ok();
        }
        self.rateur.reset();
        self.nb_attente = 0;
    }

    fn deactivate(&mut self) {
        self.noyau = None;
    }

    fn process(&mut self, buffer: &mut Buffer, _: &mut AuxiliaryBuffers, ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        if self.noyau.is_none() {
            return ProcessStatus::Normal;
        }
        self.garder();
        self.envoyer_parametres();
        let noyau = self.noyau.as_mut().unwrap();
        let mut evt = ctx.next_event();
        for (s, trame) in buffer.iter_samples().enumerate() {
            // les messages arrivés jusqu'ici partent au prochain bloc du firmware (précision : 5,8 ms)
            while let Some(e) = evt {
                if e.timing() as usize > s {
                    break;
                }
                if let Some(m) = en_midi(&e) {
                    if self.nb_attente < MAX_MIDI {
                        self.en_attente[self.nb_attente] = m;
                        self.nb_attente += 1;
                    }
                }
                evt = ctx.next_event();
            }
            let (g, d) = self.rateur.suivant(|| {
                for m in &self.en_attente[..self.nb_attente] {
                    noyau.midi(m[0], m[1], m[2]);
                }
                self.nb_attente = 0;
                noyau.bloc()
            });
            let mut canaux = trame.into_iter();
            if let Some(c) = canaux.next() {
                *c = g;
            }
            if let Some(c) = canaux.next() {
                *c = d;
            }
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for TastyWaves {
    const CLAP_ID: &'static str = "io.github.ninjathune-human.fm1-plugins.tastywaves";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Synthé à tables d'ondes à 12 voix, tiré d'un firmware FM-1 open source");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
}

impl Vst3Plugin for TastyWaves {
    const VST3_CLASS_ID: [u8; 16] = *b"NinjaTastyWaves1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(TastyWaves);
nih_export_vst3!(TastyWaves);
