use std::{fs, path::PathBuf};

#[test]
fn bindings_match_v150_feature() {
    let bindings = fs::read_to_string(PathBuf::from(env!("OUT_DIR")).join("bindings.rs"))
        .expect("read generated bindings");

    if cfg!(feature = "v150") {
        for declaration in [
            "pub fn sprt_init(",
            "pub fn v150_1_init(",
            "pub fn v150_1_rx_sse_packet(",
            "pub type sprt_state_t",
            "pub type v150_1_state_t",
        ] {
            assert!(bindings.contains(declaration), "missing {declaration}");
        }
    } else {
        for prefix in ["sprt_", "v150_1_", "SPRT_", "V150_1_"] {
            assert!(!bindings.contains(prefix), "unexpected {prefix} binding");
        }
    }
}

#[cfg(unix)]
#[test]
fn native_archive_matches_v150_feature() {
    // Inspect the archive itself: a successful link alone could hide unwanted
    // objects through dead stripping.
    let output = std::process::Command::new("ar")
        .arg("t")
        .arg(PathBuf::from(env!("OUT_DIR")).join("libspandsp.a"))
        .output()
        .expect("list native archive members");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let members = String::from_utf8(output.stdout).unwrap();
    for object in ["sprt.o", "v150_1.o", "v150_1_sse.o"] {
        assert_eq!(
            members.lines().any(|member| member.ends_with(object)),
            cfg!(feature = "v150"),
            "archive member {object} does not match v150 feature"
        );
    }
}

#[cfg(feature = "v150")]
#[test]
fn opted_in_apis_link_and_run() {
    // Call an API from each gated translation unit to verify usable bindings
    // and native linkage, including their dependencies.
    unsafe {
        for value in [
            spandsp_sys::sprt_transmission_channel_to_str(-1),
            spandsp_sys::v150_1_msg_id_to_str(-1),
            spandsp_sys::v150_1_sse_moip_ric_to_str(-1),
        ] {
            assert!(!value.is_null());
            assert_eq!(std::ffi::CStr::from_ptr(value).to_bytes(), b"unknown");
        }
    }
}
