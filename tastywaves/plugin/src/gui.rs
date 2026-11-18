//! L'interface : l'écran du firmware, ses sept encodeurs et ses quatorze boutons.
//! Le noyau ne sort jamais du fil audio : la fenêtre dépose ses gestes dans `Partage`, et reçoit l'écran en retour.

use crate::core::ECRAN;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, FontId, Rect, Sense, Stroke, Vec2};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::sync::{Arc, Mutex};

pub enum Geste {
    Bouton(u32, bool),
    Encodeur(u32, i32),
}

pub struct Etat {
    pub ecran: Vec<u16>,
    pub neuf: bool, // un nouvel écran est arrivé
    pub temoins: [bool; 14],
    pub gestes: Vec<Geste>,
}

pub type Partage = Arc<Mutex<Etat>>;

pub fn partage() -> Partage {
    Arc::new(Mutex::new(Etat { ecran: vec![0; ECRAN * ECRAN], neuf: false, temoins: [false; 14], gestes: Vec::new() }))
}

const BOUTONS: [&str; 14] = ["OCT-", "OCT+", "FX", "SEL", "ENV", "LFO", "EDIT", "GLO", "HOME", "SAVE", "ARP", "SEQ", "PLAY", "REC"];
const ENCODEURS: [&str; 7] = ["SELECT", "TABLE", "1", "2", "3", "4", "PRESET"];

const FOND: Color32 = Color32::from_rgb(0x10, 0x10, 0x11);
const TRAIT: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3e);
const TEXTE: Color32 = Color32::from_rgb(0xb8, 0xb8, 0xbc);
const ACCENT: Color32 = Color32::from_rgb(0xe8, 0xa3, 0x3c);

struct Local {
    partage: Partage,
    ecran: Option<egui::TextureHandle>,
    bas: [bool; 14],
    reste: [f32; 7], // déplacement vertical non encore converti en crans
}

fn rgb565(p: u16) -> Color32 {
    let (r, g, b) = ((p >> 11) as u8 & 31, (p >> 5) as u8 & 63, p as u8 & 31);
    Color32::from_rgb(r << 3 | r >> 2, g << 2 | g >> 4, b << 3 | b >> 2)
}

pub fn editeur(etat: Arc<EguiState>, partage: Partage) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        etat,
        Local { partage, ecran: None, bas: [false; 14], reste: [0.0; 7] },
        |ctx, _| {
            let mut v = egui::Visuals::dark();
            v.panel_fill = FOND;
            ctx.set_visuals(v);
        },
        |ctx, _, l| {
            // l'écran, mis à jour quand le noyau en a rendu un nouveau
            let (temoins, image) = {
                let mut e = l.partage.lock().unwrap();
                let image = if e.neuf {
                    e.neuf = false;
                    let px: Vec<Color32> = e.ecran.iter().map(|p| rgb565(*p)).collect();
                    Some(egui::ColorImage { size: [ECRAN, ECRAN], pixels: px })
                } else {
                    None
                };
                (e.temoins, image)
            };
            if let Some(img) = image {
                match &mut l.ecran {
                    Some(t) => t.set(img, egui::TextureOptions::NEAREST),
                    None => l.ecran = Some(ctx.load_texture("ecran", img, egui::TextureOptions::NEAREST)),
                }
            }

            let mut gestes: Vec<Geste> = Vec::new();
            egui::CentralPanel::default().show(ctx, |ui| {
                let largeur = ui.available_width();
                let marge = 18.0;
                let cote = (largeur - 2.0 * marge).min(ui.available_height() * 0.55).max(120.0);

                ui.add_space(marge);
                ui.vertical_centered(|ui| {
                    if let Some(t) = &l.ecran {
                        ui.add(egui::Image::new((t.id(), Vec2::splat(cote))).corner_radius(2.0));
                    } else {
                        ui.allocate_space(Vec2::splat(cote));
                    }
                });
                ui.add_space(14.0);

                // sept encodeurs : glisser à la verticale, ou molette
                let pas = (largeur - 2.0 * marge) / 7.0;
                let (zone, _) = ui.allocate_exact_size(Vec2::new(largeur - 2.0 * marge, 64.0), Sense::hover());
                for i in 0..7 {
                    let c = egui::pos2(zone.left() + pas * (i as f32 + 0.5), zone.top() + 22.0);
                    let r = Rect::from_center_size(c, Vec2::splat(40.0));
                    let rep = ui.interact(r, ui.id().with(("enc", i)), Sense::click_and_drag());
                    let molette = if rep.hovered() { ctx.input(|e| e.smooth_scroll_delta.y) } else { 0.0 };
                    l.reste[i] += -rep.drag_delta().y + molette;
                    let crans = (l.reste[i] / 10.0).trunc();
                    if crans != 0.0 {
                        l.reste[i] -= crans * 10.0;
                        gestes.push(Geste::Encodeur(i as u32, crans as i32));
                    }
                    let p = ui.painter();
                    p.circle_stroke(c, 18.0, Stroke::new(1.5_f32, if rep.dragged() { ACCENT } else { TRAIT }));
                    p.line_segment([c + Vec2::new(0.0, -6.0), c + Vec2::new(0.0, -14.0)], Stroke::new(1.5_f32, TEXTE)); // sans butée : repère fixe
                    p.text(c + Vec2::new(0.0, 32.0), egui::Align2::CENTER_CENTER, ENCODEURS[i], FontId::proportional(10.0), TEXTE);
                }
                ui.add_space(8.0);

                // quatorze boutons, deux rangées de sept
                for rang in 0..2 {
                    let (zone, _) = ui.allocate_exact_size(Vec2::new(largeur - 2.0 * marge, 30.0), Sense::hover());
                    for k in 0..7 {
                        let i = rang * 7 + k;
                        let c = egui::pos2(zone.left() + pas * (k as f32 + 0.5), zone.center().y);
                        let r = Rect::from_center_size(c, Vec2::new(pas - 8.0, 26.0));
                        let rep = ui.interact(r, ui.id().with(("btn", i)), Sense::click_and_drag());
                        let bas = rep.is_pointer_button_down_on();
                        if bas != l.bas[i] {
                            l.bas[i] = bas;
                            gestes.push(Geste::Bouton(i as u32, bas));
                        }
                        let p = ui.painter();
                        let allume = temoins[i];
                        let fond = if bas { TRAIT } else { FOND };
                        p.rect(r, 3.0, fond, Stroke::new(1.0_f32, if allume { ACCENT } else { TRAIT }), egui::StrokeKind::Inside);
                        p.text(r.center(), egui::Align2::CENTER_CENTER, BOUTONS[i], FontId::proportional(10.5), if allume { ACCENT } else { TEXTE });
                    }
                }
            });

            if !gestes.is_empty() {
                l.partage.lock().unwrap().gestes.extend(gestes);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        },
    )
}
