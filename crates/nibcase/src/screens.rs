//! SDL owns both panels throughout play. Textures and menu rasters are reused across frames.
use crate::unit::Part;
use nibcase_libretro::{Frame, SharedFrame};
use nibkit::menu::indicators::Indicators;
use nibkit::menu::raster::{Canvas as Raster, Fonts};
use nibkit::menu::toast;
use nibkit::menu::{Draw, Status};
use nibkit::screens::{self, Rect as Region};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, ScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
/// A picture laid over the game on the top panel in play (RetroAchievements' toast, its
/// indicators): its texture, where it goes on the canvas, whether it is up, and the serial of
/// what it shows (nibkit's), to upload it again only when that changes.
#[derive(Default)]
struct Overlay {
    image: Option<Image>,
    at: Option<Rect>,
    serial: u64,
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
    /// Over the game: the toast on show, bottom right, and the indicators, bottom left.
    toast: Overlay,
    indicators: Overlay,
    /// The volume and brightness indicator (nibkit's `menu::level`, every app's), over the
    /// game at the bottom right whether or not RetroAchievements is on: what nibOS announces
    /// of the volume keys, the indicator's timing, and its picture (its serial the level and
    /// the fade's step). A toast moves up above it while both show.
    levels: Option<nibkit::levels::LevelWatch>,
    level: nibkit::menu::level::Indicator,
    level_overlay: Overlay,
    /// A texture of the top panel's renderer a GPU core's frame is copied into on the GPU
    /// (`play_hw`).
    hw: Option<Hw>,
    /// Time spent presenting the panels (waiting for their refresh, with vsync) since
    /// [`Screens::take_presenting`] last asked.
    presenting: Duration,
}
/// Presents a panel, adding the time it took (its wait for the refresh) to `spent`.
fn swap(c: &mut WindowCanvas, spent: &mut Duration) {
    let at = Instant::now();
    c.present();
    *spent += at.elapsed();
}
/// The top panel's texture a GPU core's frames are copied into, the size of its framebuffer.
struct Hw {
    texture: Texture,
    size: (u32, u32),
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
/// Uploads a picture laid over the game (straight alpha, blended) to `overlay`, up at `at`.
fn lay(
    creator: &TextureCreator<WindowContext>,
    overlay: &mut Overlay,
    rgba: &[u8],
    at: Rect,
    serial: u64,
) -> Result<(), String> {
    let picture = Frame {
        width: at.width(),
        height: at.height(),
        rgba: rgba.to_vec(),
    };
    upload(creator, &mut overlay.image, &picture)?;
    if let Some(image) = &mut overlay.image {
        image.texture.set_blend_mode(BlendMode::Blend);
    }
    overlay.at = Some(at);
    overlay.serial = serial;
    Ok(())
}
/// `at` moved `up` pixels.
fn raised(at: Rect, up: i32) -> Rect {
    Rect::new(at.x(), at.y() - up, at.width(), at.height())
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
            toast: Overlay::default(),
            indicators: Overlay::default(),
            levels: nibkit::levels::watch(),
            level: nibkit::menu::level::Indicator::new(),
            level_overlay: Overlay::default(),
            hw: None,
            presenting: Duration::ZERO,
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
    /// The time spent presenting since it was last asked (`NIBCASE_DEBUG=frames`).
    pub fn take_presenting(&mut self) -> Duration {
        std::mem::take(&mut self.presenting)
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
            swap(c, &mut self.presenting);
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
        self.finish_top()
    }
    /// Whether frames can be drawn from the GPU: not with SDL's dummy driver.
    pub fn gpu(&self) -> bool {
        !self.dummy
    }
    /// The top panel's renderer's GL context made current, for a GPU core's context to be made
    /// in its share group (`nibcase_libretro::share_with_current_context`): SDL makes it
    /// current to bind one of its textures.
    pub fn top_gl_current(&mut self) -> Result<(), String> {
        let mut texture = self.creators[self.top.canvas]
            .create_texture_static(PixelFormatEnum::RGBA32, 1, 1)
            .map_err(|e| e.to_string())?;
        // SAFETY: binds and unbinds the renderer's own texture on its context, then destroys
        // it; the context stays current.
        unsafe {
            texture.gl_bind_texture();
            texture.gl_unbind_texture();
            texture.destroy();
        }
        Ok(())
    }
    /// A GPU core's frame, `size` at the corner of its framebuffer (the GL way up), copied on
    /// the GPU into a texture of the top panel's renderer and drawn as `play` draws a frame.
    pub fn play_hw(
        &mut self,
        frame: &SharedFrame,
        size: (u32, u32),
        aspect: f32,
        fit: bool,
        tv: bool,
    ) -> Result<(), String> {
        if self.hw.as_ref().is_none_or(|h| h.size != frame.size) {
            if let Some(old) = self.hw.take() {
                // SAFETY: uniquely owned; its renderer is alive on this thread.
                unsafe { old.texture.destroy() };
            }
            let mut texture = self.creators[self.top.canvas]
                .create_texture_static(PixelFormatEnum::RGBA32, frame.size.0, frame.size.1)
                .map_err(|e| e.to_string())?;
            texture.set_blend_mode(BlendMode::None);
            eprintln!(
                "nibcase: drawing frames from the GPU ({} x {}, copied on it)",
                frame.size.0, frame.size.1
            );
            self.hw = Some(Hw {
                texture,
                size: frame.size,
            });
        }
        let hw = self.hw.as_mut().unwrap();
        // SAFETY: binds the renderer's own texture on its context, current until the unbind;
        // the copy writes into it there.
        let copied = unsafe {
            hw.texture.gl_bind_texture();
            let copied = nibcase_libretro::copy_shared_frame(frame, size.0, size.1);
            hw.texture.gl_unbind_texture();
            copied
        };
        copied?;
        hw.texture.set_scale_mode(if self.smooth {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        });
        let shape = Frame {
            width: size.0,
            height: size.1,
            rgba: Vec::new(),
        };
        let v = self.top;
        let dst = on(v, destination(&shape, aspect, fit, tv));
        let c = &mut self.canvases[v.canvas];
        c.set_clip_rect(Some(native(v.rect)));
        c.set_draw_color(Color::BLACK);
        c.fill_rect(native(v.rect))?;
        // The frame's bottom row is the buffer's first: drawn turned the right way up.
        c.copy_ex(
            &hw.texture,
            Rect::new(0, 0, size.0, size.1),
            dst,
            0.0,
            None,
            false,
            true,
        )?;
        self.finish_top()
    }
    /// The overlays over the game on the top panel, the bottom panel, and both shown.
    fn finish_top(&mut self) -> Result<(), String> {
        self.level_tick()?;
        let v = self.top;
        let lift = self.toast_lift();
        let c = &mut self.canvases[v.canvas];
        for (o, up) in [
            (&self.indicators, 0),
            (&self.toast, lift),
            (&self.level_overlay, 0),
        ] {
            if let (Some(image), Some(at)) = (&o.image, o.at) {
                c.copy(&image.texture, None, raised(at, up))?;
            }
        }
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
                swap(c, &mut self.presenting);
            }
            self.bottom_blank = true;
        }
        if self.shot.is_some() {
            self.present()
        } else {
            swap(&mut self.canvases[self.top.canvas], &mut self.presenting);
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
        self.level_tick()?;
        let lift = self.toast_lift();
        let f = |r: (u32, u32, u32, u32)| (r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
        for (v, part, image) in [(self.top, lid, &self.game), (self.bottom, body, &self.body)] {
            let c = &mut self.canvases[v.canvas];
            c.set_clip_rect(Some(native(v.rect)));
            c.set_draw_color(Color::BLACK);
            c.fill_rect(native(v.rect))?;
            c.copy(&image.as_ref().unwrap().texture, None, on(v, f(part.dst)))?;
            if v.canvas == self.top.canvas && v.rect == self.top.rect {
                for (o, up) in [
                    (&self.indicators, 0),
                    (&self.toast, lift),
                    (&self.level_overlay, 0),
                ] {
                    if let (Some(image), Some(at)) = (&o.image, o.at) {
                        c.copy(&image.texture, None, raised(at, up))?;
                    }
                }
            }
            c.set_clip_rect(None);
        }
        // The bottom panel is redrawn every frame now: a menu shown after this redraws it.
        self.bottom_blank = false;
        if self.shot.is_some() || self.top.canvas == self.bottom.canvas {
            self.present()
        } else {
            swap(&mut self.canvases[self.bottom.canvas], &mut self.presenting);
            swap(&mut self.canvases[self.top.canvas], &mut self.presenting);
            Ok(())
        }
    }
    /// RetroAchievements' toast and indicators over the game in play, as `toasts` and
    /// `indicators` have them at `now` (ticked here, once a frame); drawn at the top panel's
    /// scale and uploaded only when what they show changes.
    pub fn overlays(
        &mut self,
        toasts: &mut toast::Queue,
        indicators: &mut Indicators,
        now: Instant,
    ) -> Result<(), String> {
        let v = self.top;
        let frame = (v.rect.w as usize, v.rect.h as usize);
        let creator = &self.creators[v.canvas];
        let place = |(x, y): (usize, usize), w: usize, h: usize| {
            Rect::new(v.rect.x + x as i32, v.rect.y + y as i32, w as u32, h as u32)
        };
        match toasts.tick(now).map(toast::Shown::serial) {
            None => self.toast.at = None,
            Some(serial) if serial != self.toast.serial || self.toast.at.is_none() => {
                let scale = nibkit::menu::scale_for(frame.0, frame.1);
                if let Some(t) = toasts.draw(&self.fonts, scale) {
                    let at = place(t.place(frame), t.width(), t.height());
                    lay(creator, &mut self.toast, t.rgba(), at, serial)?;
                }
            }
            Some(_) => {}
        }
        match indicators.tick(now) {
            0 => self.indicators.at = None,
            serial if serial != self.indicators.serial || self.indicators.at.is_none() => {
                if let Some(stack) = indicators.draw(&self.fonts, frame) {
                    let at = place(stack.place(), stack.width(), stack.height());
                    lay(creator, &mut self.indicators, stack.rgba(), at, serial)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    /// The volume and brightness indicator at this frame: a level nibOS announced shows from
    /// now; drawn at the top panel's scale, its opacity in its alpha, and uploaded again only
    /// when the level or the fade's step changes.
    fn level_tick(&mut self) -> Result<(), String> {
        let now = Instant::now();
        if let Some(level) = self.levels.as_mut().and_then(|w| w.changed()) {
            self.level.show(level, now);
        }
        let opacity = self.level.tick(now);
        let Some(level) = self.level.level().filter(|_| opacity > 0.0) else {
            self.level_overlay.at = None;
            return Ok(());
        };
        let step = (opacity * 32.0).round() as u64;
        let what = matches!(level.what, nibkit::levels::What::Brightness) as u64;
        let serial = 1 + step + (what << 8) + (u64::from(level.percent) << 9);
        if serial == self.level_overlay.serial && self.level_overlay.at.is_some() {
            return Ok(());
        }
        let v = self.top;
        let frame = (v.rect.w as usize, v.rect.h as usize);
        let scale = nibkit::menu::scale_for(frame.0, frame.1);
        let Some(panel) = self.level.draw(&self.fonts, scale) else {
            return Ok(());
        };
        let ((x, y), w, h) = (panel.place(frame), panel.width(), panel.height());
        let at = Rect::new(v.rect.x + x as i32, v.rect.y + y as i32, w as u32, h as u32);
        if let Some(px) = self.level.faded(&self.fonts, scale, opacity) {
            let px = px.to_vec();
            lay(
                &self.creators[v.canvas],
                &mut self.level_overlay,
                &px,
                at,
                serial,
            )?;
        }
        Ok(())
    }

    /// How far up the toast goes while the level indicator shows in its corner, in pixels.
    fn toast_lift(&self) -> i32 {
        if self.level_overlay.at.is_none() {
            return 0;
        }
        let v = self.top;
        let scale = nibkit::menu::scale_for(v.rect.w as usize, v.rect.h as usize);
        self.level.lift(scale) as i32
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
            &mut self.toast.image,
            &mut self.indicators.image,
            &mut self.level_overlay.image,
        ] {
            if let Some(image) = image.take() {
                // SAFETY: textures are uniquely owned and all renderers outlive this Drop method.
                unsafe { image.texture.destroy() };
            }
        }
        if let Some(hw) = self.hw.take() {
            // SAFETY: as above; its EGL image goes with it.
            unsafe { hw.texture.destroy() };
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
