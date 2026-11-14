// Copyright 2026 Aerospike, Inc.
//
// Portions may be licensed to Aerospike, Inc. under one or more contributor
// license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

//! Immortal classes on free-threaded Python.
//!
//! Every instance holds a reference to its class, and freeing an instance
//! releases it. On a free-threaded build that release is an atomic write to
//! the class's shared reference count, so threads that free many keys,
//! records and batch rows contend on one counter per class. An immortal
//! class is never counted, which leaves nothing to contend on.
//!
//! The interpreter's own call for this refuses an object with more than one
//! reference, which a class always has, so the header is written directly,
//! the way the interpreter writes it. The header layout is private to the
//! interpreter: only versions whose layout has been checked are touched.

use pyo3::prelude::*;

/// Set to any value to leave the classes as the interpreter made them.
#[cfg(Py_GIL_DISABLED)]
const OPT_OUT_ENV: &str = "AEROSPIKE_NATIVE_MORTAL_CLASSES";

/// Interpreter minor versions whose object header matches what is written here.
#[cfg(Py_GIL_DISABLED)]
const CHECKED_MINORS: std::ops::RangeInclusive<u8> = 14..=15;

/// What this extension's classes report as their module. The scan below
/// touches nothing else: a class from another package may already be in use
/// on other threads, and its header is not ours to write. A genuine builtin
/// class is static, so the heap-type check keeps those out.
#[cfg(Py_GIL_DISABLED)]
const OWN_CLASS_MODULES: [&str; 2] = ["_native", "builtins"];

/// The interpreter's marker for a local reference count that is never changed.
#[cfg(Py_GIL_DISABLED)]
const IMMORTAL_REFCNT_LOCAL: u32 = u32::MAX;

/// The interpreter's GC bit for an object with deferred reference counting.
#[cfg(Py_GIL_DISABLED)]
const GC_BITS_DEFERRED: u8 = 1 << 6;

/// Makes every class in `module` immortal, and the classes created once per
/// stream step, which are in no module namespace. Call once, at the end of
/// module initialization, while the importing thread is the only one that
/// can see the classes.
#[cfg(Py_GIL_DISABLED)]
pub(crate) fn make_classes_immortal(module: &Bound<'_, PyModule>) -> PyResult<()> {
    use pyo3::ffi;
    use pyo3::types::PyType;
    use std::sync::atomic::Ordering::Relaxed;

    let version = module.py().version_info();
    if version.major != 3
        || !CHECKED_MINORS.contains(&version.minor)
        || std::env::var_os(OPT_OUT_ENV).is_some()
    {
        return Ok(());
    }

    let py = module.py();
    let unregistered = [
        py.get_type::<crate::stream_step::ReadyStep>().into_any(),
        py.get_type::<crate::stream_step::YieldFirstStep>().into_any(),
    ];
    for value in module.dict().values().into_iter().chain(unregistered) {
        let Ok(class) = value.cast::<PyType>() else {
            continue;
        };
        let own = class
            .module()
            .is_ok_and(|name| OWN_CLASS_MODULES.iter().any(|own| name == *own));
        if !own {
            continue;
        }
        let op = class.as_ptr();
        // SAFETY: `op` is a live class object and the thread is attached. No
        // other thread can reach the class before module initialization
        // returns, so nothing reads the header while it is written. The
        // fields and values are the ones the interpreter itself writes to
        // make an object immortal on the versions checked above.
        unsafe {
            if ffi::PyType_HasFeature(class.as_type_ptr(), ffi::Py_TPFLAGS_HEAPTYPE) == 0 {
                continue;
            }
            if ffi::PyObject_GC_IsTracked(op) != 0 {
                ffi::PyObject_GC_UnTrack(op.cast());
            }
            (*op).ob_tid = 0;
            (*op).ob_ref_local.store(IMMORTAL_REFCNT_LOCAL, Relaxed);
            (*op).ob_ref_shared.store(0, Relaxed);
            (*op).ob_gc_bits |= GC_BITS_DEFERRED;
        }
    }
    Ok(())
}

/// Classes are only contended on a free-threaded build.
#[cfg(not(Py_GIL_DISABLED))]
pub(crate) fn make_classes_immortal(_module: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
