//! Scanners receive only deterministic WASI 0.2 values. All other imported functions trap if called.

use wasmtime::component::{Component, ComponentType, Linker, Lower};
use wasmtime::{Error, StoreContextMut, StoreLimits};

#[derive(ComponentType, Lower)]
#[component(record)]
struct Datetime {
    seconds: u64,
    nanoseconds: u32,
}

pub(super) fn link(
    linker: &mut Linker<StoreLimits>,
    component: &Component,
) -> wasmtime::Result<()> {
    for (name, _) in component.component_type().imports(component.engine()) {
        let Some((interface, _)) = name.split_once("@0.2.") else {
            continue;
        };
        match interface {
            "wasi:cli/environment" => {
                let mut api = linker.instance(name)?;
                api.func_wrap("get-environment", |_, ()| {
                    Ok((Vec::<(String, String)>::new(),))
                })?;
                api.func_wrap("get-arguments", |_, ()| Ok((Vec::<String>::new(),)))?;
                api.func_wrap("initial-cwd", |_, ()| Ok((None::<String>,)))?;
            }
            "wasi:clocks/wall-clock" => {
                let mut api = linker.instance(name)?;
                api.func_wrap("now", |_, ()| {
                    Ok((Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    },))
                })?;
                api.func_wrap("resolution", |_, ()| {
                    Ok((Datetime {
                        seconds: 0,
                        nanoseconds: 1,
                    },))
                })?;
            }
            "wasi:clocks/monotonic-clock" => {
                let mut api = linker.instance(name)?;
                api.func_wrap("now", |_, ()| Ok((0u64,)))?;
                api.func_wrap("resolution", |_, ()| Ok((1u64,)))?;
            }
            "wasi:random/insecure-seed" => {
                linker
                    .instance(name)?
                    .func_wrap("insecure-seed", |_, ()| Ok(((0u64, 0u64),)))?;
            }
            "wasi:random/random" => {
                let mut api = linker.instance(name)?;
                api.func_wrap("get-random-bytes", random_bytes)?;
                api.func_wrap("get-random-u64", |_, ()| Ok((0u64,)))?;
            }
            "wasi:random/insecure" => {
                let mut api = linker.instance(name)?;
                api.func_wrap("get-insecure-random-bytes", random_bytes)?;
                api.func_wrap("get-insecure-random-u64", |_, ()| Ok((0u64,)))?;
            }
            _ => {}
        }
    }
    linker.define_unknown_imports_as_traps(component)
}

fn random_bytes(
    mut store: StoreContextMut<'_, StoreLimits>,
    (len,): (u64,),
) -> wasmtime::Result<(Vec<u8>,)> {
    if len > 65_536 {
        return Err(Error::msg("random byte request exceeds the scanner limit"));
    }
    let remaining = store
        .get_fuel()?
        .checked_sub(len)
        .ok_or_else(|| Error::msg("random byte request exhausted fuel"))?;
    store.set_fuel(remaining)?;
    Ok((vec![0; len as usize],))
}
