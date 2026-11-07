//! Hardware-rendered cores (libretro's `SET_HW_RENDER`): an OpenGL ES (or OpenGL) context of the
//! core's own, made through EGL with no window (Mesa's surfaceless platform, the GPU's render
//! node), a framebuffer the core draws each frame into, and the frame read back into a [`Frame`]
//! as a software core's is, so the player, the pause picture and the states' thumbnails need
//! nothing new.
//!
//! EGL and GL are loaded when a core asks for them, not linked: nibcase runs without a GPU for
//! every core that doesn't. The core's context is current only while the core runs (around every
//! call into it); whatever was current before (SDL's renderer's) is put back after, so SDL never
//! draws into the core's context nor the core into SDL's.
//!
//! Frames come back through two pixel-pack buffers: frame N is read into one while frame N-1,
//! read into the other a frame ago and finished by the GPU meanwhile, is copied out. The picture
//! is a frame late and the CPU never waits for the GPU to finish drawing. A context without
//! pixel-pack buffers (OpenGL ES 2) reads synchronously.

use crate::ffi::*;
use crate::Frame;
use libloading::Library;
use std::ffi::{c_char, c_void, CStr};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

type Egl = *mut c_void;

/// `eglGetProcAddress`, once EGL is loaded: what the core's `get_proc_address` answers with,
/// from any thread. EGL stays loaded while a context lives (one core a process).
static GET_PROC: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

/// The framebuffer the core draws into, for its `get_current_framebuffer`: asked from any thread
/// too (PPSSPP asks from its emulation thread).
static FRAMEBUFFER: AtomicUsize = AtomicUsize::new(0);

/// Make the core's context in the share group of whatever context is current when the core asks
/// for one (the player's top screen's renderer), rather than on a display of its own.
static SHARE_WITH_CURRENT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// The core's context made, from now on, in the share group of the context current when the
/// core asks for one: its colour buffer is then a texture that context can copy from
/// ([`copy_shared_frame`]), with no buffer exported to the display. For the player, which makes
/// its top screen's renderer current before loading a game.
pub fn share_with_current_context(on: bool) {
    SHARE_WITH_CURRENT.store(on, Ordering::Release);
}

/// The core's framebuffer (0, the default, before there is one).
pub(crate) fn current_framebuffer() -> usize {
    FRAMEBUFFER.load(Ordering::Acquire)
}

/// A GL function, by name, or `None` before EGL is loaded or for a name it doesn't know.
pub(crate) fn proc_address(name: *const c_char) -> retro_proc_address_t {
    let get = GET_PROC.load(Ordering::Acquire);
    if name.is_null() || get.is_null() {
        return None;
    }
    // SAFETY: GET_PROC holds eglGetProcAddress while EGL is loaded; the name is the core's,
    // NUL-terminated. The function (or NULL, as None) is cast by the core to its own type.
    unsafe {
        let get: unsafe extern "C" fn(*const c_char) -> *mut c_void = std::mem::transmute(get);
        std::mem::transmute::<*mut c_void, retro_proc_address_t>(get(name))
    }
}

const EGL_NONE: i32 = 0x3038;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_OPENGL_ES2_BIT: i32 = 0x4;
const EGL_OPENGL_ES3_BIT: i32 = 0x40;
const EGL_OPENGL_BIT: i32 = 0x8;
const EGL_CONTEXT_MAJOR_VERSION: i32 = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: i32 = 0x30FB;
const EGL_CONTEXT_OPENGL_PROFILE_MASK: i32 = 0x30FD;
const EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT: i32 = 0x1;
const EGL_CONTEXT_OPENGL_COMPATIBILITY_PROFILE_BIT: i32 = 0x2;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_OPENGL_API: u32 = 0x30A2;
const EGL_PLATFORM_SURFACELESS_MESA: u32 = 0x31DD;
const EGL_DRAW: i32 = 0x3059;
const EGL_READ: i32 = 0x305A;

const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_RENDERBUFFER: u32 = 0x8D41;
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_RGBA: u32 = 0x1908;
const GL_RGBA8: u32 = 0x8058;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_DEPTH_ATTACHMENT: u32 = 0x8D00;
const GL_DEPTH_STENCIL_ATTACHMENT: u32 = 0x821A;
const GL_DEPTH24_STENCIL8: u32 = 0x88F0;
const GL_DEPTH_COMPONENT24: u32 = 0x81A6;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
const GL_TEXTURE_BINDING_2D: u32 = 0x8069;
const GL_RENDERBUFFER_BINDING: u32 = 0x8CA7;
const GL_PIXEL_PACK_BUFFER: u32 = 0x88EB;
const GL_PIXEL_PACK_BUFFER_BINDING: u32 = 0x88ED;
const GL_STREAM_READ: u32 = 0x88E1;
const GL_MAP_READ_BIT: u32 = 0x1;
const GL_SYNC_GPU_COMMANDS_COMPLETE: u32 = 0x9117;
const GL_SYNC_FLUSH_COMMANDS_BIT: u32 = 0x1;
const GL_WAIT_FAILED: u32 = 0x911D;
const GL_PACK_ALIGNMENT: u32 = 0x0D05;
const GL_PACK_ROW_LENGTH: u32 = 0x0D02;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_NEAREST: i32 = 0x2600;
const GL_VERSION: u32 = 0x1F02;
const GL_RENDERER: u32 = 0x1F01;

/// What the core asked for, as EGL makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Kind {
    /// OpenGL ES rather than OpenGL.
    pub es: bool,
    pub major: i32,
    pub minor: i32,
    /// OpenGL's core profile.
    pub core: bool,
}

impl Kind {
    /// The context a `retro_hw_render_callback` asks for, or `None` for one EGL can't make
    /// (Vulkan, Direct3D).
    pub fn of(hw: &retro_hw_render_callback) -> Option<Kind> {
        const OPENGL: u32 = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGL;
        const OPENGLES2: u32 = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGLES2;
        const OPENGL_CORE: u32 = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGL_CORE;
        const OPENGLES3: u32 = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGLES3;
        const OPENGLES_VERSION: u32 = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGLES_VERSION;
        let (major, minor) = (hw.version_major as i32, hw.version_minor as i32);
        match hw.context_type {
            OPENGLES2 => Some(Kind {
                es: true,
                major: 2,
                minor: 0,
                core: false,
            }),
            OPENGLES3 => Some(Kind {
                es: true,
                major: 3,
                minor: 0,
                core: false,
            }),
            OPENGLES_VERSION => Some(Kind {
                es: true,
                major: major.max(2),
                minor,
                core: false,
            }),
            OPENGL => Some(Kind {
                es: false,
                major: 2,
                minor: 1,
                core: false,
            }),
            OPENGL_CORE => Some(Kind {
                es: false,
                major: major.max(3),
                minor,
                core: true,
            }),
            _ => None,
        }
    }
}

macro_rules! fns {
    ($doc:literal $struct:ident { $( $name:ident : fn($($arg:ty),*) $(-> $ret:ty)? ;)* }) => {
        #[doc = $doc]
        #[allow(non_snake_case)]
        struct $struct { $( $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?, )* }
        impl $struct {
            /// Each function by name through `eglGetProcAddress` (core functions too: Mesa's EGL
            /// has EGL_KHR_get_all_proc_addresses).
            unsafe fn load(get: unsafe extern "C" fn(*const c_char) -> *mut c_void)
                -> Result<$struct, String> {
                Ok($struct { $( $name: {
                    let name = concat!(stringify!($name), "\0");
                    // SAFETY: a NUL-terminated name; EGL returns the function or NULL.
                    let f = unsafe { get(name.as_ptr().cast()) };
                    if f.is_null() {
                        return Err(format!("no {}", stringify!($name)));
                    }
                    // SAFETY: the named function has the signature the GL headers give it.
                    unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn($($arg),*) $(-> $ret)?>(f) }
                }, )* })
            }
        }
    };
}

fns! {
    "OpenGL ES 2's functions, all OpenGL's too, that the framebuffer and reading it need."
    Fns {
    glGetError: fn() -> u32;
    glGetString: fn(u32) -> *const u8;
    glGetIntegerv: fn(u32, *mut i32);
    glGenFramebuffers: fn(i32, *mut u32);
    glDeleteFramebuffers: fn(i32, *const u32);
    glBindFramebuffer: fn(u32, u32);
    glFramebufferTexture2D: fn(u32, u32, u32, u32, i32);
    glFramebufferRenderbuffer: fn(u32, u32, u32, u32);
    glCheckFramebufferStatus: fn(u32) -> u32;
    glGenTextures: fn(i32, *mut u32);
    glDeleteTextures: fn(i32, *const u32);
    glBindTexture: fn(u32, u32);
    glTexImage2D: fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void);
    glTexParameteri: fn(u32, u32, i32);
    glGenRenderbuffers: fn(i32, *mut u32);
    glDeleteRenderbuffers: fn(i32, *const u32);
    glBindRenderbuffer: fn(u32, u32);
    glRenderbufferStorage: fn(u32, u32, i32, i32);
    glPixelStorei: fn(u32, i32);
    glReadPixels: fn(i32, i32, i32, i32, u32, u32, *mut c_void);
    glFlush: fn();
    }
}

fns! {
    "The functions only OpenGL ES 3 / OpenGL 3 have: pixel-pack buffers, mapped, and fences."
    Async {
    glGenBuffers: fn(i32, *mut u32);
    glDeleteBuffers: fn(i32, *const u32);
    glBindBuffer: fn(u32, u32);
    glBufferData: fn(u32, isize, *const c_void, u32);
    glMapBufferRange: fn(u32, isize, isize, u32) -> *mut c_void;
    glUnmapBuffer: fn(u32) -> u8;
    glFenceSync: fn(u32, u32) -> *mut c_void;
    glClientWaitSync: fn(*mut c_void, u32, u64) -> u32;
    glDeleteSync: fn(*mut c_void);
    glWaitSync: fn(*mut c_void, u32, u64);
    glBlitFramebuffer: fn(i32, i32, i32, i32, i32, i32, i32, i32, u32, u32);
    }
}

#[allow(non_snake_case)]
struct EglFns {
    eglGetProcAddress: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    eglGetDisplay: unsafe extern "C" fn(Egl) -> Egl,
    eglInitialize: unsafe extern "C" fn(Egl, *mut i32, *mut i32) -> u32,
    eglTerminate: unsafe extern "C" fn(Egl) -> u32,
    eglBindAPI: unsafe extern "C" fn(u32) -> u32,
    eglChooseConfig: unsafe extern "C" fn(Egl, *const i32, *mut Egl, i32, *mut i32) -> u32,
    eglCreateContext: unsafe extern "C" fn(Egl, Egl, Egl, *const i32) -> Egl,
    eglDestroyContext: unsafe extern "C" fn(Egl, Egl) -> u32,
    eglMakeCurrent: unsafe extern "C" fn(Egl, Egl, Egl, Egl) -> u32,
    eglGetCurrentContext: unsafe extern "C" fn() -> Egl,
    eglGetCurrentDisplay: unsafe extern "C" fn() -> Egl,
    eglGetCurrentSurface: unsafe extern "C" fn(i32) -> Egl,
    eglQueryAPI: unsafe extern "C" fn() -> u32,
    eglGetError: unsafe extern "C" fn() -> i32,
}

/// What was current before the core's context was made so: put back after.
#[derive(Clone, Copy)]
struct Current {
    api: u32,
    display: Egl,
    draw: Egl,
    read: Egl,
    context: Egl,
}

/// Time spent reading frames back, over the frames since it was last logged (seconds).
#[derive(Default)]
struct Timing {
    frames: u32,
    /// The whole of `read`.
    total: f64,
    /// Asking for this frame's pixels (glReadPixels into a buffer, the fence, the flush).
    issue: f64,
    /// Waiting for the frame before to be finished.
    wait: f64,
    /// Mapping it and copying it out.
    copy: f64,
}

impl Timing {
    fn frame(&mut self) {
        self.frames += 1;
        if self.frames == 600 {
            let ms = |t: f64| t * 1000.0 / 600.0;
            eprintln!(
                "nibcase: gl: reading a frame back {:.2} ms: issuing {:.2}, waiting for the GPU {:.2}, copying {:.2}",
                ms(self.total),
                ms(self.issue),
                ms(self.wait),
                ms(self.copy)
            );
            *self = Timing::default();
        }
    }
}

/// One pixel-pack buffer and the fence after the read into it.
struct Pending {
    buffer: u32,
    fence: *mut c_void,
    width: u32,
    height: u32,
}

/// The core's context, its framebuffer and the frames on their way back.
pub(crate) struct Gl {
    egl: EglFns,
    gl: Fns,
    pbo: Option<Async>,
    kind: Kind,
    display: Egl,
    context: Egl,
    outer: Option<Current>,
    fbo: u32,
    color: u32,
    depth: u32,
    size: (u32, u32),
    want_depth: bool,
    want_stencil: bool,
    buffers: [u32; 2],
    buffer_bytes: usize,
    next: usize,
    pending: Option<Pending>,
    /// Shares a group with the context current when it was made (its display isn't ours to
    /// terminate), and the fence after the last frame drawn, for that context to wait on.
    shared: bool,
    frame_fence: *mut c_void,
    /// With `NIBCASE_DEBUG=gl`: where reading frames back goes, logged every 600 frames.
    timing: Option<Timing>,
    // Kept last: dropped after every function above is done with.
    _gles: Option<Library>,
    _lib: Library,
}

impl Gl {
    /// A context for a core that asked for `kind`, current when this returns (the caller's
    /// is put back by [`Gl::leave`]).
    pub fn new(kind: Kind, depth: bool, stencil: bool) -> Result<Gl, String> {
        let (lib, egl) = load_egl()?;
        // A GLES library loaded too, so its functions resolve for cores that link it themselves.
        // SAFETY: loading the system's GLES; its symbols are only reached through
        // eglGetProcAddress.
        let gles = unsafe { Library::new("libGLESv2.so.2") }.ok();
        // SAFETY: EGL calls with the arguments the EGL 1.5 specification gives them; every
        // handle passed is one EGL returned.
        unsafe {
            let outer = current(&egl);
            // In the share group of the context current now (the player's renderer), on its
            // display; or, for a program with none (nibcase-test), on Mesa's surfaceless
            // platform: the GPU without a window or a display.
            let share = SHARE_WITH_CURRENT.load(Ordering::Acquire)
                && !outer.display.is_null()
                && !outer.context.is_null();
            let platform = (egl.eglGetProcAddress)(c"eglGetPlatformDisplayEXT".as_ptr());
            let mut display = if share { outer.display } else { null_mut() };
            if !share && !platform.is_null() {
                let get: unsafe extern "C" fn(u32, Egl, *const i32) -> Egl =
                    std::mem::transmute(platform);
                display = get(
                    EGL_PLATFORM_SURFACELESS_MESA,
                    null_mut(),
                    [EGL_NONE].as_ptr(),
                );
            }
            if display.is_null() {
                display = (egl.eglGetDisplay)(null_mut());
            }
            if display.is_null()
                || (!share && (egl.eglInitialize)(display, null_mut(), null_mut()) == 0)
            {
                return Err(format!("EGL: no display ({:#x})", (egl.eglGetError)()));
            }
            let (api, bit) = if kind.es {
                (
                    EGL_OPENGL_ES_API,
                    if kind.major >= 3 {
                        EGL_OPENGL_ES3_BIT
                    } else {
                        EGL_OPENGL_ES2_BIT
                    },
                )
            } else {
                (EGL_OPENGL_API, EGL_OPENGL_BIT)
            };
            let fail = |what: &str| {
                let e = (egl.eglGetError)();
                if !share {
                    (egl.eglTerminate)(display);
                }
                format!("EGL: {what} ({e:#x})")
            };
            if (egl.eglBindAPI)(api) == 0 {
                return Err(fail("the API"));
            }
            let attributes = [
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_RENDERABLE_TYPE,
                bit,
                EGL_SURFACE_TYPE,
                0,
                EGL_NONE,
            ];
            let (mut config, mut count) = (null_mut(), 0);
            if (egl.eglChooseConfig)(display, attributes.as_ptr(), &mut config, 1, &mut count) == 0
                || count == 0
            {
                return Err(fail("no configuration"));
            }
            let mut context_attributes = vec![
                EGL_CONTEXT_MAJOR_VERSION,
                kind.major,
                EGL_CONTEXT_MINOR_VERSION,
                kind.minor,
            ];
            if !kind.es && kind.major >= 3 {
                context_attributes.extend([
                    EGL_CONTEXT_OPENGL_PROFILE_MASK,
                    if kind.core {
                        EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT
                    } else {
                        EGL_CONTEXT_OPENGL_COMPATIBILITY_PROFILE_BIT
                    },
                ]);
            }
            context_attributes.push(EGL_NONE);
            let group = if share { outer.context } else { null_mut() };
            let context =
                (egl.eglCreateContext)(display, config, group, context_attributes.as_ptr());
            if context.is_null() {
                return Err(fail("no context"));
            }
            if (egl.eglMakeCurrent)(display, null_mut(), null_mut(), context) == 0 {
                (egl.eglDestroyContext)(display, context);
                return Err(fail("the context won't be made current"));
            }
            let loaded = Fns::load(egl.eglGetProcAddress);
            let gl = match loaded {
                Ok(gl) => gl,
                Err(e) => {
                    restore(&egl, outer, display);
                    (egl.eglDestroyContext)(display, context);
                    if !share {
                        (egl.eglTerminate)(display);
                    }
                    return Err(format!("GL: {e}"));
                }
            };
            let text = |name| {
                let s = (gl.glGetString)(name);
                if s.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(s.cast()).to_string_lossy().into_owned()
                }
            };
            // What EGL made, not what was asked: Mesa gives OpenGL ES 3.2 for a core that asks
            // for 2.0 (as PPSSPP does).
            let version = text(GL_VERSION);
            let made = major_version(&version).unwrap_or(kind.major);
            let pbo = if made >= 3 {
                Async::load(egl.eglGetProcAddress).ok()
            } else {
                None
            };
            let kind = Kind {
                major: made.max(kind.major),
                ..kind
            };
            eprintln!(
                "nibcase: {} on {}{}{}",
                version,
                text(GL_RENDERER),
                if share {
                    ", sharing the player's renderer's textures"
                } else {
                    ""
                },
                if pbo.is_some() {
                    ""
                } else {
                    ", frames read back as they finish"
                }
            );
            GET_PROC.store(egl.eglGetProcAddress as *mut c_void, Ordering::Release);
            Ok(Gl {
                egl,
                gl,
                pbo,
                kind,
                display,
                context,
                outer: Some(outer),
                fbo: 0,
                color: 0,
                depth: 0,
                size: (0, 0),
                want_depth: depth,
                want_stencil: stencil,
                buffers: [0; 2],
                buffer_bytes: 0,
                next: 0,
                pending: None,
                shared: share,
                frame_fence: null_mut(),
                timing: std::env::var("NIBCASE_DEBUG")
                    .is_ok_and(|d| d.split(',').any(|w| w == "gl"))
                    .then(Timing::default),
                _gles: gles,
                _lib: lib,
            })
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The core's context made current, what was current kept to be put back.
    pub fn enter(&mut self) {
        // SAFETY: EGL handles this context made; the saved ones are EGL's own.
        unsafe {
            let outer = current(&self.egl);
            if outer.context != self.context {
                (self.egl.eglBindAPI)(if self.kind.es {
                    EGL_OPENGL_ES_API
                } else {
                    EGL_OPENGL_API
                });
                (self.egl.eglMakeCurrent)(self.display, null_mut(), null_mut(), self.context);
                self.outer = Some(outer);
            }
        }
    }

    /// What was current before [`Gl::enter`] (or [`Gl::new`]) put back.
    pub fn leave(&mut self) {
        if let Some(outer) = self.outer.take() {
            // SAFETY: as enter.
            unsafe { restore(&self.egl, outer, self.display) };
        }
    }

    /// The framebuffer at least `width` x `height` (made, or its storage grown, keeping its
    /// name: the core may hold it). Current context only.
    pub fn ensure(&mut self, width: u32, height: u32) -> Result<(), String> {
        let (width, height) = (width.max(self.size.0), height.max(self.size.1));
        if self.fbo != 0 && (width, height) == self.size {
            return Ok(());
        }
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("invalid hardware framebuffer size".into());
        }
        let gl = &self.gl;
        // SAFETY: GL calls on the current context with names it made.
        unsafe {
            let (mut fb, mut tex, mut rb) = (0, 0, 0);
            (gl.glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &mut fb);
            (gl.glGetIntegerv)(GL_TEXTURE_BINDING_2D, &mut tex);
            (gl.glGetIntegerv)(GL_RENDERBUFFER_BINDING, &mut rb);
            if self.fbo == 0 {
                (gl.glGenFramebuffers)(1, &mut self.fbo);
                (gl.glGenTextures)(1, &mut self.color);
                if self.want_depth || self.want_stencil {
                    (gl.glGenRenderbuffers)(1, &mut self.depth);
                }
            }
            (gl.glBindTexture)(GL_TEXTURE_2D, self.color);
            (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            (gl.glTexImage2D)(
                GL_TEXTURE_2D,
                0,
                if self.kind.es && self.kind.major < 3 {
                    GL_RGBA as i32
                } else {
                    GL_RGBA8 as i32
                },
                width as i32,
                height as i32,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
            (gl.glBindFramebuffer)(GL_FRAMEBUFFER, self.fbo);
            (gl.glFramebufferTexture2D)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                self.color,
                0,
            );
            if self.depth != 0 {
                (gl.glBindRenderbuffer)(GL_RENDERBUFFER, self.depth);
                let stencil = self.want_stencil;
                (gl.glRenderbufferStorage)(
                    GL_RENDERBUFFER,
                    if stencil {
                        GL_DEPTH24_STENCIL8
                    } else {
                        GL_DEPTH_COMPONENT24
                    },
                    width as i32,
                    height as i32,
                );
                (gl.glFramebufferRenderbuffer)(
                    GL_FRAMEBUFFER,
                    if stencil {
                        GL_DEPTH_STENCIL_ATTACHMENT
                    } else {
                        GL_DEPTH_ATTACHMENT
                    },
                    GL_RENDERBUFFER,
                    self.depth,
                );
            }
            let status = (gl.glCheckFramebufferStatus)(GL_FRAMEBUFFER);
            (gl.glBindFramebuffer)(GL_FRAMEBUFFER, fb as u32);
            (gl.glBindTexture)(GL_TEXTURE_2D, tex as u32);
            (gl.glBindRenderbuffer)(GL_RENDERBUFFER, rb as u32);
            if status != GL_FRAMEBUFFER_COMPLETE {
                return Err(format!("hardware framebuffer incomplete ({status:#x})"));
            }
        }
        self.size = (width, height);
        FRAMEBUFFER.store(self.fbo as usize, Ordering::Release);
        Ok(())
    }

    /// The frame the core has just drawn, `width` x `height` at the framebuffer's corner, read
    /// back: this one, or with pixel-pack buffers the one before (`None` on the first). Rows
    /// come out top first; `bottom_left_origin` says the core drew them the GL way up.
    pub fn read(
        &mut self,
        width: u32,
        height: u32,
        bottom_left_origin: bool,
        sync: bool,
    ) -> Result<Option<Frame>, String> {
        if width == 0 || height == 0 || width > self.size.0 || height > self.size.1 {
            return Err("hardware frame larger than its framebuffer".into());
        }
        let bytes = width as usize * height as usize * 4;
        let started = std::time::Instant::now();
        let mut lap = started;
        let mut laps = [0f64; 3];
        let mut split = |i: usize| {
            let now = std::time::Instant::now();
            laps[i] += (now - lap).as_secs_f64();
            lap = now;
        };
        let gl = &self.gl;
        // SAFETY: GL calls on the current context, into buffers of the sizes given.
        unsafe {
            let mut fb = 0;
            (gl.glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &mut fb);
            (gl.glBindFramebuffer)(GL_FRAMEBUFFER, self.fbo);
            (gl.glPixelStorei)(GL_PACK_ALIGNMENT, 4);
            if self.kind.major >= 3 {
                (gl.glPixelStorei)(GL_PACK_ROW_LENGTH, 0);
            }
            let frame = match self.pbo.as_ref().filter(|_| !sync) {
                None => {
                    let mut rgba = vec![0u8; bytes];
                    (gl.glReadPixels)(
                        0,
                        0,
                        width as i32,
                        height as i32,
                        GL_RGBA,
                        GL_UNSIGNED_BYTE,
                        rgba.as_mut_ptr().cast(),
                    );
                    Some(finish(rgba, width, height, bottom_left_origin))
                }
                Some(p) => {
                    let mut bound = 0;
                    (gl.glGetIntegerv)(GL_PIXEL_PACK_BUFFER_BINDING, &mut bound);
                    if self.buffers[0] == 0 || self.buffer_bytes < bytes {
                        if self.buffers[0] != 0 {
                            (p.glDeleteBuffers)(2, self.buffers.as_ptr());
                        }
                        if let Some(old) = self.pending.take() {
                            (p.glDeleteSync)(old.fence);
                        }
                        (p.glGenBuffers)(2, self.buffers.as_mut_ptr());
                        let size = self.size.0 as usize * self.size.1 as usize * 4;
                        for b in self.buffers {
                            (p.glBindBuffer)(GL_PIXEL_PACK_BUFFER, b);
                            (p.glBufferData)(
                                GL_PIXEL_PACK_BUFFER,
                                size as isize,
                                std::ptr::null(),
                                GL_STREAM_READ,
                            );
                        }
                        self.buffer_bytes = size;
                    }
                    // This frame, into the next buffer, the GPU left to finish it.
                    let buffer = self.buffers[self.next];
                    self.next ^= 1;
                    (p.glBindBuffer)(GL_PIXEL_PACK_BUFFER, buffer);
                    (gl.glReadPixels)(
                        0,
                        0,
                        width as i32,
                        height as i32,
                        GL_RGBA,
                        GL_UNSIGNED_BYTE,
                        std::ptr::null_mut(),
                    );
                    let fence = (p.glFenceSync)(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
                    (gl.glFlush)();
                    split(0);
                    let ready = self.pending.replace(Pending {
                        buffer,
                        fence,
                        width,
                        height,
                    });
                    // The frame before, read a frame ago: done by now, or waited for.
                    let out = match ready {
                        None => None,
                        Some(r) => {
                            let waited = (p.glClientWaitSync)(
                                r.fence,
                                GL_SYNC_FLUSH_COMMANDS_BIT,
                                1_000_000_000,
                            );
                            (p.glDeleteSync)(r.fence);
                            split(1);
                            let n = r.width as usize * r.height as usize * 4;
                            (p.glBindBuffer)(GL_PIXEL_PACK_BUFFER, r.buffer);
                            let mapped = if waited == GL_WAIT_FAILED {
                                null_mut()
                            } else {
                                (p.glMapBufferRange)(
                                    GL_PIXEL_PACK_BUFFER,
                                    0,
                                    n as isize,
                                    GL_MAP_READ_BIT,
                                )
                            };
                            if mapped.is_null() {
                                None
                            } else {
                                let rgba =
                                    std::slice::from_raw_parts(mapped.cast::<u8>(), n).to_vec();
                                (p.glUnmapBuffer)(GL_PIXEL_PACK_BUFFER);
                                Some(finish(rgba, r.width, r.height, bottom_left_origin))
                            }
                        }
                    };
                    split(2);
                    (p.glBindBuffer)(GL_PIXEL_PACK_BUFFER, bound as u32);
                    out
                }
            };
            (gl.glBindFramebuffer)(GL_FRAMEBUFFER, fb as u32);
            if let Some(t) = self.timing.as_mut() {
                t.total += started.elapsed().as_secs_f64();
                t.issue += laps[0];
                t.wait += laps[1];
                t.copy += laps[2];
                t.frame();
            }
            let error = (gl.glGetError)();
            if error != 0 && frame.is_none() && (sync || self.pbo.is_none()) {
                return Err(format!(
                    "reading the hardware frame back: GL error {error:#x}"
                ));
            }
            Ok(frame)
        }
    }
}

impl Gl {
    /// The core's commands sent to the GPU.
    pub fn flush(&self) {
        // SAFETY: the current context's flush.
        unsafe { (self.gl.glFlush)() };
    }

    /// Whether the context shares a group with the player's renderer.
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// After a frame, in a shared context: a fence for the renderer's context to wait on before
    /// copying the frame, the one before deleted; then the commands sent. Current context only.
    pub fn fence_frame(&mut self) {
        if let Some(p) = &self.pbo {
            // SAFETY: fences of this share group, made and deleted on its current context.
            unsafe {
                if !self.frame_fence.is_null() {
                    (p.glDeleteSync)(self.frame_fence);
                }
                self.frame_fence = (p.glFenceSync)(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
            }
        }
        self.flush();
    }

    /// The colour buffer's texture (a name in the share group), its size, and the fence after
    /// the last frame (null before the first).
    pub fn shared_frame(&self) -> Option<SharedFrame> {
        (self.shared && self.color != 0).then_some(SharedFrame {
            texture: self.color,
            size: self.size,
            fence: self.frame_fence,
        })
    }
}

/// A shared context's frame: its colour buffer's texture, the buffer's size (the frame at its
/// corner the GL way up), and the fence to wait on before reading it.
#[derive(Clone, Copy, Debug)]
pub struct SharedFrame {
    pub texture: u32,
    pub size: (u32, u32),
    pub fence: *mut c_void,
}

/// The functions [`copy_shared_frame`] calls, loaded once (eglGetProcAddress's are the same in
/// every context).
struct CopyFns {
    gl: Fns,
    sync: Async,
    _lib: Library,
}

static COPY_FNS: std::sync::OnceLock<Option<CopyFns>> = std::sync::OnceLock::new();

/// The renderer's framebuffer for reading a shared frame: one, made on its context.
static COPY_FBO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static COPY_DRAW_FBO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const GL_READ_FRAMEBUFFER: u32 = 0x8CA8;
const GL_DRAW_FRAMEBUFFER: u32 = 0x8CA9;
const GL_READ_FRAMEBUFFER_BINDING: u32 = 0x8CAA;
const GL_DRAW_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
const GL_COLOR_BUFFER_BIT: u32 = 0x4000;

const GL_COLOR_ATTACHMENT0_READ: u32 = GL_COLOR_ATTACHMENT0;
const GL_TIMEOUT_IGNORED: u64 = u64::MAX;

/// `frame`, `width` x `height` at its corner, copied into the texture bound to `GL_TEXTURE_2D` in
/// the current context (the player's renderer's, in the core's share group), after waiting on
/// the GPU for its fence: a copy on the GPU, rows as they are (the frame's bottom first).
pub fn copy_shared_frame(frame: &SharedFrame, width: u32, height: u32) -> Result<(), String> {
    let fns = COPY_FNS
        .get_or_init(|| {
            let (lib, egl) = load_egl().ok()?;
            // SAFETY: GL functions by name from EGL, valid in any context.
            unsafe {
                Some(CopyFns {
                    gl: Fns::load(egl.eglGetProcAddress).ok()?,
                    sync: Async::load(egl.eglGetProcAddress).ok()?,
                    _lib: lib,
                })
            }
        })
        .as_ref()
        .ok_or("no GL functions for copying the frame")?;
    let (gl, sync) = (&fns.gl, &fns.sync);
    // Within the buffer: a frame the core says is larger is cut to it.
    let (width, height) = (width.min(frame.size.0), height.min(frame.size.1));
    // SAFETY: GL calls on the current context; the texture and fence are of its share group.
    unsafe {
        // Errors left by the renderer's own calls (SDL doesn't read them) aren't the copy's.
        for _ in 0..16 {
            if (gl.glGetError)() == 0 {
                break;
            }
        }
        if !frame.fence.is_null() {
            (sync.glWaitSync)(frame.fence, 0, GL_TIMEOUT_IGNORED);
        }
        // Two framebuffers of the renderer's: one reading the core's texture, one drawing
        // into the renderer's texture bound now. A blit stays on the GPU.
        let mut fbos = [
            COPY_FBO.load(Ordering::Relaxed),
            COPY_DRAW_FBO.load(Ordering::Relaxed),
        ];
        if fbos[0] == 0 {
            (gl.glGenFramebuffers)(2, fbos.as_mut_ptr());
            COPY_FBO.store(fbos[0], Ordering::Relaxed);
            COPY_DRAW_FBO.store(fbos[1], Ordering::Relaxed);
        }
        let (mut read, mut draw, mut target) = (0, 0, 0);
        (gl.glGetIntegerv)(GL_READ_FRAMEBUFFER_BINDING, &mut read);
        (gl.glGetIntegerv)(GL_DRAW_FRAMEBUFFER_BINDING, &mut draw);
        (gl.glGetIntegerv)(GL_TEXTURE_BINDING_2D, &mut target);
        (gl.glBindFramebuffer)(GL_READ_FRAMEBUFFER, fbos[0]);
        (gl.glFramebufferTexture2D)(
            GL_READ_FRAMEBUFFER,
            GL_COLOR_ATTACHMENT0_READ,
            GL_TEXTURE_2D,
            frame.texture,
            0,
        );
        (gl.glBindFramebuffer)(GL_DRAW_FRAMEBUFFER, fbos[1]);
        (gl.glFramebufferTexture2D)(
            GL_DRAW_FRAMEBUFFER,
            GL_COLOR_ATTACHMENT0_READ,
            GL_TEXTURE_2D,
            target as u32,
            0,
        );
        let status = (
            (gl.glCheckFramebufferStatus)(GL_READ_FRAMEBUFFER),
            (gl.glCheckFramebufferStatus)(GL_DRAW_FRAMEBUFFER),
        );
        if status == (GL_FRAMEBUFFER_COMPLETE, GL_FRAMEBUFFER_COMPLETE) {
            let (w, h) = (width as i32, height as i32);
            (sync.glBlitFramebuffer)(
                0,
                0,
                w,
                h,
                0,
                0,
                w,
                h,
                GL_COLOR_BUFFER_BIT,
                GL_NEAREST as u32,
            );
        }
        (gl.glBindFramebuffer)(GL_READ_FRAMEBUFFER, read as u32);
        (gl.glBindFramebuffer)(GL_DRAW_FRAMEBUFFER, draw as u32);
        if status != (GL_FRAMEBUFFER_COMPLETE, GL_FRAMEBUFFER_COMPLETE) {
            return Err(format!(
                "the shared frame can't be blitted ({:#x}, {:#x})",
                status.0, status.1
            ));
        }
        let error = (gl.glGetError)();
        if error != 0 {
            return Err(format!(
                "copying the shared frame ({width} x {height} of {} x {}): GL error {error:#x}",
                frame.size.0, frame.size.1
            ));
        }
    }
    Ok(())
}

impl Drop for Gl {
    fn drop(&mut self) {
        self.enter();
        // SAFETY: names this context made, deleted on it; then the context itself.
        unsafe {
            if let Some(p) = &self.pbo {
                if let Some(old) = self.pending.take() {
                    (p.glDeleteSync)(old.fence);
                }
                if !self.frame_fence.is_null() {
                    (p.glDeleteSync)(self.frame_fence);
                }
                if self.buffers[0] != 0 {
                    (p.glDeleteBuffers)(2, self.buffers.as_ptr());
                }
            }
            if self.fbo != 0 {
                (self.gl.glDeleteFramebuffers)(1, &self.fbo);
                (self.gl.glDeleteTextures)(1, &self.color);
                if self.depth != 0 {
                    (self.gl.glDeleteRenderbuffers)(1, &self.depth);
                }
            }
        }
        self.leave();
        GET_PROC.store(null_mut(), Ordering::Release);
        FRAMEBUFFER.store(0, Ordering::Release);
        // SAFETY: the context is no longer current on this thread.
        unsafe {
            (self.egl.eglDestroyContext)(self.display, self.context);
            // A shared context's display is the player's renderer's.
            if !self.shared {
                (self.egl.eglTerminate)(self.display);
            }
        }
    }
}

/// The major version in a `GL_VERSION` string: "OpenGL ES 3.2 Mesa ..." or "4.6 (Core ...)".
fn major_version(version: &str) -> Option<i32> {
    let number = version
        .split_whitespace()
        .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    number.split('.').next()?.parse().ok()
}

/// The system's EGL and the functions nibcase calls.
fn load_egl() -> Result<(Library, EglFns), String> {
    // SAFETY: loading the system's EGL; every symbol is checked below.
    let lib = unsafe { Library::new("libEGL.so.1") }.map_err(|e| format!("EGL: {e}"))?;
    macro_rules! sym {
        ($name:literal) => {
            // SAFETY: the EGL 1.4 function of that name, with the signature of its field.
            *unsafe { lib.get(concat!($name, "\0").as_bytes()) }
                .map_err(|e| format!("EGL {}: {e}", $name))?
        };
    }
    let egl = EglFns {
        eglGetProcAddress: sym!("eglGetProcAddress"),
        eglGetDisplay: sym!("eglGetDisplay"),
        eglInitialize: sym!("eglInitialize"),
        eglTerminate: sym!("eglTerminate"),
        eglBindAPI: sym!("eglBindAPI"),
        eglChooseConfig: sym!("eglChooseConfig"),
        eglCreateContext: sym!("eglCreateContext"),
        eglDestroyContext: sym!("eglDestroyContext"),
        eglMakeCurrent: sym!("eglMakeCurrent"),
        eglGetCurrentContext: sym!("eglGetCurrentContext"),
        eglGetCurrentDisplay: sym!("eglGetCurrentDisplay"),
        eglGetCurrentSurface: sym!("eglGetCurrentSurface"),
        eglQueryAPI: sym!("eglQueryAPI"),
        eglGetError: sym!("eglGetError"),
    };
    Ok((lib, egl))
}

/// What is current on this thread, to put back.
unsafe fn current(egl: &EglFns) -> Current {
    // SAFETY: EGL's queries of this thread's state.
    unsafe {
        Current {
            api: (egl.eglQueryAPI)(),
            display: (egl.eglGetCurrentDisplay)(),
            draw: (egl.eglGetCurrentSurface)(EGL_DRAW),
            read: (egl.eglGetCurrentSurface)(EGL_READ),
            context: (egl.eglGetCurrentContext)(),
        }
    }
}

/// `outer` current again; nothing current when nothing was (ours released through `ours`).
unsafe fn restore(egl: &EglFns, outer: Current, ours: Egl) {
    // SAFETY: handles EGL gave, made current again on the thread they were current on.
    unsafe {
        if outer.display.is_null() {
            (egl.eglMakeCurrent)(ours, null_mut(), null_mut(), null_mut());
        } else {
            (egl.eglBindAPI)(outer.api);
            (egl.eglMakeCurrent)(outer.display, outer.draw, outer.read, outer.context);
        }
    }
}

/// A frame read back, its rows top first, opaque (GL leaves alpha as the core drew it).
fn finish(mut rgba: Vec<u8>, width: u32, height: u32, bottom_left_origin: bool) -> Frame {
    let row = width as usize * 4;
    if bottom_left_origin {
        let h = height as usize;
        for y in 0..h / 2 {
            let (top, bottom) = rgba.split_at_mut((h - 1 - y) * row);
            top[y * row..(y + 1) * row].swap_with_slice(&mut bottom[..row]);
        }
    }
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    Frame {
        width,
        height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_come_out_top_first_and_opaque() {
        // Two rows, bottom first as GL reads them: the bottom row (2) comes out last.
        let rgba = vec![2, 2, 2, 0, 1, 1, 1, 9];
        let f = finish(rgba.clone(), 1, 2, true);
        assert_eq!(f.rgba, [1, 1, 1, 255, 2, 2, 2, 255]);
        let f = finish(rgba, 1, 2, false);
        assert_eq!(f.rgba, [2, 2, 2, 255, 1, 1, 1, 255]);
        let odd = finish((0..12).collect(), 1, 3, true);
        assert_eq!(odd.rgba, [8, 9, 10, 255, 4, 5, 6, 255, 0, 1, 2, 255]);
    }

    #[test]
    fn versions_are_read_from_gl_version() {
        assert_eq!(major_version("OpenGL ES 3.2 Mesa 26.2.4"), Some(3));
        assert_eq!(major_version("OpenGL ES 2.0 Mesa"), Some(2));
        assert_eq!(major_version("4.6 (Core Profile) Mesa 26.2.4"), Some(4));
        assert_eq!(major_version("OpenGL ES"), None);
    }

    #[test]
    fn the_contexts_egl_can_make() {
        let mut hw = retro_hw_render_callback {
            context_type: retro_hw_context_type_RETRO_HW_CONTEXT_OPENGLES3,
            ..Default::default()
        };
        let es3 = Kind::of(&hw).unwrap();
        assert!(es3.es && es3.major == 3);
        hw.context_type = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGLES2;
        assert_eq!(Kind::of(&hw).unwrap().major, 2);
        hw.context_type = retro_hw_context_type_RETRO_HW_CONTEXT_OPENGL_CORE;
        hw.version_major = 3;
        hw.version_minor = 3;
        let core = Kind::of(&hw).unwrap();
        assert!(!core.es && core.core && (core.major, core.minor) == (3, 3));
        hw.context_type = retro_hw_context_type_RETRO_HW_CONTEXT_VULKAN;
        assert!(Kind::of(&hw).is_none());
    }
}
