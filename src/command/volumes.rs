use ansi_term::Colour::Yellow;
use clap::{App, Arg};
use k8s_openapi::api::core::v1 as api;
use k8s_openapi::List;
use rustyline::completion::Pair as RustlinePair;

use crate::{
    cmd::{exec_match, start_clap, Cmd},
    command::{handle_list_result, Extractor},
    completer,
    env::Env,
    kobj::{KObj, ObjType},
    output::ClickWriter,
    table::CellSpec,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{stderr, Write};

lazy_static! {
    static ref PV_EXTRACTORS: HashMap<String, Extractor<api::PersistentVolume>> = {
        let mut m: HashMap<String, Extractor<api::PersistentVolume>> = HashMap::new();
        m.insert("Capacity".to_owned(), volume_capacity);
        m.insert("Access Modes".to_owned(), volume_access_modes);
        m.insert("Reclaim Policy".to_owned(), volume_reclaim_policy);
        m.insert("Status".to_owned(), volume_status);
        m.insert("Claim".to_owned(), volume_claim);
        m.insert("Storage Class".to_owned(), volume_storage_class);
        m.insert("Reason".to_owned(), volume_reason);
        m
    };
}

fn pv_to_kobj(volume: &api::PersistentVolume) -> KObj {
    //println!("{:?}", volume.spec.as_ref().map(|spec| &spec.claim_ref));
    let meta = &volume.metadata;
    KObj {
        name: meta.name.clone().unwrap_or_else(|| "<Unknown>".into()),
        namespace: meta.namespace.clone(),
        typ: ObjType::PersistentVolume,
    }
}

fn volume_capacity(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume.spec.as_ref().and_then(|spec| {
        spec.capacity
            .get("storage")
            .as_ref()
            .map(|q| q.0.clone().into())
    })
}

fn volume_access_modes(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume.spec.as_ref().map(|spec| {
        spec.access_modes
            .iter()
            .map(|mode| match mode.as_str() {
                "ReadWriteOnce" => "RWO",
                "ReadOnlyMany" => "ROX",
                "ReadWriteMany" => "RWX",
                "ReadWriteOncePod" => "RWOP",
                _ => "Unknown",
            })
            .collect::<Vec<&str>>()
            .join(", ")
            .into()
    })
}

fn volume_reclaim_policy(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume.spec.as_ref().and_then(|spec| {
        spec.persistent_volume_reclaim_policy
            .as_ref()
            .map(|p| p.clone().into())
    })
}

fn volume_status(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume
        .status
        .as_ref()
        .and_then(|stat| stat.phase.as_ref().map(|p| p.as_str().into()))
}

fn volume_claim(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume
        .spec
        .as_ref()
        .map(|spec| match spec.claim_ref.as_ref() {
            Some(claim_ref) => {
                let mut claim = claim_ref.namespace.clone().unwrap_or_else(|| "".into());
                claim.push('/');
                claim.push_str(claim_ref.name.as_deref().unwrap_or(""));
                claim.into()
            }
            None => "".into(),
        })
}

fn volume_storage_class(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume.spec.as_ref().and_then(|spec| {
        spec.storage_class_name
            .as_ref()
            .map(|sc| sc.as_str().into())
    })
}

fn volume_reason(volume: &api::PersistentVolume) -> Option<CellSpec<'_>> {
    volume
        .status
        .as_ref()
        .map(|stat| match stat.reason.as_ref() {
            Some(r) => r.as_str().into(),
            None => "".into(),
        })
}

command!(
    PersistentVolumes,
    "pvs",
    "Get persistent volumes in current context",
    |clap: App<'static, 'static>| clap.arg(
        Arg::with_name("regex")
            .short("r")
            .long("regex")
            .help("Filter pvs by the specified regex")
            .takes_value(true)
    ),
    vec!["persistentvolumes", "pvs"],
    noop_complete!(),
    no_named_complete!(),
    |matches, env, writer| {
        let regex = match crate::table::get_regex(&matches) {
            Ok(r) => r,
            Err(s) => {
                write!(stderr(), "{}\n", s).unwrap_or(());
                return;
            }
        };

        let (request, _response_body) =
            api::PersistentVolume::list_persistent_volume(Default::default()).unwrap();
        let pv_list_opt: Option<List<api::PersistentVolume>> =
            env.run_on_context(|c| c.execute_list(request));
        handle_list_result(
            env,
            writer,
            vec![
                "Name",
                "Age",
                "Capacity",
                "Access Modes",
                "Reclaim Policy",
                "Status",
                "Claim",
                "Storage Class",
                "Reason",
            ],
            pv_list_opt,
            Some(&PV_EXTRACTORS),
            regex,
            None,
            matches.is_present("reverse"),
            pv_to_kobj,
        );
    }
);
