//! SDL owns both panels throughout play. Textures and menu rasters are reused across frames.
use crate::unit::Part;
use nibcase_libretro::Frame;
use nibkit::menu::raster::{Canvas as Raster, Fonts};
use nibkit::menu::{Draw, Status};
use nibkit::screens::{self, Rect as Region};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, ScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct View {
    canvas: usize,
    rect: Region,
    window: u32,
}
struct Image {
    texture: Texture,
    width: u32,
    height: u32,
}
pub struct Screens {
    pub sdl: sdl2::Sdl,
    _video: sdl2::VideoSubsystem,
    canvases: Vec<WindowCanvas>,
    creators: Vec<TextureCreator<WindowContext>>,
    top: View,
    bottom: View,
    game: Option<Image>,
    menu: Option<Image>,
    paused: Option<Image>,
    fonts: Fonts,
    off: bool,
    dummy: bool,
    shot: Option<PathBuf>,
    bottom_blank: bool,
    /// The part of a cut unit on the bottom panel (the top's is in `game`).
    body: Option<Image>,
    /// What the bottom panel shows in play instead of black (a unit's touch buttons).
    card: Option<Image>,
    /// Scale the game smoothly: a photograph of a unit, not a raster of pixels.
    pub smooth: bool,
}
fn upload(
    creator: &TextureCreator<WindowContext>,
    image: &mut Option<Image>,
    frame: &Frame,
) -> Result<(), String> {
    let all = (0, 0, frame.width, frame.height);
    upload_part(creator, image, frame, all, false)
}
/// Uploads the `src` rectangle of `frame` into `image`, made (again) at that size when needed.
fn upload_part(
    creator: &TextureCreator<WindowContext>,
    image: &mut Option<Image>,
    frame: &Frame,
    src: (u32, u32, u32, u32),
    smooth: bool,
) -> Result<(), String> {
    let (sx, sy, width, height) = src;
    if image
        .as_ref()
        .is_some_and(|i| i.width != width || i.height != height)
    {
        if let Some(i) = image.take() {
            // SAFETY: this texture is uniquely owned and its renderer is still alive on this thread.
            unsafe { i.texture.destroy() };
        }
    }
    if image.is_none() {
        let mut texture = creator
            .create_texture_streaming(PixelFormatEnum::RGBA32, width, height)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::None);
        *image = Some(Image {
            texture,
            width,
            height,
        });
    }
    let texture = &mut image.as_mut().unwrap().texture;
    texture.set_scale_mode(if smooth {
        ScaleMode::Linear
    } else {
        ScaleMode::Nearest
    });
    texture
        .update(
            None,
            &frame.rgba[(sy as usize * frame.width as usize + sx as usize) * 4..],
            frame.width as usize * 4,
        )
        .map_err(|e| e.to_string())
}
fn native(rect: Region) -> Rect {
    Rect::new(rect.x, rect.y, rect.w, rect.h)
}
/// A box in a panel's 1024 x 768 units, on the panel `v` as it is on screen.
fn on(v: View, (x, y, w, h): (f32, f32, f32, f32)) -> Rect {
    let (sx, sy) = (v.rect.w as f32 / 1024.0, v.rect.h as f32 / 768.0);
    Rect::new(
        v.rect.x + (x * sx).round() as i32,
        v.rect.y + (y * sy).round() as i32,
        (w * sx).round() as u32,
        (h * sy).round() as u32,
    )
}
pub fn destination(frame: &Frame, aspect: f32, fit: bool, tv: bool) -> (f32, f32, f32, f32) {
    let (w, h) = (frame.width.max(1) as f32, frame.height.max(1) as f32);
    let ratio = if tv && aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        w / h
    };
    let pixel_width = ratio * h;
    let scale = (1024.0 / pixel_width).min(768.0 / h);
    let scale = if fit { scale } else { scale.floor().max(1.0) };
    let mut dw = pixel_width * scale;
    let mut dh = h * scale;
    if dw > 1024.0 || dh > 768.0 {
        let k = (1024.0 / dw).min(768.0 / dh);
        dw *= k;
        dh *= k;
    }
    ((1024.0 - dw) / 2.0, (768.0 - dh) / 2.0, dw, dh)
}
impl Screens {
    pub fn open() -> Result<Self, String> {
        if std::env::var_os("SDL_KMSDRM_ATOMIC").is_none() {
            std::env::set_var("SDL_KMSDRM_ATOMIC", "0");
        }
        let dummy = std::env::var("SDL_VIDEODRIVER").is_ok_and(|s| s == "dummy");
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let bounds: Vec<_> = (0..video.num_video_displays()?)
            .filter_map(|i| video.display_bounds(i).ok())
            .map(|r| Region::new(r.x(), r.y(), r.width(), r.height()))
            .collect();
        let mut canvases = Vec::new();
        let mut views = Vec::new();
        if let Some(p) = screens::place(&bounds, false) {
            if p.shared {
                let (t, b) = (p.top.rect, p.bottom.rect);
                let (x, y) = (t.x.min(b.x), t.y.min(b.y));
                let (w, h) = (t.w + b.w, t.h.max(b.h));
                let window = video
                    .window("nibcase", w, h)
                    .position(x, y)
                    .borderless()
                    .opengl()
                    .build()
                    .map_err(|e| e.to_string())?;
                let window_id = window.id();
                let canvas = window
                    .into_canvas()
                    .accelerated()
                    .present_vsync()
                    .build()
                    .map_err(|e| e.to_string())?;
                canvases.push(canvas);
                for r in [t, b] {
                    views.push(View {
                        canvas: 0,
                        rect: Region::new(r.x - x, r.y - y, r.w, r.h),
                        window: window_id,
                    });
                }
            } else {
                for (index, r) in [p.top.rect, p.bottom.rect].into_iter().enumerate() {
                    let window = video
                        .window("nibcase", r.w, r.h)
                        .position(r.x, r.y)
                        .fullscreen_desktop()
                        .opengl()
                        .build()
                        .map_err(|e| e.to_string())?;
                    let window_id = window.id();
                    let builder = window.into_canvas().accelerated();
                    let builder = if index == 0 {
                        builder.present_vsync()
                    } else {
                        builder
                    };
                    canvases.push(builder.build().map_err(|e| e.to_string())?);
                    views.push(View {
                        canvas: index,
                        rect: Region::new(0, 0, r.w, r.h),
                        window: window_id,
                    });
                }
            }
        } else {
            let size = if dummy { (1024, 1536) } else { (512, 768) };
            let window = video
                .window("nibcase", size.0, size.1)
                .position_centered()
                .build()
                .map_err(|e| e.to_string())?;
            let window_id = window.id();
            let builder = window.into_canvas();
            let canvas = if dummy {
                builder.software()
            } else {
                builder.accelerated().present_vsync()
            }
            .build()
            .map_err(|e| e.to_string())?;
            canvases.push(canvas);
            for y in [0, size.1 / 2] {
                views.push(View {
                    canvas: 0,
                    rect: Region::new(0, y as i32, size.0, size.1 / 2),
                    window: window_id,
                });
            }
        }
        for c in &mut canvases {
            c.set_blend_mode(BlendMode::Blend);
            c.set_draw_color(Color::BLACK);
            c.clear();
        }
        let creators = canvases.iter().map(|c| c.texture_creator()).collect();
        sdl.mouse().show_cursor(!nibkit::on_device());
        Ok(Self {
            sdl,
            _video: video,
            canvases,
            creators,
            top: views[0],
            bottom: views[1],
            game: None,
            menu: None,
            paused: None,
            fonts: Fonts::new(),
            off: false,
            dummy,
            shot: None,
            bottom_blank: false,
            body: None,
            card: None,
            smooth: false,
        })
    }
    pub fn set_sync(&mut self, on: bool) {
        if self.dummy {
            return;
        }
        // SAFETY: the renderer is live, owned by this thread; SDL selects the GL swap interval.
        let result = unsafe {
            sdl2::sys::SDL_RenderSetVSync(self.canvases[self.top.canvas].raw(), i32::from(on))
        };
        if result != 0 {
            eprintln!("nibcase: vsync: {}", sdl2::get_error());
        }
    }
    pub fn bottom_power(&mut self, off: bool) {
        if self.off != off {
            nibkit::backlight::set_off(screens::Screen::Bottom, off);
            self.off = off;
        }
    }
    pub fn touch(&self, window: u32, x: f32, y: f32, normalized: bool) -> Option<(f32, f32)> {
        if window != 0 && window != self.bottom.window {
            return None;
        }
        if normalized {
            return Some((x * 1024.0, y * 768.0));
        }
        self.bottom
            .rect
            .panel_point(x.round() as i32, y.round() as i32)
    }
    fn clear(&mut self) {
        self.bottom_blank = false;
        for c in &mut self.canvases {
            c.set_clip_rect(None);
            c.set_draw_color(Color::BLACK);
            c.clear();
        }
    }
    pub fn capture_next(&mut self, path: PathBuf) {
        self.shot = Some(path);
        // A panel drawn once and left shows its picture, but its back buffer, which a
        // screenshot reads, no longer holds it: draw it again for the shot.
        self.bottom_blank = false;
    }
    fn present(&mut self) -> Result<(), String> {
        if let Some(path) = self.shot.take() {
            self.screenshot(&path)?;
        }
        for c in &mut self.canvases {
            c.present();
        }
        Ok(())
    }
    fn raster(&self, draws: &[Draw], picture: &Frame) -> Frame {
        let mut canvas = Raster::new(1.0);
        canvas.draw(&self.fonts, draws, |index| {
            (index == 0 && picture.width > 0).then_some((
                picture.rgba.as_slice(),
                picture.width as usize,
                picture.height as usize,
            ))
        });
        Frame {
            width: canvas.w as u32,
            height: canvas.h as u32,
            rgba: canvas.px,
        }
    }
    pub fn picker(&mut self, menu: &nibkit::menu::Menu, status: &Status) -> Result<(), String> {
        self.clear();
        let frame = self.raster(&nibkit::menu::layout::menu(menu, status), &Frame::default());
        upload(&self.creators[self.bottom.canvas], &mut self.menu, &frame)?;
        let image = self.menu.as_ref().unwrap();
        self.canvases[self.bottom.canvas].copy(&image.texture, None, native(self.bottom.rect))?;
        self.present()
    }
    pub fn play(&mut self, frame: &Frame, aspect: f32, fit: bool, tv: bool) -> Result<(), String> {
        if frame.width == 0 {
            return Ok(());
        }
        let all = (0, 0, frame.width, frame.height);
        upload_part(
            &self.creators[self.top.canvas],
            &mut self.game,
            frame,
            all,
            self.smooth,
        )?;
        let v = self.top;
        let dst = on(v, destination(frame, aspect, fit, tv));
        let c = &mut self.canvases[v.canvas];
        c.set_clip_rect(Some(native(v.rect)));
        c.set_draw_color(Color::BLACK);
        c.fill_rect(native(v.rect))?;
        c.copy(&self.game.as_ref().unwrap().texture, None, dst)?;
        // The bottom panel: the card if there is one, else black; on a panel of its own it is
        // drawn once and left, never redrawn in play.
        let b = self.bottom;
        if self.top.canvas == b.canvas {
            c.set_clip_rect(None);
            match &self.card {
                Some(card) => c.copy(&card.texture, None, native(b.rect))?,
                None => c.fill_rect(native(b.rect))?,
            }
        } else if !self.bottom_blank {
            let c = &mut self.canvases[b.canvas];
            c.set_draw_color(Color::BLACK);
            c.clear();
            if let Some(card) = &self.card {
                c.copy(&card.texture, None, native(b.rect))?;
            }
            // A screenshot reads both panels before presenting them, below.
            if self.shot.is_none() {
                c.present();
            }
            self.bottom_blank = true;
        }
        if self.shot.is_some() {
            self.present()
        } else {
            self.canvases[self.top.canvas].present();
            Ok(())
        }
    }
    /// A unit cut at its hinge: `lid` of `frame` on the top panel, `body` on the bottom one.
    pub fn play_split(&mut self, frame: &Frame, lid: Part, body: Part) -> Result<(), String> {
        if frame.width == 0 {
            return Ok(());
        }
        let smooth = self.smooth;
        upload_part(
            &self.creators[self.top.canvas],
            &mut self.game,
            frame,
            lid.src,
            smooth,
        )?;
        upload_part(
            &self.creators[self.bottom.canvas],
            &mut self.body,
            frame,
            body.src,
            smooth,
        )?;
        let f = |r: (u32, u32, u32, u32)| (r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
        for (v, part, image) in [(self.top, lid, &self.game), (self.bottom, body, &self.body)] {
            let c = &mut self.canvases[v.canvas];
            c.set_clip_rect(Some(native(v.rect)));
            c.set_draw_color(Color::BLACK);
            c.fill_rect(native(v.rect))?;
            c.copy(&image.as_ref().unwrap().texture, None, on(v, f(part.dst)))?;
            c.set_clip_rect(None);
        }
        // The bottom panel is redrawn every frame now: a menu shown after this redraws it.
        self.bottom_blank = false;
        if self.shot.is_some() || self.top.canvas == self.bottom.canvas {
            self.present()
        } else {
            self.canvases[self.bottom.canvas].present();
            self.canvases[self.top.canvas].present();
            Ok(())
        }
    }
    /// What the bottom panel shows in play from now on, drawn from `draws`; `None` for black.
    pub fn set_card(&mut self, draws: Option<&[Draw]>) -> Result<(), String> {
        match draws {
            Some(draws) => {
                let card = self.raster(draws, &Frame::default());
                upload(&self.creators[self.bottom.canvas], &mut self.card, &card)?;
            }
            None => {
                if let Some(card) = self.card.take() {
                    // SAFETY: uniquely owned, and its renderer is alive on this thread.
                    unsafe { card.texture.destroy() };
                }
            }
        }
        // Drawn at the next frame.
        self.bottom_blank = false;
        Ok(())
    }
    pub fn pause(
        &mut self,
        model: &crate::menu::Menu,
        status: &Status,
        frame: &Frame,
        title: &str,
    ) -> Result<(), String> {
        self.clear();
        let top = if model.page == 1 {
            let state = model.states.get(model.cards.at).and_then(Option::as_ref);
            let picture = state.map(|s| &s.picture).unwrap_or(frame);
            let label = if model.cards.at == 0 {
                "Resume point".into()
            } else {
                format!("Slot {}", model.cards.at)
            };
            let details = state
                .map(|s| format!("{} seconds played", s.metadata.play_seconds))
                .unwrap_or_else(|| "Empty slot".into());
            self.raster(
                &nibkit::menu::top::state_measured(
                    &label,
                    &details,
                    &[],
                    (picture.width > 0).then_some(0),
                    |s, size, bold| self.fonts.width(s, size, bold),
                ),
                picture,
            )
        } else {
            let mut draws = Vec::new();
            let (x, y, w, h) = destination(frame, 4.0 / 3.0, false, false);
            if frame.width > 0 {
                draws.push(Draw::Picture {
                    index: 0,
                    x,
                    y,
                    w,
                    h,
                    faded: true,
                });
            }
            // The game's name as DSlot shows it, fitted to the screen (a disc game's can be long).
            draws.extend(nibkit::menu::top::paused_measured(
                &nibkit::names::display_name(title),
                "Paused",
                |s, size, bold| self.fonts.width(s, size, bold),
            ));
            self.raster(&draws, frame)
        };
        upload(&self.creators[self.top.canvas], &mut self.paused, &top)?;
        self.canvases[self.top.canvas].copy(
            &self.paused.as_ref().unwrap().texture,
            None,
            native(self.top.rect),
        )?;
        let draws = if model.page == 1 {
            nibkit::menu::layout::shelf(&model.cards, status)
        } else {
            nibkit::menu::layout::menu(&model.rows, status)
        };
        let mut raster = Raster::new(1.0);
        raster.draw(&self.fonts, &draws, |index| model.picture(index));
        let bottom = Frame {
            width: raster.w as u32,
            height: raster.h as u32,
            rgba: raster.px,
        };
        upload(&self.creators[self.bottom.canvas], &mut self.menu, &bottom)?;
        self.canvases[self.bottom.canvas].copy(
            &self.menu.as_ref().unwrap().texture,
            None,
            native(self.bottom.rect),
        )?;
        self.present()
    }
    pub fn screenshot(&self, path: &Path) -> Result<(), String> {
        let mut pixels = Vec::new();
        let mut width = 0;
        let mut height = 0;
        for v in [self.top, self.bottom] {
            let bytes = self.canvases[v.canvas]
                .read_pixels(Some(native(v.rect)), PixelFormatEnum::RGBA32)?;
            if width != 0 && width != v.rect.w {
                return Err("screens have different screenshot widths".into());
            }
            width = v.rect.w;
            height += v.rect.h;
            pixels.extend(bytes);
        }
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&pixels)
            .map_err(|e| e.to_string())
    }
}
impl Drop for Screens {
    fn drop(&mut self) {
        self.bottom_power(false);
        for image in [
            &mut self.game,
            &mut self.menu,
            &mut self.paused,
            &mut self.body,
            &mut self.card,
        ] {
            if let Some(image) = image.take() {
                // SAFETY: textures are uniquely owned and all renderers outlive this Drop method.
                unsafe { image.texture.destroy() };
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_scaling_and_tv_shape() {
        let f = Frame {
            width: 256,
            height: 224,
            rgba: Vec::new(),
        };
        assert_eq!(
            destination(&f, 4.0 / 3.0, false, false),
            (128.0, 48.0, 768.0, 672.0)
        );
        let d = destination(&f, 4.0 / 3.0, false, true);
        assert!((d.2 - 896.0).abs() < 0.01);
        assert_eq!(d.3, 672.0);
    }
}
