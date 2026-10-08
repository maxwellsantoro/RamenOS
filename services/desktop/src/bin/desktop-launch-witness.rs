//! Deterministic Rust host child. Its inherited private pipes carry generated
//! desktop messages; it has no parent fixture PeerContext or broker API access.
use desktop_service::dev::{DesktopMessage, encode_message, read_frame, write_frame};
use kernel_api::cap::Handle;
use kernel_api::generated::desktop_session_v1 as idl;
use std::io::{stdin, stdout};

fn main() {
    let mode = std::env::args().nth(1).expect("explicit witness mode");
    if mode == "stall" {
        loop {
            std::hint::spin_loop();
        }
    }
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    let bootstrap = idl::InstanceBootstrap::from_envelope(
        &read_frame(&mut input).expect("private bootstrap frame"),
    )
    .expect("generated bootstrap fields");
    if mode == "fault" {
        std::process::exit(23);
    }
    let handle = Handle::unpack(bootstrap.observation_handle);
    let observe = |request_id, session_id, instance_id| idl::ObserveInstance {
        request_id,
        session_id,
        session_generation: bootstrap.session_generation,
        instance_id,
        instance_generation: bootstrap.instance_generation,
    };
    if mode == "flood" {
        for index in 0..65 {
            let request = observe(100 + index, bootstrap.session_id, bootstrap.instance_id);
            write_frame(&mut output, &encode_message(handle, &request)).expect("flood request");
            let reply = read_frame(&mut input).expect("bounded flood reply");
            assert_eq!(
                idl::ObserveInstanceReply::from_envelope(&reply)
                    .unwrap()
                    .status,
                0
            );
        }
        panic!("65th exchange should not be acknowledged");
    }
    assert_eq!(mode, "observe");
    let request = observe(1, bootstrap.session_id, bootstrap.instance_id);
    write_frame(&mut output, &encode_message(handle, &request)).unwrap();
    assert_eq!(
        idl::ObserveInstanceReply::from_envelope(&read_frame(&mut input).unwrap())
            .unwrap()
            .status,
        0
    );

    let prepare = idl::PrepareLaunch {
        request_id: 2,
        session_id: bootstrap.session_id,
        session_generation: bootstrap.session_generation,
        application_hash: [0; 32],
        requested_rights: 1,
        reserved: 0,
    };
    write_frame(&mut output, &encode_message(handle, &prepare)).unwrap();
    assert_eq!(
        idl::PrepareLaunchReply::from_envelope(&read_frame(&mut input).unwrap())
            .unwrap()
            .status,
        1
    );
    let confirm = idl::ConfirmLaunch {
        request_id: 3,
        session_id: bootstrap.session_id,
        session_generation: bootstrap.session_generation,
        plan_id: 0,
        preview_revision: 0,
    };
    write_frame(&mut output, &encode_message(handle, &confirm)).unwrap();
    assert_eq!(
        idl::ConfirmLaunchReply::from_envelope(&read_frame(&mut input).unwrap())
            .unwrap()
            .status,
        1
    );
    let restart = idl::PrepareRestart {
        request_id: 4,
        session_id: bootstrap.session_id,
        session_generation: bootstrap.session_generation,
        instance_id: bootstrap.instance_id,
        instance_generation: bootstrap.instance_generation,
    };
    write_frame(&mut output, &encode_message(handle, &restart)).unwrap();
    assert_eq!(
        idl::PrepareRestartReply::from_envelope(&read_frame(&mut input).unwrap())
            .unwrap()
            .status,
        1
    );
    let foreign = observe(
        5,
        bootstrap.session_id.checked_add(1).unwrap(),
        bootstrap.instance_id.checked_add(1).unwrap(),
    );
    write_frame(&mut output, &encode_message(handle, &foreign)).unwrap();
    assert_eq!(
        idl::ObserveInstanceReply::from_envelope(&read_frame(&mut input).unwrap())
            .unwrap()
            .status,
        1
    );
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
