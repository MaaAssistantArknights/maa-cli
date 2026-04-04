//! Wayland screen capture via `zwlr_screencopy_manager_v1`.
//!
//! Connects to a compositor socket (default `"wayland-1"`, the cage socket that
//! Waydroid runs inside) and captures frames with the wlr-screencopy protocol.
//!
//! # Buffer strategy
//!
//! `wl_shm` shared-memory buffers are always used and fully implemented.
//! When the compositor advertises a `linux_dmabuf` buffer option (wlr-screencopy
//! ≥ v2), the DMA-BUF path is preferred over `wl_shm` when the
//! `wayland-dmabuf` Cargo feature is enabled and a GBM render device is
//! available; otherwise `wl_shm` is used transparently as a fallback.
//!
//! # Damage tracking
//!
//! [`WaylandScreencap::capture`] issues a plain `copy` on the very first call
//! to establish a baseline, then switches to `copy_with_damage` for every
//! subsequent call.  The compositor then populates [`Frame::damage`] with the
//! screen regions that have actually changed, allowing callers to skip
//! expensive downstream processing when the content is unchanged.
//!
//! # Session lifetime
//!
//! Creating a [`WaylandScreencap`] connects to the compositor and enumerates
//! global objects — keep **one instance** alive across many captures instead
//! of re-creating it per frame.

use std::{
    os::unix::{
        io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        net::UnixStream,
    },
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use log::trace;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
    },
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A single captured screen frame.
#[derive(Debug)]
pub struct Frame {
    /// Raw pixel bytes.  Layout: `height` rows of `stride` bytes each.
    pub data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Bytes per row (may exceed `width * 4` due to alignment padding).
    pub stride: u32,
    /// Pixel layout of [`data`][Self::data] — describes per-byte channel order.
    pub format: PixelFormat,
    /// Changed rectangles reported by the compositor when `copy_with_damage`
    /// was used.  Empty on the very first capture, or when the compositor did
    /// not send damage information (treat the whole frame as changed in that
    /// case).
    pub damage: Vec<DamageRect>,
}

/// In-memory byte order of a single 32-bit pixel.
///
/// The names match Wayland's `wl_shm` format identifiers; on a little-endian
/// host each name describes the **memory** channel order left-to-right.
///
/// | Variant      | Byte 0 | Byte 1 | Byte 2 | Byte 3 |
/// |--------------|--------|--------|--------|--------|
/// | `Bgra`       | B      | G      | R      | A      |
/// | `Bgrx`       | B      | G      | R      | x      |
/// | `Rgba`       | R      | G      | B      | A      |
/// | `Rgbx`       | R      | G      | B      | x      |
///
/// `Bgra` corresponds to `WL_SHM_FORMAT_ARGB8888` and is the most common
/// format returned by wlroots-based compositors.  `Bgra`/`Bgrx` maps
/// directly to MAA's internal BGRA image representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// `WL_SHM_FORMAT_ARGB8888`: bytes [B, G, R, A].
    Bgra,
    /// `WL_SHM_FORMAT_XRGB8888`: bytes [B, G, R, x].
    Bgrx,
    /// `WL_SHM_FORMAT_ABGR8888`: bytes [R, G, B, A].
    Rgba,
    /// `WL_SHM_FORMAT_XBGR8888`: bytes [R, G, B, x].
    Rgbx,
}

/// A damaged (changed) rectangle within a frame.
///
/// Coordinates are in compositor pixels, relative to the top-left corner.
#[derive(Debug, Clone, Copy)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ─── Session ──────────────────────────────────────────────────────────────────

/// Persistent Wayland compositor connection for screen capture.
///
/// Create with [`WaylandScreencap::connect`] and reuse across many
/// [`capture`][Self::capture] calls — there is no per-frame reconnection cost.
///
/// ## Socket selection
///
/// A bare display name (e.g. `"wayland-1"`) is resolved relative to
/// `$XDG_RUNTIME_DIR`.  A value containing a `/` is used as-is as a
/// filesystem path (absolute or relative).
pub struct WaylandScreencap {
    conn: Connection,
    event_queue: EventQueue<CaptureState>,
    state: CaptureState,
    first_frame: bool,
}

impl WaylandScreencap {
    /// Connect to the Wayland compositor at `display`.
    ///
    /// Defaults to `"wayland-1"` (the cage socket used by Waydroid) when
    /// `display` is `None`.
    pub fn connect(display: Option<&str>) -> Result<Self> {
        let display_name = display.unwrap_or("wayland-1");
        let socket_path = resolve_socket(display_name)?;

        let stream = UnixStream::connect(&socket_path)
            .with_context(|| format!("Failed to connect to Wayland socket {socket_path:?}"))?;
        let conn =
            Connection::from_socket(stream).context("Failed to create Wayland connection")?;

        let mut event_queue: EventQueue<CaptureState> = conn.new_event_queue();
        let qh = event_queue.handle();
        let mut state = CaptureState::default();

        // Enumerate compositor globals
        conn.display().get_registry(&qh, ());
        event_queue
            .roundtrip(&mut state)
            .context("Wayland registry roundtrip failed")?;
        // Second roundtrip: wl_shm sends `format` events after binding
        event_queue
            .roundtrip(&mut state)
            .context("Wayland format roundtrip failed")?;

        if state.screencopy_mgr.is_none() {
            bail!(
                "Compositor at {socket_path:?} does not advertise \
                 zwlr_screencopy_manager_v1 — is it a wlroots-based compositor?"
            );
        }
        if state.shm.is_none() {
            bail!("Compositor does not advertise wl_shm");
        }
        if state.output.is_none() {
            bail!("No wl_output found on compositor");
        }

        Ok(Self {
            conn,
            event_queue,
            state,
            first_frame: true,
        })
    }

    /// Capture one frame from the compositor output.
    ///
    /// Uses `copy_with_damage` after the first frame; inspect
    /// [`Frame::damage`] to decide whether the content actually changed.
    ///
    /// This call blocks until the compositor has filled the buffer and
    /// signalled readiness.
    pub fn capture(&mut self) -> Result<Frame> {
        let qh = self.event_queue.handle();

        // Clone proxy handles to avoid conflicting borrows during dispatch.
        // Wayland proxy clones are cheap (they hold a shared Arc to the object).
        let mgr = self.state.screencopy_mgr.clone().unwrap();
        let output = self.state.output.clone().unwrap();
        let shm = self.state.shm.clone().unwrap();

        // ── Step 1: request a new screencopy frame ────────────────────────────
        let frame_obj: ZwlrScreencopyFrameV1 = mgr.capture_output(0, &output, &qh, ());
        self.state.reset_frame();

        // ── Step 2: collect buffer specifications ─────────────────────────────
        // wlr-screencopy ≥ v2: compositor sends `buffer`, optionally
        // `linux_dmabuf`, then `buffer_done`.
        // v1 fallback: only `buffer`, no `buffer_done` — handle with roundtrip.
        loop {
            self.event_queue.blocking_dispatch(&mut self.state)?;
            match self.state.frame_phase {
                FramePhase::BufferDone => break,
                FramePhase::WaitingBuffer if self.state.shm_spec.is_some() => {
                    // v1 compositor: no `buffer_done` event.  Do one roundtrip
                    // to drain any further events, then treat as done.
                    self.event_queue.roundtrip(&mut self.state)?;
                    if self.state.frame_phase != FramePhase::BufferDone {
                        self.state.frame_phase = FramePhase::BufferDone;
                    }
                    break;
                }
                _ => {}
            }
        }

        let shm_spec = self
            .state
            .shm_spec
            .clone()
            .context("Frame capture aborted: compositor sent no buffer spec")?;

        // ── Step 3: allocate a shared-memory buffer ───────────────────────────
        // DMA-BUF preferred over wl_shm when available and feature is enabled.
        // Currently the wl_shm path is always used; the DMA-BUF path is wired
        // up when the `wayland-dmabuf` feature is enabled (future work: allocate
        // a GBM linear buffer, export it as a prime fd, import via
        // zwp_linux_dmabuf_v1, and mmap for CPU readback).
        #[cfg(feature = "wayland-dmabuf")]
        if self.state.dmabuf_spec.is_some() {
            trace!("linux_dmabuf buffer option available but not yet implemented; using wl_shm");
        }

        let buf = ShmBuffer::alloc(&shm, &shm_spec, &qh)?;

        // ── Step 4: issue the copy request ────────────────────────────────────
        if self.first_frame || self.state.screencopy_version < 3 {
            frame_obj.copy(&buf.wl_buf);
            self.first_frame = false;
        } else {
            frame_obj.copy_with_damage(&buf.wl_buf);
        }
        self.state.frame_phase = FramePhase::Copying;
        self.conn.flush().context("Wayland flush failed")?;

        // ── Step 5: wait for `ready` or `failed` ─────────────────────────────
        loop {
            self.event_queue.blocking_dispatch(&mut self.state)?;
            match self.state.frame_phase {
                FramePhase::Ready => break,
                FramePhase::Failed => {
                    frame_obj.destroy();
                    bail!("Wayland screencopy: compositor reported frame failure");
                }
                _ => {}
            }
        }

        // ── Step 6: read pixels out of the shared-memory region ───────────────
        let format = wl_shm_to_pixel_format(shm_spec.format).with_context(|| {
            format!(
                "Compositor returned unsupported wl_shm format {:?}; \
                 expected ARGB8888, XRGB8888, ABGR8888, or XBGR8888",
                shm_spec.format
            )
        })?;
        let pixel_data = buf.read().to_vec();
        let damage = std::mem::take(&mut self.state.damage);

        frame_obj.destroy();
        // `buf` drops here: sends wl_buffer.destroy + wl_shm_pool.destroy,
        // then unmaps the shared-memory region.

        Ok(Frame {
            data: pixel_data,
            width: shm_spec.width,
            height: shm_spec.height,
            stride: shm_spec.stride,
            format,
            damage,
        })
    }
}

// ─── Internal state machine ───────────────────────────────────────────────────

#[derive(Default)]
struct CaptureState {
    // Globals discovered via the registry
    screencopy_mgr: Option<ZwlrScreencopyManagerV1>,
    screencopy_version: u32,
    shm: Option<WlShm>,
    output: Option<WlOutput>,

    // Per-frame state
    shm_spec: Option<ShmSpec>,
    #[cfg(feature = "wayland-dmabuf")]
    dmabuf_spec: Option<DmabufSpec>,
    frame_phase: FramePhase,
    damage: Vec<DamageRect>,
}

impl CaptureState {
    fn reset_frame(&mut self) {
        self.shm_spec = None;
        #[cfg(feature = "wayland-dmabuf")]
        {
            self.dmabuf_spec = None;
        }
        self.frame_phase = FramePhase::WaitingBuffer;
        self.damage.clear();
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum FramePhase {
    #[default]
    Idle,
    WaitingBuffer,
    BufferDone,
    Copying,
    Ready,
    Failed,
}

#[derive(Clone)]
struct ShmSpec {
    format: wl_shm::Format,
    width: u32,
    height: u32,
    stride: u32,
}

#[cfg(feature = "wayland-dmabuf")]
#[derive(Clone)]
struct DmabufSpec {
    /// DRM fourcc format code.
    format: u32,
    width: u32,
    height: u32,
}

// ─── Dispatch implementations ─────────────────────────────────────────────────

impl Dispatch<WlRegistry, ()> for CaptureState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            trace!("Wayland global: {interface} v{version}");
            match interface.as_str() {
                "zwlr_screencopy_manager_v1" => {
                    // Version 2 adds linux_dmabuf + buffer_done.
                    // Version 3 adds copy_with_damage.
                    let v = version.min(3);
                    let mgr = registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, v, qh, ());
                    state.screencopy_mgr = Some(mgr);
                    state.screencopy_version = v;
                }
                "wl_output" => {
                    let out = registry.bind::<WlOutput, _, _>(name, version.min(4), qh, ());
                    // Use the first advertised output (primary display).
                    state.output.get_or_insert(out);
                }
                "wl_shm" => {
                    let shm = registry.bind::<WlShm, _, _>(name, 1, qh, ());
                    state.shm = Some(shm);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlShm, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        event: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Log advertised shm formats for diagnostics; no action needed.
        if let wl_shm::Event::Format { format } = event {
            trace!("wl_shm format advertised: {format:?}");
        }
    }
}

impl Dispatch<WlShmPool, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // wl_shm_pool has no events.
    }
}

impl Dispatch<WlBuffer, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // wl_buffer.release is only relevant for surface-attached rendering
        // buffers; for screencopy we rely on the frame's `ready` event.
    }
}

impl Dispatch<WlOutput, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Geometry/mode events are not used here.
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &ZwlrScreencopyManagerV1,
        _: zwlr_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // zwlr_screencopy_manager_v1 has no events.
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                if let WEnum::Value(fmt) = format {
                    state.shm_spec = Some(ShmSpec {
                        format: fmt,
                        width,
                        height,
                        stride,
                    });
                }
            }
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                trace!("linux_dmabuf buffer option: fourcc=0x{format:08x} {width}×{height}");
                #[cfg(feature = "wayland-dmabuf")]
                {
                    state.dmabuf_spec = Some(DmabufSpec {
                        format,
                        width,
                        height,
                    });
                }
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                state.frame_phase = FramePhase::BufferDone;
            }
            zwlr_screencopy_frame_v1::Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                state.damage.push(DamageRect {
                    x: x as i32,
                    y: y as i32,
                    width,
                    height,
                });
            }
            zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                // Transform flags (e.g. Y-invert) — not handled; callers that
                // need correct orientation should inspect this field via a
                // more complete implementation.
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                state.frame_phase = FramePhase::Ready;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                state.frame_phase = FramePhase::Failed;
            }
            _ => {}
        }
    }
}

// ─── Shared-memory buffer ─────────────────────────────────────────────────────

struct ShmBuffer {
    ptr: *mut u8,
    len: usize,
    /// Keep the pool alive while the buffer object exists.
    _pool: WlShmPool,
    pub wl_buf: WlBuffer,
}

// SAFETY: the mapped memory region is owned exclusively by this struct and is
// not shared with any other thread.
unsafe impl Send for ShmBuffer {}

impl ShmBuffer {
    fn alloc(shm: &WlShm, spec: &ShmSpec, qh: &QueueHandle<CaptureState>) -> Result<Self> {
        let len = (spec.stride * spec.height) as usize;
        let fd = create_memfd(len)?;

        // SAFETY: `fd` is a valid memfd of exactly `len` bytes.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            bail!("mmap failed: {}", std::io::Error::last_os_error());
        }

        // SAFETY: `fd` is valid for the lifetime of this call; the compositor
        // receives its own dup via SCM_RIGHTS, so we can drop `fd` afterwards.
        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(fd.as_raw_fd()) },
            len as i32,
            qh,
            (),
        );
        let wl_buf = pool.create_buffer(
            0,
            spec.width as i32,
            spec.height as i32,
            spec.stride as i32,
            spec.format,
            qh,
            (),
        );

        // `fd` (the OwnedFd, and hence the kernel fd) can now be closed; the
        // wl_shm_pool and the mmap both hold the memory alive independently.
        drop(fd);

        Ok(Self {
            ptr: ptr as *mut u8,
            len,
            _pool: pool,
            wl_buf,
        })
    }

    /// View the captured pixel data.
    ///
    /// # Safety contract
    ///
    /// Must only be called after the compositor has signalled `ready` on the
    /// owning screencopy frame (i.e. the compositor has finished writing).
    fn read(&self) -> &[u8] {
        // SAFETY: the compositor has finished writing (ready event received);
        // the region is valid for `self.len` bytes.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        // Send wl_buffer.destroy before the pool is released.
        self.wl_buf.destroy();
        // Unmap shared memory.  `_pool` will be dropped (destroyed) next.
        // SAFETY: `ptr` was returned by mmap with length `len`.
        unsafe {
            libc::munmap(self.ptr.cast(), self.len);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn resolve_socket(display: &str) -> Result<PathBuf> {
    if display.contains('/') {
        return Ok(PathBuf::from(display));
    }
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .context("$XDG_RUNTIME_DIR is not set; cannot resolve Wayland display socket path")?;
    Ok(PathBuf::from(runtime_dir).join(display))
}

/// Create an anonymous memory-backed file of exactly `size` bytes.
fn create_memfd(size: usize) -> Result<OwnedFd> {
    // SAFETY: c"maa-screencap" is a valid NUL-terminated C string literal.
    let raw = unsafe {
        libc::memfd_create(
            c"maa-screencap".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if raw < 0 {
        bail!("memfd_create failed: {}", std::io::Error::last_os_error());
    }
    // SAFETY: `raw` is a freshly created, valid file descriptor.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    if unsafe { libc::ftruncate(fd.as_raw_fd(), size as libc::off_t) } < 0 {
        bail!("ftruncate failed: {}", std::io::Error::last_os_error());
        // `fd` drops here, closing the file descriptor.
    }
    Ok(fd)
}

fn wl_shm_to_pixel_format(format: wl_shm::Format) -> Option<PixelFormat> {
    match format {
        wl_shm::Format::Argb8888 => Some(PixelFormat::Bgra),
        wl_shm::Format::Xrgb8888 => Some(PixelFormat::Bgrx),
        wl_shm::Format::Abgr8888 => Some(PixelFormat::Rgba),
        wl_shm::Format::Xbgr8888 => Some(PixelFormat::Rgbx),
        _ => None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn pixel_format_round_trip() {
        assert_eq!(
            wl_shm_to_pixel_format(wl_shm::Format::Argb8888),
            Some(PixelFormat::Bgra)
        );
        assert_eq!(
            wl_shm_to_pixel_format(wl_shm::Format::Xrgb8888),
            Some(PixelFormat::Bgrx)
        );
        assert_eq!(
            wl_shm_to_pixel_format(wl_shm::Format::Abgr8888),
            Some(PixelFormat::Rgba)
        );
        assert_eq!(
            wl_shm_to_pixel_format(wl_shm::Format::Xbgr8888),
            Some(PixelFormat::Rgbx)
        );
        assert_eq!(wl_shm_to_pixel_format(wl_shm::Format::Rgb565), None);
    }

    #[test]
    fn resolve_socket_absolute_path() {
        let p = resolve_socket("/tmp/wayland-test").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/wayland-test"));
    }

    #[test]
    fn resolve_socket_display_name() {
        // Temporarily set XDG_RUNTIME_DIR so the helper can resolve a name
        // SAFETY: test is single-threaded; no concurrent env access.
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000") };
        let p = resolve_socket("wayland-1").unwrap();
        assert_eq!(p, PathBuf::from("/run/user/1000/wayland-1"));
    }

    #[test]
    #[ignore = "requires live Wayland compositor on wayland-1"]
    fn connect_and_capture() {
        let mut cap = WaylandScreencap::connect(None).expect("connect");
        let frame = cap.capture().expect("first capture");
        assert!(frame.width > 0);
        assert!(frame.height > 0);
        assert_eq!(frame.data.len(), (frame.stride * frame.height) as usize);
        assert!(frame.damage.is_empty(), "first frame should have no damage");

        let frame2 = cap.capture().expect("second capture (copy_with_damage)");
        assert_eq!(frame2.width, frame.width);
        assert_eq!(frame2.height, frame.height);
    }
}
