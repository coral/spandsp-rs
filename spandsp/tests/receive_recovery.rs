#![cfg(feature = "fax")]
use spandsp::{fax::FaxState, t30::T30State, t38_terminal::T38Terminal, *};
use spandsp_sys as sys;
use std::{
    collections::VecDeque,
    ffi::{CString, c_char, c_int, c_void},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

#[link(name = "tiff")]
unsafe extern "C" {
    fn TIFFOpen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn TIFFClose(t: *mut c_void);
    fn TIFFSetField(t: *mut c_void, tag: u32, ...) -> c_int;
    fn TIFFGetField(t: *mut c_void, tag: u32, ...) -> c_int;
    fn TIFFWriteScanline(t: *mut c_void, buf: *mut c_void, row: u32, sample: u16) -> c_int;
    fn TIFFReadScanline(t: *mut c_void, buf: *mut c_void, row: u32, sample: u16) -> c_int;
    fn TIFFWriteDirectory(t: *mut c_void) -> c_int;
    fn TIFFReadDirectory(t: *mut c_void) -> c_int;
}
const WIDTH: u32 = 1728;
const ROWS: u32 = 800;
fn pixels(page: u32, row: u32) -> Vec<u8> {
    (0..WIDTH / 8)
        .map(|x| {
            if (x + row / 9 + page * 3).is_multiple_of(2) {
                0xff
            } else {
                0
            }
        })
        .collect()
}
struct Files {
    dir: PathBuf,
    input: PathBuf,
    output: PathBuf,
}
impl Files {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "spandsp-recovery-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.tif");
        let output = dir.join("output.tif");
        unsafe {
            let name = CString::new(input.to_str().unwrap()).unwrap();
            let t = TIFFOpen(name.as_ptr(), c"w".as_ptr());
            assert!(!t.is_null());
            for p in 0..2 {
                for (tag, value) in [
                    (256, WIDTH),
                    (257, ROWS),
                    (258, 1),
                    (259, 1),
                    (262, 0),
                    (277, 1),
                    (278, ROWS),
                    (284, 1),
                    (296, 2),
                ] {
                    assert_eq!(TIFFSetField(t, tag, value), 1);
                }
                TIFFSetField(t, 282, 204.0f64);
                TIFFSetField(t, 283, 196.0f64);
                for r in 0..ROWS {
                    assert_eq!(
                        TIFFWriteScanline(t, pixels(p, r).as_mut_ptr().cast(), r, 0),
                        1
                    );
                }
                assert_eq!(TIFFWriteDirectory(t), 1);
            }
            TIFFClose(t);
        }
        Self { dir, input, output }
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn read_output(path: &Path) -> Vec<Vec<Vec<u8>>> {
    if !path.exists() {
        return vec![];
    }
    unsafe {
        let name = CString::new(path.to_str().unwrap()).unwrap();
        let t = TIFFOpen(name.as_ptr(), c"r".as_ptr());
        assert!(!t.is_null());
        let mut pages = vec![];
        loop {
            let (mut w, mut h) = (0u32, 0u32);
            assert_eq!(TIFFGetField(t, 256, &mut w), 1);
            assert_eq!(TIFFGetField(t, 257, &mut h), 1);
            assert_eq!(w, WIDTH);
            assert!(h > 0 && h <= ROWS, "height {h}");
            let mut rows = vec![];
            for r in 0..h {
                let mut row = vec![0u8; w as usize / 8];
                assert_eq!(TIFFReadScanline(t, row.as_mut_ptr().cast(), r, 0), 1);
                rows.push(row);
            }
            pages.push(rows);
            if TIFFReadDirectory(t) == 0 {
                break;
            }
        }
        TIFFClose(t);
        pages
    }
}
fn config(tx: T30State<'_>, rx: T30State<'_>, files: &Files, ecm: bool) {
    tx.set_tx_file(files.input.to_str().unwrap(), -1, -1)
        .unwrap();
    rx.set_rx_file(files.output.to_str().unwrap(), -1).unwrap();
    for t in [&tx, &rx] {
        t.set_ecm_capability(ecm).unwrap();
        t.set_supported_compressions(t4::T4Compression::T4_1D.bits() as i32)
            .unwrap();
        t.set_supported_modems(spandsp::t30::T30ModemSupport::V17)
            .unwrap();
    }
    unsafe {
        sys::t30_set_supported_output_compressions(
            rx.as_ptr(),
            spandsp::t4::T4Compression::T6.bits() as i32,
        );
    }
}
#[derive(Default)]
struct Callbacks {
    phases: usize,
    completion: Option<i32>,
}
unsafe extern "C" fn phase(user: *mut c_void, _: i32) -> i32 {
    unsafe {
        (*user.cast::<Callbacks>()).phases += 1;
    }
    0
}
unsafe extern "C" fn phase_e(user: *mut c_void, code: i32) {
    unsafe {
        (*user.cast::<Callbacks>()).completion = Some(code);
    }
}
fn callbacks(t: T30State<'_>) -> Box<Callbacks> {
    let mut cb = Box::<Callbacks>::default();
    let user = (&mut *cb as *mut Callbacks).cast();
    unsafe {
        t.set_phase_b_handler_raw(Some(phase), user);
        t.set_phase_d_handler_raw(Some(phase), user);
        t.set_phase_e_handler_raw(Some(phase_e), user);
    }
    cb
}
#[derive(Debug, Clone, Copy)]
enum Stop {
    BeforeImage,
    FirstPage,
    BetweenPages,
    SecondPage,
    Success,
    FailureAfterImage,
    Corrupt,
    Timeout,
}
fn should_stop(stop: Stop, stats: &sys::t30_stats_t, active: bool) -> bool {
    match stop {
        Stop::Corrupt | Stop::Timeout => false,
        Stop::BeforeImage => true,
        Stop::FirstPage => stats.pages_rx == 0 && stats.length > 200,
        Stop::BetweenPages => stats.pages_rx == 1,
        Stop::SecondPage => stats.pages_rx == 1 && stats.length > 200 && stats.length < ROWS as i32,
        Stop::Success => !active,
        Stop::FailureAfterImage => stats.pages_rx == 2,
    }
}
fn verify(files: &Files, report: &ReceiveReport, stop: Stop, enabled: bool) {
    assert!(report.output_closed, "{report:?}");
    assert_eq!(report.output_error, None, "{report:?}");
    let output = read_output(&files.output);
    assert_eq!(output.len(), report.pages.len(), "{report:?}");
    for (p, (image, meta)) in output.iter().zip(&report.pages).enumerate() {
        assert_eq!(image.len(), meta.decoded_rows as usize);
        assert_eq!(meta.pixel_width, WIDTH);
        assert_eq!(meta.decoder_bad_rows, 0);
        assert!(meta.horizontal_resolution > 0 && meta.vertical_resolution > 0);
        for (r, row) in image.iter().enumerate() {
            assert_eq!(*row, pixels(p as u32, r as u32), "page {p} row {r}");
        }
    }
    match stop {
        Stop::BeforeImage => {
            assert!(output.is_empty());
            assert_eq!(report.confirmed_complete_pages, 0);
        }
        Stop::FirstPage | Stop::Corrupt | Stop::Timeout => {
            assert_eq!(report.confirmed_complete_pages, 0);
            if enabled {
                assert_eq!(output.len(), 1);
                assert_eq!(report.pages[0].kind, ReceivePageKind::RecoveredPartial);
            } else {
                assert!(output.is_empty());
            }
        }
        Stop::BetweenPages => {
            assert_eq!(report.confirmed_complete_pages, 1);
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].len(), ROWS as usize);
        }
        Stop::SecondPage => {
            assert_eq!(report.confirmed_complete_pages, 1);
            assert_eq!(output[0].len(), ROWS as usize);
            assert_eq!(output.len(), if enabled { 2 } else { 1 });
        }
        Stop::Success => {
            assert_eq!(report.completion, ReceiveCompletion::Completed { code: 0 });
            assert_eq!(report.confirmed_complete_pages, 2);
            assert_eq!(output.len(), 2);
        }
        Stop::FailureAfterImage => {
            assert_eq!(report.confirmed_complete_pages, 2);
            assert_eq!(output.len(), 2);
            assert!(!matches!(
                report.completion,
                ReceiveCompletion::Completed { code: 0 }
            ));
        }
    }
    if matches!(stop, Stop::Timeout) {
        assert!(matches!(report.completion,ReceiveCompletion::Completed{code} if code != 0));
    }
    if !matches!(
        stop,
        Stop::Success | Stop::FailureAfterImage | Stop::Timeout
    ) {
        assert!(matches!(
            report.completion,
            ReceiveCompletion::Interrupted { .. }
        ));
    }
}
fn audio(ecm: bool, alaw: bool, stop: Stop, enabled: bool) {
    let files = Files::new();
    let tx = FaxState::new(true).unwrap();
    let mut rx = FaxState::new_receiver(if enabled {
        ReceiveRecovery::PreserveDecodedRows
    } else {
        ReceiveRecovery::Disabled
    })
    .unwrap();
    config(
        tx.get_t30_state().unwrap(),
        rx.get_t30_state().unwrap(),
        &files,
        ecm,
    );
    let cb = callbacks(rx.get_t30_state().unwrap());
    tx.set_transmit_on_idle(true);
    rx.set_transmit_on_idle(true);
    let mut stopped = false;
    let mut fault_at = None;
    for step in 0..30000 {
        let stats = rx.get_t30_state().unwrap().get_transfer_statistics();
        if matches!(stop, Stop::Corrupt | Stop::Timeout)
            && fault_at.is_none()
            && stats.pages_rx == 0
            && stats.length > 200
        {
            fault_at = Some(step);
        }
        if should_stop(stop, &stats, rx.get_t30_state().unwrap().call_active())
            || matches!(stop, Stop::Corrupt) && fault_at.is_some_and(|at| step >= at + 100)
            || matches!(stop, Stop::Timeout)
                && fault_at.is_some()
                && !rx.get_t30_state().unwrap().call_active()
        {
            stopped = true;
            break;
        }
        let mut a = [0i16; 160];
        let mut b = [0i16; 160];
        tx.tx(&mut a);
        rx.tx(&mut b);
        for sample in a.iter_mut().chain(b.iter_mut()) {
            *sample = if alaw {
                g711::alaw_to_linear(g711::linear_to_alaw(*sample))
            } else {
                g711::ulaw_to_linear(g711::linear_to_ulaw(*sample))
            };
        }
        if fault_at.is_some() {
            if matches!(stop, Stop::Timeout) {
                a.fill(0);
                b.fill(0);
            } else {
                for (i, sample) in a.iter_mut().enumerate() {
                    *sample = if i % 2 == 0 { 16000 } else { -16000 };
                }
            }
        }
        rx.rx(&mut a);
        tx.rx(&mut b);
    }
    assert!(stopped, "audio {ecm} {alaw} {stop:?}");
    if matches!(stop, Stop::FailureAfterImage) {
        unsafe {
            sys::t30_set_status(
                rx.get_t30_state().unwrap().as_ptr(),
                sys::t30_err_e::T30_ERR_RX_DCNPHD as i32,
            );
            sys::t30_terminate(rx.get_t30_state().unwrap().as_ptr());
        }
    }
    let report = rx.finalize_receive().clone();
    match report.completion {
        ReceiveCompletion::Completed { code } => assert_eq!(cb.completion, Some(code)),
        ReceiveCompletion::Interrupted { .. } => assert_eq!(cb.completion, None),
    }
    if !matches!(stop, Stop::BeforeImage) {
        assert!(cb.phases > 0);
    }
    drop(cb);
    let bytes = std::fs::read(&files.output).ok();
    assert_eq!(
        format!("{report:?}"),
        format!("{:?}", rx.finalize_receive())
    );
    assert_eq!(bytes, std::fs::read(&files.output).ok());
    assert_eq!(rx.tx(&mut [0; 160]), 0);
    assert_eq!(rx.rx(&mut [0; 160]), 160);
    verify(&files, &report, stop, enabled);
    drop(rx);
    assert_eq!(bytes, std::fs::read(&files.output).ok());
}
#[derive(Default)]
struct Packets {
    queue: VecDeque<(u16, Vec<u8>)>,
    seq: u16,
}
unsafe extern "C" fn packet(
    _: *mut sys::t38_core_state_t,
    user: *mut c_void,
    data: *const u8,
    len: i32,
    _count: i32,
) -> i32 {
    unsafe {
        let q = &mut *user.cast::<Packets>();
        q.queue.push_back((
            q.seq,
            std::slice::from_raw_parts(data, len as usize).to_vec(),
        ));
        q.seq = q.seq.wrapping_add(1);
    }
    0
}
fn t38(ecm: bool, stop: Stop, enabled: bool) {
    let files = Files::new();
    let mut a = Box::<Packets>::default();
    let mut b = Box::<Packets>::default();
    let tx = unsafe {
        T38Terminal::new_raw(true, Some(packet), (&mut *a as *mut Packets).cast()).unwrap()
    };
    let mut rx = unsafe {
        T38Terminal::new_receiver_raw(
            if enabled {
                ReceiveRecovery::PreserveDecodedRows
            } else {
                ReceiveRecovery::Disabled
            },
            Some(packet),
            (&mut *b as *mut Packets).cast(),
        )
        .unwrap()
    };
    config(
        tx.get_t30_state().unwrap(),
        rx.get_t30_state().unwrap(),
        &files,
        ecm,
    );
    let cb = callbacks(rx.get_t30_state().unwrap());
    let mut stopped = false;
    let mut fault_at = None;
    for step in 0..30000 {
        let stats = rx.get_t30_state().unwrap().get_transfer_statistics();
        if matches!(stop, Stop::Corrupt | Stop::Timeout)
            && fault_at.is_none()
            && stats.pages_rx == 0
            && stats.length > 200
        {
            fault_at = Some(step);
        }
        if should_stop(stop, &stats, rx.get_t30_state().unwrap().call_active())
            || matches!(stop, Stop::Corrupt) && fault_at.is_some_and(|at| step >= at + 100)
            || matches!(stop, Stop::Timeout)
                && fault_at.is_some()
                && !rx.get_t30_state().unwrap().call_active()
        {
            stopped = true;
            break;
        }
        tx.send_timeout(160);
        rx.send_timeout(160);
        while let Some((seq, p)) = a.queue.pop_front() {
            if matches!(stop, Stop::Timeout) && fault_at.is_some() {
                continue;
            }
            if matches!(stop, Stop::Corrupt) && fault_at == Some(step) {
                continue;
            }
            rx.get_t38_core_state()
                .unwrap()
                .rx_ifp_packet(&p, seq)
                .unwrap();
        }
        while let Some((seq, p)) = b.queue.pop_front() {
            tx.get_t38_core_state()
                .unwrap()
                .rx_ifp_packet(&p, seq)
                .unwrap();
        }
    }
    assert!(stopped, "t38 {ecm} {stop:?}");
    if matches!(stop, Stop::FailureAfterImage) {
        unsafe {
            sys::t30_set_status(
                rx.get_t30_state().unwrap().as_ptr(),
                sys::t30_err_e::T30_ERR_RX_DCNPHD as i32,
            );
            sys::t30_terminate(rx.get_t30_state().unwrap().as_ptr());
        }
    }
    let report = rx.finalize_receive().clone();
    match report.completion {
        ReceiveCompletion::Completed { code } => assert_eq!(cb.completion, Some(code)),
        ReceiveCompletion::Interrupted { .. } => assert_eq!(cb.completion, None),
    }
    if !matches!(stop, Stop::BeforeImage) {
        assert!(cb.phases > 0);
    }
    drop(cb);
    let bytes = std::fs::read(&files.output).ok();
    drop(b); // Callback storage can be freed before repeated finalize / media / Drop.
    assert_eq!(rx.send_timeout(160), 1);
    assert!(rx.get_t38_core_state().is_err());
    assert_eq!(
        format!("{report:?}"),
        format!("{:?}", rx.finalize_receive())
    );
    verify(&files, &report, stop, enabled);
    drop(rx);
    assert_eq!(bytes, std::fs::read(&files.output).ok());
}
#[test]
fn audio_matrix() {
    for ecm in [false, true] {
        for alaw in [false, true] {
            for enabled in [false, true] {
                for stop in [
                    Stop::BeforeImage,
                    Stop::FirstPage,
                    Stop::BetweenPages,
                    Stop::SecondPage,
                    Stop::Success,
                    Stop::FailureAfterImage,
                    Stop::Corrupt,
                    Stop::Timeout,
                ] {
                    audio(ecm, alaw, stop, enabled);
                }
            }
        }
    }
}
#[test]
fn t38_matrix() {
    for ecm in [false, true] {
        for enabled in [false, true] {
            for stop in [
                Stop::BeforeImage,
                Stop::FirstPage,
                Stop::BetweenPages,
                Stop::SecondPage,
                Stop::Success,
                Stop::FailureAfterImage,
                Stop::Corrupt,
                Stop::Timeout,
            ] {
                t38(ecm, stop, enabled);
            }
        }
    }
}
