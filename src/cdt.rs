// Copyright 2023-2026 Aerospike, Inc.
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

use std::collections::BTreeMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};



use crate::record::PythonValue;

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListOrderType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "ListOrderType", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ListOrderType {
        /// List is not ordered. This is the default.
        #[pyo3(name = "UNORDERED")]
        Unordered,
        /// List is ordered.
        #[pyo3(name = "ORDERED")]
        Ordered,
    }

    #[pymethods]
    impl ListOrderType {
        fn __richcmp__(&self, other: &ListOrderType, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Ok(false),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    impl From<&ListOrderType> for aerospike_core::operations::lists::ListOrderType {
        fn from(input: &ListOrderType) -> Self {
            match input {
                ListOrderType::Unordered => aerospike_core::operations::lists::ListOrderType::Unordered,
                ListOrderType::Ordered => aerospike_core::operations::lists::ListOrderType::Ordered,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListWriteFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "ListWriteFlags", module = "aerospike_native._native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ListWriteFlags {
        /// Default is the default behavior. It means: Allow duplicate values and insertions at any index.
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        /// AddUnique means: Only add unique values.
        #[pyo3(name = "ADD_UNIQUE")]
        AddUnique = 1,
        /// InsertBounded means: Enforce list boundaries when inserting. Do not allow values to be inserted at index outside current list boundaries.
        #[pyo3(name = "INSERT_BOUNDED")]
        InsertBounded = 2,
        /// NoFail means: do not raise error if a list item fails due to write flag constraints.
        #[pyo3(name = "NO_FAIL")]
        NoFail = 4,
        /// Partial means: allow other valid list items to be committed if a list item fails due to write flag constraints.
        #[pyo3(name = "PARTIAL")]
        Partial = 8,
    }

    #[pymethods]
    impl ListWriteFlags {
        /// Bitwise OR of list write flags (bitmask). Result is an ``int`` suitable for ``ListPolicy``.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(a | b)
        }

        /// ``int | ListWriteFlags`` support.
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(b | a)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(a & b)
        }

        /// ``int & ListWriteFlags`` support.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// ``int ^ ListWriteFlags`` support.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = list_policy_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width).
        fn __invert__(&self) -> u8 {
            !u8::from(*self)
        }

        /// Equality / ordering against another ``ListWriteFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match list_policy_flags_from_py(other) {
                Ok(v) => v as i64,
                Err(_) => {
                    return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
                }
            };
            Ok(match op {
                pyo3::class::basic::CompareOp::Eq => a == b,
                pyo3::class::basic::CompareOp::Ne => a != b,
                pyo3::class::basic::CompareOp::Lt => a < b,
                pyo3::class::basic::CompareOp::Le => a <= b,
                pyo3::class::basic::CompareOp::Gt => a > b,
                pyo3::class::basic::CompareOp::Ge => a >= b,
            })
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    impl From<&ListWriteFlags> for aerospike_core::operations::lists::ListWriteFlags {
        fn from(input: &ListWriteFlags) -> Self {
            match input {
                ListWriteFlags::Default => aerospike_core::operations::lists::ListWriteFlags::DEFAULT,
                ListWriteFlags::AddUnique => aerospike_core::operations::lists::ListWriteFlags::ADD_UNIQUE,
                ListWriteFlags::InsertBounded => aerospike_core::operations::lists::ListWriteFlags::INSERT_BOUNDED,
                ListWriteFlags::NoFail => aerospike_core::operations::lists::ListWriteFlags::NO_FAIL,
                ListWriteFlags::Partial => aerospike_core::operations::lists::ListWriteFlags::PARTIAL,
            }
        }
    }

    impl From<ListWriteFlags> for u8 {
        fn from(flags: ListWriteFlags) -> Self {
            flags as u8
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListReturnType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// ListReturnType - supports bitwise OR for combining with INVERTED flag.
    /// 
    /// Example:
    ///     combined = ListReturnType.VALUE | ListReturnType.INVERTED
    #[gen_stub_pyclass]
    #[pyclass(from_py_object, name = "ListReturnType", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ListReturnType(u32);

    type CoreListRT = aerospike_core::operations::lists::ListReturnType;

    #[gen_stub_pymethods]
    #[pymethods]
    impl ListReturnType {
        /// Do not return a result.
        #[classattr]
        const NONE: ListReturnType = ListReturnType(CoreListRT::NONE.bits() as u32);
        /// Return index offset order.
        #[classattr]
        const INDEX: ListReturnType = ListReturnType(CoreListRT::INDEX.bits() as u32);
        /// Return reverse index offset order.
        #[classattr]
        const REVERSE_INDEX: ListReturnType = ListReturnType(CoreListRT::REVERSE_INDEX.bits() as u32);
        /// Return value order.
        #[classattr]
        const RANK: ListReturnType = ListReturnType(CoreListRT::RANK.bits() as u32);
        /// Return reverse value order.
        #[classattr]
        const REVERSE_RANK: ListReturnType = ListReturnType(CoreListRT::REVERSE_RANK.bits() as u32);
        /// Return count of items selected.
        #[classattr]
        const COUNT: ListReturnType = ListReturnType(CoreListRT::COUNT.bits() as u32);
        /// Return value for single key read and value list for range read.
        #[classattr]
        const VALUE: ListReturnType = ListReturnType(CoreListRT::VALUES.bits() as u32);
        /// Return true if count > 0.
        #[classattr]
        const EXISTS: ListReturnType = ListReturnType(CoreListRT::EXISTS.bits() as u32);
        /// Invert meaning of list command and return values.
        /// Can be OR'd with other return types: VALUE | INVERTED
        #[classattr]
        const INVERTED: ListReturnType = ListReturnType(CoreListRT::NONE.inverted().bits() as u32);

        /// Bitwise OR - allows combining return type with INVERTED flag
        fn __or__(&self, other: &Self) -> Self {
            Self(self.0 | other.0)
        }

        /// Bitwise AND
        fn __and__(&self, other: &Self) -> Self {
            Self(self.0 & other.0)
        }

        /// Convert to integer
        fn __int__(&self) -> u32 {
            self.0
        }

        fn __richcmp__(&self, other: &ListReturnType, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                pyo3::class::basic::CompareOp::Lt => Ok(self.0 < other.0),
                pyo3::class::basic::CompareOp::Le => Ok(self.0 <= other.0),
                pyo3::class::basic::CompareOp::Gt => Ok(self.0 > other.0),
                pyo3::class::basic::CompareOp::Ge => Ok(self.0 >= other.0),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }

        fn __repr__(&self) -> String {
            let base = self.0 & 0xFFFF;
            let inverted = (self.0 & 0x10000) != 0;
            let base_name = match base {
                x if x == CoreListRT::NONE.bits() as u32 => "NONE",
                x if x == CoreListRT::INDEX.bits() as u32 => "INDEX",
                x if x == CoreListRT::REVERSE_INDEX.bits() as u32 => "REVERSE_INDEX",
                x if x == CoreListRT::RANK.bits() as u32 => "RANK",
                x if x == CoreListRT::REVERSE_RANK.bits() as u32 => "REVERSE_RANK",
                x if x == CoreListRT::COUNT.bits() as u32 => "COUNT",
                x if x == CoreListRT::VALUES.bits() as u32 => "VALUE",
                x if x == CoreListRT::EXISTS.bits() as u32 => "EXISTS",
                _ => "UNKNOWN",
            };
            if inverted && base != 0 {
                format!("ListReturnType.{} | ListReturnType.INVERTED", base_name)
            } else if inverted {
                "ListReturnType.INVERTED".to_string()
            } else {
                format!("ListReturnType.{}", base_name)
            }
        }
    }

    impl From<ListReturnType> for CoreListRT {
        fn from(input: ListReturnType) -> Self {
            CoreListRT::from_bits(input.0 as i64)
        }
    }

    impl From<&ListReturnType> for CoreListRT {
        fn from(input: &ListReturnType) -> Self {
            CoreListRT::from_bits(input.0 as i64)
        }
    }
    
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListSortFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "ListSortFlags", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ListSortFlags {
        /// Default. Preserve duplicate values when sorting list.
        #[pyo3(name = "DEFAULT")]
        Default,
        /// Sort in descending order.
        #[pyo3(name = "DESCENDING")]
        Descending,
        /// Drop duplicate values when sorting list.
        #[pyo3(name = "DROP_DUPLICATES")]
        DropDuplicates,
    }

    #[pymethods]
    impl ListSortFlags {
        fn __richcmp__(&self, other: &ListSortFlags, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Ok(false),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    impl From<&ListSortFlags> for aerospike_core::operations::lists::ListSortFlags {
        fn from(input: &ListSortFlags) -> Self {
            match input {
                ListSortFlags::Default => aerospike_core::operations::lists::ListSortFlags::DEFAULT,
                ListSortFlags::Descending => aerospike_core::operations::lists::ListSortFlags::DESCENDING,
                ListSortFlags::DropDuplicates => aerospike_core::operations::lists::ListSortFlags::DROP_DUPLICATES,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapOrder
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "MapOrder", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MapOrder {
        /// Map is not ordered. This is the default.
        #[pyo3(name = "UNORDERED")]
        Unordered,
        /// Order map by key.
        #[pyo3(name = "KEY_ORDERED")]
        KeyOrdered,
        /// Order map by key, then value.
        #[pyo3(name = "KEY_VALUE_ORDERED")]
        KeyValueOrdered,
    }

    #[pymethods]
    impl MapOrder {
        fn __richcmp__(&self, other: &MapOrder, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Ok(false),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    impl From<&MapOrder> for aerospike_core::operations::maps::MapOrder {
        fn from(input: &MapOrder) -> Self {
            match input {
                MapOrder::Unordered => aerospike_core::operations::maps::MapOrder::Unordered,
                MapOrder::KeyOrdered => aerospike_core::operations::maps::MapOrder::KeyOrdered,
                MapOrder::KeyValueOrdered => aerospike_core::operations::maps::MapOrder::KeyValueOrdered,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapWriteMode
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "MapWriteMode", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MapWriteMode {
        /// If the key already exists, the item will be overwritten.
        /// If the key does not exist, a new item will be created.
        #[pyo3(name = "UPDATE")]
        Update,
        /// If the key already exists, the item will be overwritten.
        /// If the key does not exist, the write will fail.
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly,
        /// If the key already exists, the write will fail.
        /// If the key does not exist, a new item will be created.
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly,
    }

    #[pymethods]
    impl MapWriteMode {
        fn __richcmp__(&self, other: &MapWriteMode, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Ok(false),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    // Core expresses the write mode as the UPDATE_ONLY / CREATE_ONLY write flags.
    impl From<&MapWriteMode> for aerospike_core::operations::maps::MapWriteFlags {
        fn from(input: &MapWriteMode) -> Self {
            match input {
                MapWriteMode::Update => aerospike_core::operations::maps::MapWriteFlags::DEFAULT,
                MapWriteMode::UpdateOnly => aerospike_core::operations::maps::MapWriteFlags::UPDATE_ONLY,
                MapWriteMode::CreateOnly => aerospike_core::operations::maps::MapWriteFlags::CREATE_ONLY,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapWriteFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "MapWriteFlags", module = "aerospike_native._native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MapWriteFlags {
        /// Default. Allow create or update.
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        /// If the key already exists, the item will be denied. If the key does not exist, a new item will be created.
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly = 1,
        /// If the key already exists, the item will be overwritten. If the key does not exist, the item will be denied.
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly = 2,
        /// Do not raise error if a map item is denied due to write flag constraints.
        #[pyo3(name = "NO_FAIL")]
        NoFail = 4,
        /// Allow other valid map items to be committed if a map item is denied due to write flag constraints.
        #[pyo3(name = "PARTIAL")]
        Partial = 8,
    }

    impl From<MapWriteFlags> for u8 {
        fn from(flags: MapWriteFlags) -> Self {
            flags as u8
        }
    }

    #[pymethods]
    impl MapWriteFlags {
        /// Combine flags with bitwise OR. Returns the wire byte as int so combined
        /// values can be passed wherever a single flag is accepted.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Right-hand bitwise OR (e.g. `int | MapWriteFlags.X`).
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(a & b)
        }

        /// Right-hand bitwise AND.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// Right-hand bitwise XOR.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = map_policy_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width).
        fn __invert__(&self) -> u8 {
            !u8::from(*self)
        }

        /// Equality / ordering against another ``MapWriteFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(
            &self,
            other: &Bound<'_, PyAny>,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match map_policy_flags_from_py(other) {
                Ok(v) => v as i64,
                Err(_) => {
                    return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
                }
            };
            Ok(match op {
                pyo3::class::basic::CompareOp::Eq => a == b,
                pyo3::class::basic::CompareOp::Ne => a != b,
                pyo3::class::basic::CompareOp::Lt => a < b,
                pyo3::class::basic::CompareOp::Le => a <= b,
                pyo3::class::basic::CompareOp::Gt => a > b,
                pyo3::class::basic::CompareOp::Ge => a >= b,
            })
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapReturnType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// MapReturnType - supports bitwise OR for combining with INVERTED flag.
    /// 
    /// Example:
    ///     combined = MapReturnType.VALUE | MapReturnType.INVERTED
    #[gen_stub_pyclass]
    #[pyclass(from_py_object, name = "MapReturnType", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MapReturnType(u32);

    type CoreMapRT = aerospike_core::operations::maps::MapReturnType;

    #[gen_stub_pymethods]
    #[pymethods]
    impl MapReturnType {
        /// Do not return a result.
        #[classattr]
        const NONE: MapReturnType = MapReturnType(CoreMapRT::NONE.bits() as u32);
        /// Return key index order.
        #[classattr]
        const INDEX: MapReturnType = MapReturnType(CoreMapRT::INDEX.bits() as u32);
        /// Return reverse key order.
        #[classattr]
        const REVERSE_INDEX: MapReturnType = MapReturnType(CoreMapRT::REVERSE_INDEX.bits() as u32);
        /// Return value order.
        #[classattr]
        const RANK: MapReturnType = MapReturnType(CoreMapRT::RANK.bits() as u32);
        /// Return reverse value order.
        #[classattr]
        const REVERSE_RANK: MapReturnType = MapReturnType(CoreMapRT::REVERSE_RANK.bits() as u32);
        /// Return count of items selected.
        #[classattr]
        const COUNT: MapReturnType = MapReturnType(CoreMapRT::COUNT.bits() as u32);
        /// Return key for single key read and key list for range read.
        #[classattr]
        const KEY: MapReturnType = MapReturnType(CoreMapRT::KEY.bits() as u32);
        /// Return value for single key read and value list for range read.
        #[classattr]
        const VALUE: MapReturnType = MapReturnType(CoreMapRT::VALUE.bits() as u32);
        /// Return key/value items.
        #[classattr]
        const KEY_VALUE: MapReturnType = MapReturnType(CoreMapRT::KEY_VALUE.bits() as u32);
        /// Returns true if count > 0.
        #[classattr]
        const EXISTS: MapReturnType = MapReturnType(CoreMapRT::EXISTS.bits() as u32);
        /// Returns an unordered map.
        #[classattr]
        const UNORDERED_MAP: MapReturnType = MapReturnType(CoreMapRT::UNORDERED_MAP.bits() as u32);
        /// Returns an ordered map.
        #[classattr]
        const ORDERED_MAP: MapReturnType = MapReturnType(CoreMapRT::ORDERED_MAP.bits() as u32);
        /// Invert meaning of map command and return values.
        /// Can be OR'd with other return types: VALUE | INVERTED
        #[classattr]
        const INVERTED: MapReturnType = MapReturnType(CoreMapRT::NONE.inverted().bits() as u32);

        /// Bitwise OR - allows combining return type with INVERTED flag
        fn __or__(&self, other: &Self) -> Self {
            Self(self.0 | other.0)
        }

        /// Bitwise AND
        fn __and__(&self, other: &Self) -> Self {
            Self(self.0 & other.0)
        }

        /// Convert to integer
        fn __int__(&self) -> u32 {
            self.0
        }

        fn __richcmp__(&self, other: &MapReturnType, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                pyo3::class::basic::CompareOp::Lt => Ok(self.0 < other.0),
                pyo3::class::basic::CompareOp::Le => Ok(self.0 <= other.0),
                pyo3::class::basic::CompareOp::Gt => Ok(self.0 > other.0),
                pyo3::class::basic::CompareOp::Ge => Ok(self.0 >= other.0),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }

        fn __repr__(&self) -> String {
            let base = self.0 & 0xFFFF;
            let inverted = (self.0 & 0x10000) != 0;
            let base_name = match base {
                x if x == CoreMapRT::NONE.bits() as u32 => "NONE",
                x if x == CoreMapRT::INDEX.bits() as u32 => "INDEX",
                x if x == CoreMapRT::REVERSE_INDEX.bits() as u32 => "REVERSE_INDEX",
                x if x == CoreMapRT::RANK.bits() as u32 => "RANK",
                x if x == CoreMapRT::REVERSE_RANK.bits() as u32 => "REVERSE_RANK",
                x if x == CoreMapRT::COUNT.bits() as u32 => "COUNT",
                x if x == CoreMapRT::KEY.bits() as u32 => "KEY",
                x if x == CoreMapRT::VALUE.bits() as u32 => "VALUE",
                x if x == CoreMapRT::KEY_VALUE.bits() as u32 => "KEY_VALUE",
                x if x == CoreMapRT::EXISTS.bits() as u32 => "EXISTS",
                x if x == CoreMapRT::UNORDERED_MAP.bits() as u32 => "UNORDERED_MAP",
                x if x == CoreMapRT::ORDERED_MAP.bits() as u32 => "ORDERED_MAP",
                _ => "UNKNOWN",
            };
            if inverted && base != 0 {
                format!("MapReturnType.{} | MapReturnType.INVERTED", base_name)
            } else if inverted {
                "MapReturnType.INVERTED".to_string()
            } else {
                format!("MapReturnType.{}", base_name)
            }
        }
    }

    impl From<MapReturnType> for CoreMapRT {
        fn from(input: MapReturnType) -> Self {
            CoreMapRT::from_bits(input.0 as i64)
        }
    }

    impl From<&MapReturnType> for CoreMapRT {
        fn from(input: &MapReturnType) -> Self {
            CoreMapRT::from_bits(input.0 as i64)
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CTX (Context) for nested CDT operations
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Context for nested CDT (Complex Data Type) operations.
    /// Used to specify the location of nested lists/maps within a record.
    #[gen_stub_pyclass]
    #[pyclass(from_py_object, name = "CTX", module = "aerospike_native._native")]
    #[derive(Clone, Debug, PartialEq)]
    pub struct CTX {
        pub(crate) ctx: aerospike_core::operations::cdt_context::CdtContext,
    }

    impl Eq for CTX {}

    #[gen_stub_pymethods]
    #[pymethods]
    impl CTX {
        /// Compare two CTX objects for equality.
        fn __eq__(&self, other: &CTX) -> bool {
            self.ctx == other.ctx
        }

        /// Lookup list by index offset.
        /// If the index is negative, the resolved index starts backwards from end of list.
        /// Examples: 0 = first item, 4 = fifth item, -1 = last item, -3 = third to last item.
        #[staticmethod]
        pub fn list_index(index: i64) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_list_index(index),
            }
        }

        /// Create list with given type at index offset, given an order and pad.
        #[staticmethod]
        pub fn list_index_create(index: i64, order: ListOrderType, pad: bool) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_list_index_create(
                    index,
                    (&order).into(),
                    pad,
                ),
            }
        }

        /// Lookup list by rank.
        /// 0 = smallest value, N = Nth smallest value, -1 = largest value.
        #[staticmethod]
        pub fn list_rank(rank: i64) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_list_rank(rank),
            }
        }

        /// Lookup list by value.
        #[staticmethod]
        pub fn list_value(value: PythonValue) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_list_value(
                    aerospike_core::Value::from(value),
                ),
            }
        }

        /// Lookup map by index offset.
        /// If the index is negative, the resolved index starts backwards from end of list.
        #[staticmethod]
        pub fn map_index(index: i64) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_index(index),
            }
        }

        /// Lookup map by rank.
        /// 0 = smallest value, N = Nth smallest value, -1 = largest value.
        #[staticmethod]
        pub fn map_rank(rank: i64) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_rank(rank),
            }
        }

        /// Lookup map by key.
        #[staticmethod]
        pub fn map_key(key: PythonValue) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_key(
                    aerospike_core::Value::from(key),
                ),
            }
        }

        /// Create map with given type at map key.
        #[staticmethod]
        pub fn map_key_create(key: PythonValue, order: MapOrder) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_key_create(
                    aerospike_core::Value::from(key),
                    (&order).into(),
                ),
            }
        }

        /// Lookup map by value.
        /// Converts HashMap to BTreeMap (OrderedMap) for exact byte-level matching to ensure consistent serialization.
        #[staticmethod]
        pub fn map_value(value: PythonValue) -> Self {
            let core_value = match value {
                PythonValue::HashMap(h) => {
                    // For map_value context, always use BTreeMap (sorted) for exact byte-level matching
                    // HashMap iteration order is non-deterministic, so we sort to ensure consistent serialization
                    let mut btree_map: BTreeMap<aerospike_core::Value, aerospike_core::Value> = BTreeMap::new();
                    for (k, v) in h {
                        btree_map.insert(aerospike_core::Value::from(k), aerospike_core::Value::from(v));
                    }
                    aerospike_core::Value::SortedMap(btree_map)
                }
                _ => aerospike_core::Value::from(value),
            };
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_value(core_value),
            }
        }

        /// Select all children (elements or entries) of the current collection level.
        ///
        /// Equivalent to calling ``all_children_with_filter`` with a constant
        /// ``true`` expression.  Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn all_children() -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_all_children(),
            }
        }

        /// Select all children of the current collection level that satisfy ``exp``.
        ///
        /// The expression is evaluated per child element.  Use loop-variable
        /// expressions (``FilterExpression.int_loop_var``,
        /// ``FilterExpression.float_loop_var``,
        /// ``FilterExpression.string_loop_var``,
        /// ``FilterExpression.map_loop_var``,
        /// ``FilterExpression.bool_loop_var``) inside ``exp`` to reference the
        /// current element.  Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn all_children_with_filter(exp: crate::expressions::FilterExpression) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_all_children_with_filter(
                    exp._as,
                ),
            }
        }

        /// Select map entries whose keys are in ``keys``.
        ///
        /// Requires Aerospike Server version >= 8.1.2.
        #[staticmethod]
        pub fn map_keys_in(keys: Vec<PythonValue>) -> Self {
            let core_keys: Vec<aerospike_core::Value> =
                keys.into_iter().map(aerospike_core::Value::from).collect();
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_map_keys_in(core_keys),
            }
        }

        /// AND-combine the previous filter context with ``exp``. Used to
        /// stack additional predicates onto an ``all_children_with_filter``
        /// step.
        ///
        /// Requires Aerospike Server version >= 8.1.2.
        #[staticmethod]
        pub fn and_filter(exp: crate::expressions::FilterExpression) -> Self {
            CTX {
                ctx: aerospike_core::operations::cdt_context::ctx_and_filter(exp._as),
            }
        }

        /// Encode a context array to base64 — pairs with :meth:`from_base64`.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn to_base64(ctx: Vec<CTX>) -> PyResult<String> {
            let core_ctx = ctx_to_vec(&ctx);
            aerospike_core::operations::cdt_context::to_base64(&core_ctx)
                .map_err(|e| crate::errors::RustClientError(e).into())
        }

        /// Restore a context array from the base64-encoded form produced by
        /// the matching :meth:`to_base64` helper.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn from_base64(b64: &str) -> PyResult<Vec<CTX>> {
            let core_ctxs = aerospike_core::operations::cdt_context::ctx_from_base64(b64)
                .map_err(crate::errors::RustClientError)?;
            Ok(core_ctxs.into_iter().map(|c| CTX { ctx: c }).collect())
        }

        /// Pack a context array to the raw byte stream that :meth:`to_base64`
        /// encodes -- pairs with :meth:`from_bytes`.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn to_bytes(py: Python<'_>, ctx: Vec<CTX>) -> PyResult<Py<pyo3::types::PyBytes>> {
            let core_ctx = ctx_to_vec(&ctx);
            let bytes = aerospike_core::operations::cdt_context::to_bytes(&core_ctx)
                .map_err(|e| PyErr::from(crate::errors::RustClientError(e)))?;
            Ok(pyo3::types::PyBytes::new(py, &bytes).unbind())
        }

        /// Restore a context array from the raw byte stream that base64
        /// encodes.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn from_bytes(bytes: Vec<u8>) -> PyResult<Vec<CTX>> {
            let core_ctxs = aerospike_core::operations::cdt_context::ctx_from_bytes(&bytes)
                .map_err(crate::errors::RustClientError)?;
            Ok(core_ctxs.into_iter().map(|c| CTX { ctx: c }).collect())
        }
    }

    impl From<&CTX> for aerospike_core::operations::cdt_context::CdtContext {
        fn from(ctx: &CTX) -> Self {
            ctx.ctx.clone()
        }
    }
    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "BitwiseResizeFlags", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BitwiseResizeFlags {
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        #[pyo3(name = "FROM_FRONT")]
        FromFront = 1,
        #[pyo3(name = "GROW_ONLY")]
        GrowOnly = 2,
        #[pyo3(name = "SHRINK_ONLY")]
        ShrinkOnly = 4,
    }

    impl From<BitwiseResizeFlags> for aerospike_core::operations::bitwise::BitwiseResizeFlags {
        fn from(flags: BitwiseResizeFlags) -> Self {
            match flags {
                BitwiseResizeFlags::Default => aerospike_core::operations::bitwise::BitwiseResizeFlags::DEFAULT,
                BitwiseResizeFlags::FromFront => aerospike_core::operations::bitwise::BitwiseResizeFlags::FROM_FRONT,
                BitwiseResizeFlags::GrowOnly => aerospike_core::operations::bitwise::BitwiseResizeFlags::GROW_ONLY,
                BitwiseResizeFlags::ShrinkOnly => aerospike_core::operations::bitwise::BitwiseResizeFlags::SHRINK_ONLY,
            }
        }
    }

    #[pymethods]
    impl BitwiseResizeFlags {
    }

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "BitWriteFlags", module = "aerospike_native._native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum BitWriteFlags {
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly = 1,
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly = 2,
        #[pyo3(name = "NO_FAIL")]
        NoFail = 4,
        #[pyo3(name = "PARTIAL")]
        Partial = 8,
    }

    impl From<BitWriteFlags> for u8 {
        fn from(flags: BitWriteFlags) -> Self {
            flags as u8
        }
    }

    #[pymethods]
    impl BitWriteFlags {
        /// Combine flags with bitwise OR. Returns the wire byte as int so combined
        /// values can be passed wherever a single flag is accepted.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Right-hand bitwise OR (e.g. `int | BitWriteFlags.X`).
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(a & b)
        }

        /// Right-hand bitwise AND.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// Right-hand bitwise XOR.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = bit_policy_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width).
        fn __invert__(&self) -> u8 {
            !u8::from(*self)
        }

        /// Equality / ordering against another ``BitWriteFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(
            &self,
            other: &Bound<'_, PyAny>,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match bit_policy_flags_from_py(other) {
                Ok(v) => v as i64,
                Err(_) => {
                    return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
                }
            };
            Ok(match op {
                pyo3::class::basic::CompareOp::Eq => a == b,
                pyo3::class::basic::CompareOp::Ne => a != b,
                pyo3::class::basic::CompareOp::Lt => a < b,
                pyo3::class::basic::CompareOp::Le => a <= b,
                pyo3::class::basic::CompareOp::Gt => a > b,
                pyo3::class::basic::CompareOp::Ge => a >= b,
            })
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "BitwiseOverflowActions", module = "aerospike_native._native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BitwiseOverflowActions {
        #[pyo3(name = "FAIL")]
        Fail = 0,
        #[pyo3(name = "SATURATE")]
        Saturate = 2,
        #[pyo3(name = "WRAP")]
        Wrap = 4,
    }

    impl From<BitwiseOverflowActions> for aerospike_core::operations::bitwise::BitwiseOverflowActions {
        fn from(action: BitwiseOverflowActions) -> Self {
            match action {
                BitwiseOverflowActions::Fail => aerospike_core::operations::bitwise::BitwiseOverflowActions::Fail,
                BitwiseOverflowActions::Saturate => aerospike_core::operations::bitwise::BitwiseOverflowActions::Saturate,
                BitwiseOverflowActions::Wrap => aerospike_core::operations::bitwise::BitwiseOverflowActions::Wrap,
            }
        }
    }

    #[pymethods]
    impl BitwiseOverflowActions {
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  BitPolicy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass]
    #[pyclass(from_py_object,
        name = "BitPolicy",
        module = "aerospike_native._native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone, Copy)]
    pub struct BitPolicy {
        pub(crate) _as: aerospike_core::operations::bitwise::BitPolicy,
    }

    impl PartialEq for BitPolicy {
        fn eq(&self, other: &Self) -> bool {
            self._as.flags == other._as.flags
        }
    }

    impl Eq for BitPolicy {}

    impl Default for BitPolicy {
        fn default() -> Self {
            BitPolicy {
                _as: aerospike_core::operations::bitwise::BitPolicy::new(aerospike_core::operations::bitwise::BitwiseWriteFlags::DEFAULT),
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl BitPolicy {
        #[new]
        #[pyo3(signature = (write_flags=None))]
        /// Create a new BitPolicy with the specified write flags.
        /// write_flags may be BitWriteFlags or int (bitmask), e.g. CREATE_ONLY | NO_FAIL.
        /// Default is default write flags.
        pub fn new(
            py: Python<'_>,
            write_flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &write_flags {
                None => 0u8,
                Some(obj) => bit_policy_flags_from_py(obj.bind(py))?,
            };
            Ok(BitPolicy {
                _as: aerospike_core::operations::bitwise::BitPolicy::new(aerospike_core::operations::bitwise::BitwiseWriteFlags::from_bits(f)),
            })
        }

        /// Get the write flags.
        pub fn get_write_flags(&self) -> u8 {
            self._as.flags.bits()
        }

        /// Set the write flags.
        /// flags may be BitWriteFlags or int (bitmask).
        pub fn set_write_flags(&mut self, flags: &Bound<'_, PyAny>) -> PyResult<()> {
            self._as.flags = aerospike_core::operations::bitwise::BitwiseWriteFlags::from_bits(bit_policy_flags_from_py(flags)?);
            Ok(())
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListPolicy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass]
    #[pyclass(from_py_object,
        name = "ListPolicy",
        module = "aerospike_native._native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone, Copy)]
    pub struct ListPolicy {
        pub(crate) _as: aerospike_core::operations::lists::ListPolicy,
    }

    impl PartialEq for ListPolicy {
        fn eq(&self, other: &Self) -> bool {
            // Compare the underlying policy fields manually since core client doesn't implement PartialEq
            self._as.attributes as u8 == other._as.attributes as u8 && self._as.flags == other._as.flags
        }
    }

    impl Eq for ListPolicy {}

    impl Default for ListPolicy {
        fn default() -> Self {
            ListPolicy {
                _as: aerospike_core::operations::lists::ListPolicy::new(
                    aerospike_core::operations::lists::ListOrderType::Unordered,
                    aerospike_core::operations::lists::ListWriteFlags::DEFAULT,
                ),
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl ListPolicy {
        #[new]
        #[pyo3(signature = (order=None, write_flags=None))]
        /// Create a new ListPolicy with the specified order and write flags.
        /// Default is unordered list with default write flags.
        /// write_flags may be ListWriteFlags or int (bitmask), e.g. ADD_UNIQUE | NO_FAIL.
        pub fn new(
            py: Python<'_>,
            order: Option<ListOrderType>,
            write_flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let order = order.unwrap_or(ListOrderType::Unordered);
            let f = match &write_flags {
                None => 0u8,
                Some(obj) => list_policy_flags_from_py(obj.bind(py))?,
            };
            Ok(ListPolicy {
                _as: aerospike_core::operations::lists::ListPolicy {
                    attributes: (&order).into(),
                    flags: aerospike_core::operations::lists::ListWriteFlags::from_bits(f),
                },
            })
        }

        #[getter]
        pub fn get_order(&self) -> ListOrderType {
            match self._as.attributes {
                aerospike_core::operations::lists::ListOrderType::Unordered => ListOrderType::Unordered,
                aerospike_core::operations::lists::ListOrderType::Ordered => ListOrderType::Ordered,
            }
        }

        #[setter]
        pub fn set_order(&mut self, order: ListOrderType) {
            self._as.attributes = (&order).into();
        }

        #[getter]
        pub fn get_write_flags(&self) -> u8 {
            self._as.flags.bits()
        }

        #[setter]
        pub fn set_write_flags(&mut self, write_flags: &Bound<'_, PyAny>) -> PyResult<()> {
            self._as.flags = aerospike_core::operations::lists::ListWriteFlags::from_bits(list_policy_flags_from_py(write_flags)?);
            Ok(())
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  HLLWriteFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// HLL write flags for HLL operations.
    #[gen_stub_pyclass_enum]
    #[pyclass(from_py_object, name = "HLLWriteFlags", module = "aerospike_native._native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum HLLWriteFlags {
        /// Default. Allow create or update.
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        /// If the bin already exists, the operation will be denied.
        /// If the bin does not exist, a new bin will be created.
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly = 1,
        /// If the bin already exists, the bin will be overwritten.
        /// If the bin does not exist, the operation will be denied.
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly = 2,
        /// Do not raise error if operation is denied.
        #[pyo3(name = "NO_FAIL")]
        NoFail = 4,
        /// Allow the resulting set to be the minimum of provided index bits.
        #[pyo3(name = "ALLOW_FOLD")]
        AllowFold = 8,
    }

    impl From<HLLWriteFlags> for u8 {
        fn from(flags: HLLWriteFlags) -> Self {
            flags as u8
        }
    }

    #[pymethods]
    impl HLLWriteFlags {
        /// Combine flags with bitwise OR. Returns the wire byte as int so combined
        /// values can be passed wherever a single flag is accepted.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Right-hand bitwise OR (e.g. `int | HLLWriteFlags.X`).
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(a & b)
        }

        /// Right-hand bitwise AND.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// Right-hand bitwise XOR.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
            let a = u8::from(*self);
            let b = hll_write_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width).
        fn __invert__(&self) -> u8 {
            !u8::from(*self)
        }

        /// Equality / ordering against another ``HLLWriteFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(
            &self,
            other: &Bound<'_, PyAny>,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match hll_write_flags_from_py(other) {
                Ok(v) => v as i64,
                Err(_) => {
                    return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
                }
            };
            Ok(match op {
                pyo3::class::basic::CompareOp::Eq => a == b,
                pyo3::class::basic::CompareOp::Ne => a != b,
                pyo3::class::basic::CompareOp::Lt => a < b,
                pyo3::class::basic::CompareOp::Le => a <= b,
                pyo3::class::basic::CompareOp::Gt => a > b,
                pyo3::class::basic::CompareOp::Ge => a >= b,
            })
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  HLLPolicy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// HLL policy for HLL operations and expressions.
    #[gen_stub_pyclass]
    #[pyclass(from_py_object,
        name = "HLLPolicy",
        module = "aerospike_native._native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone, Copy)]
    pub struct HLLPolicy {
        pub(crate) _as: aerospike_core::operations::hll::HllPolicy,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl HLLPolicy {
        #[new]
        #[pyo3(signature = (write_flags=None))]
        /// Create a new HLLPolicy with the specified write flags.
        /// write_flags may be HLLWriteFlags or int (bitmask), e.g. CREATE_ONLY | NO_FAIL.
        /// Default is default write flags.
        pub fn new(
            py: Python<'_>,
            write_flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &write_flags {
                None => 0,
                Some(obj) => hll_write_flags_from_py(obj.bind(py))?,
            };
            Ok(HLLPolicy {
                _as: aerospike_core::operations::hll::HllPolicy { flags: aerospike_core::operations::hll::HllWriteFlags::from_bits(f) },
            })
        }

        /// Get the write flags as an int bitmask.
        #[getter]
        pub fn get_write_flags(&self) -> i64 {
            self._as.flags.bits() as i64
        }

        /// Set the write flags.
        /// flags may be HLLWriteFlags or int (bitmask).
        #[setter]
        pub fn set_write_flags(&mut self, flags: &Bound<'_, PyAny>) -> PyResult<()> {
            self._as.flags = aerospike_core::operations::hll::HllWriteFlags::from_bits(hll_write_flags_from_py(flags)?);
            Ok(())
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapPolicy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass]
    #[pyclass(from_py_object,
        name = "MapPolicy",
        module = "aerospike_native._native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone, Copy)]
    pub struct MapPolicy {
        pub(crate) _as: aerospike_core::operations::maps::MapPolicy,
    }

    impl PartialEq for MapPolicy {
        fn eq(&self, other: &Self) -> bool {
            // Compare the underlying policy fields manually since core client doesn't implement PartialEq
            self._as.order as u8 == other._as.order as u8
                && self._as.flags == other._as.flags
                && self._as.persist_index == other._as.persist_index
        }
    }

    impl Eq for MapPolicy {}

    impl Default for MapPolicy {
        fn default() -> Self {
            MapPolicy {
                _as: aerospike_core::operations::maps::MapPolicy::new(
                    aerospike_core::operations::maps::MapOrder::Unordered,
                    aerospike_core::operations::maps::MapWriteFlags::DEFAULT,
                ),
            }
        }
    }

    fn flags_u8_to_map_write_flags(v: u8) -> MapWriteFlags {
        match v {
            0 => MapWriteFlags::Default,
            1 => MapWriteFlags::CreateOnly,
            2 => MapWriteFlags::UpdateOnly,
            4 => MapWriteFlags::NoFail,
            8 => MapWriteFlags::Partial,
            _ => MapWriteFlags::Default,
        }
    }

    /// Narrows an ``int`` bitmask to the flag byte sent on the wire. A value
    /// outside 0-255 is refused: truncating it would send different flags.
    fn flags_byte(i: i64) -> PyResult<u8> {
        u8::try_from(i)
            .map_err(|_| PyValueError::new_err(format!("flags must be in 0-255, got {i}")))
    }

    /// Extract flags as u8 from ListWriteFlags or int (bitmask). Used for ListPolicy.
    pub(crate) fn list_policy_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
        if let Ok(f) = ob.extract::<ListWriteFlags>() {
            return Ok(u8::from(f));
        }
        if let Ok(i) = ob.extract::<i64>() {
            return flags_byte(i);
        }
        Err(PyValueError::new_err("write_flags must be ListWriteFlags or int"))
    }

    /// Extract flags as u8 from MapWriteFlags or int (bitmask). Used for MapPolicy.
    pub(crate) fn map_policy_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
        if let Ok(m) = ob.extract::<MapWriteFlags>() {
            return Ok(u8::from(m));
        }
        if let Ok(i) = ob.extract::<i64>() {
            return flags_byte(i);
        }
        Err(PyValueError::new_err("flags must be MapWriteFlags or int"))
    }

    /// Extract flags as u8 from BitWriteFlags or int (bitmask). Used for BitPolicy.
    pub(crate) fn bit_policy_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
        if let Ok(f) = ob.extract::<BitWriteFlags>() {
            return Ok(u8::from(f));
        }
        if let Ok(i) = ob.extract::<i64>() {
            return flags_byte(i);
        }
        Err(PyValueError::new_err("write_flags must be BitWriteFlags or int"))
    }

    /// Extract flags as u8 from HLLWriteFlags or int (bitmask). Used for
    /// HLLPolicy, the HLL operations and HLLWriteFlags' own dunders.
    pub(crate) fn hll_write_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
        if let Ok(f) = ob.extract::<HLLWriteFlags>() {
            return Ok(u8::from(f));
        }
        if let Ok(i) = ob.extract::<i64>() {
            return flags_byte(i);
        }
        Err(PyValueError::new_err("write_flags must be HLLWriteFlags or int"))
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl MapPolicy {
        #[new]
        #[pyo3(signature = (order=None, write_mode=None, flags=None, persist_index=None))]
        /// Create a new MapPolicy with the specified order and write mode.
        /// Optionally pass flags and persist_index for server 4.3+ behavior.
        /// Flags may be MapWriteFlags or int (bitmask), e.g. CREATE_ONLY | PARTIAL | NO_FAIL.
        /// Default is unordered map with update write mode.
        pub fn new(
            py: Python<'_>,
            order: Option<MapOrder>,
            write_mode: Option<MapWriteMode>,
            flags: Option<Py<PyAny>>,
            persist_index: Option<bool>,
        ) -> PyResult<Self> {
            let order = order.unwrap_or(MapOrder::Unordered);
            let core_order: aerospike_core::operations::maps::MapOrder = (&order).into();
            let f = match &flags {
                None => 0u8,
                Some(obj) => map_policy_flags_from_py(obj.bind(py))?,
            };
            let _as = if persist_index == Some(true) {
                aerospike_core::operations::maps::MapPolicy::with_persisted_index(
                    core_order, aerospike_core::operations::maps::MapWriteFlags::from_bits(f),
                )
            } else if f != 0 {
                aerospike_core::operations::maps::MapPolicy::new(core_order, aerospike_core::operations::maps::MapWriteFlags::from_bits(f))
            } else {
                let write_mode = write_mode.unwrap_or(MapWriteMode::Update);
                aerospike_core::operations::maps::MapPolicy::new(
                    core_order,
                    (&write_mode).into(),
                )
            };
            Ok(MapPolicy { _as })
        }

        /// Create a new MapPolicy with order and write flags (server 4.3+).
        /// Flags may be MapWriteFlags or int (bitmask), e.g. CREATE_ONLY | PARTIAL | NO_FAIL.
        #[classmethod]
        fn new_with_flags(
            _cls: &Bound<'_, pyo3::types::PyType>,
            _py: Python<'_>,
            order: Option<MapOrder>,
            flags: &Bound<'_, PyAny>,
        ) -> PyResult<Self> {
            let order = order.unwrap_or(MapOrder::Unordered);
            let f = map_policy_flags_from_py(flags)?;
            Ok(MapPolicy {
                _as: aerospike_core::operations::maps::MapPolicy::new(
                    (&order).into(),
                    aerospike_core::operations::maps::MapWriteFlags::from_bits(f),
                ),
            })
        }

        /// Create a new MapPolicy with order, write flags, and persisted index (server 4.3+).
        /// Flags may be MapWriteFlags or int (bitmask), e.g. CREATE_ONLY | PARTIAL | NO_FAIL.
        #[classmethod]
        fn new_with_flags_and_persisted_index(
            _cls: &Bound<'_, pyo3::types::PyType>,
            _py: Python<'_>,
            order: Option<MapOrder>,
            flags: &Bound<'_, PyAny>,
        ) -> PyResult<Self> {
            let order = order.unwrap_or(MapOrder::Unordered);
            let f = map_policy_flags_from_py(flags)?;
            Ok(MapPolicy {
                _as: aerospike_core::operations::maps::MapPolicy::with_persisted_index(
                    (&order).into(),
                    aerospike_core::operations::maps::MapWriteFlags::from_bits(f),
                ),
            })
        }

        #[getter]
        pub fn get_order(&self) -> MapOrder {
            match self._as.order {
                aerospike_core::operations::maps::MapOrder::Unordered => MapOrder::Unordered,
                aerospike_core::operations::maps::MapOrder::KeyOrdered => MapOrder::KeyOrdered,
                aerospike_core::operations::maps::MapOrder::KeyValueOrdered => MapOrder::KeyValueOrdered,
            }
        }

        #[setter]
        pub fn set_order(&mut self, order: MapOrder) {
            self._as.order = (&order).into();
        }

        #[getter]
        pub fn get_write_mode(&self) -> MapWriteMode {
            if self._as.flags.contains(aerospike_core::operations::maps::MapWriteFlags::CREATE_ONLY) {
                MapWriteMode::CreateOnly
            } else if self._as.flags.contains(aerospike_core::operations::maps::MapWriteFlags::UPDATE_ONLY) {
                MapWriteMode::UpdateOnly
            } else {
                MapWriteMode::Update
            }
        }

        #[setter]
        pub fn set_write_mode(&mut self, write_mode: MapWriteMode) {
            let mode_bits = (aerospike_core::operations::maps::MapWriteFlags::CREATE_ONLY | aerospike_core::operations::maps::MapWriteFlags::UPDATE_ONLY).bits();
            let mode: aerospike_core::operations::maps::MapWriteFlags = (&write_mode).into();
            self._as.flags = aerospike_core::operations::maps::MapWriteFlags::from_bits((self._as.flags.bits() & !mode_bits) | mode.bits());
        }

        /// Get the write flags as a MapWriteFlags variant.
        ///
        /// Note: this getter is lossy for combined bitmasks — if the underlying
        /// raw byte is a composite (e.g. ``CREATE_ONLY | NO_FAIL == 5``), the
        /// returned variant collapses to ``MapWriteFlags.DEFAULT``. Use
        /// :py:attr:`raw_flags` for the lossless ``int`` value.
        #[getter]
        pub fn get_flags(&self) -> MapWriteFlags {
            flags_u8_to_map_write_flags(self._as.flags.bits())
        }

        /// Get the raw write-flags byte as ``int`` — lossless replacement for
        /// :py:attr:`flags` when combined bitmasks are in use.
        #[getter(raw_flags)]
        pub fn get_raw_flags(&self) -> u8 {
            self._as.flags.bits()
        }

        #[setter]
        pub fn set_flags(&mut self, flags: &Bound<'_, PyAny>) -> PyResult<()> {
            self._as.flags = aerospike_core::operations::maps::MapWriteFlags::from_bits(map_policy_flags_from_py(flags)?);
            Ok(())
        }

        #[getter]
        pub fn get_persist_index(&self) -> bool {
            self._as.persist_index
        }

        #[setter]
        pub fn set_persist_index(&mut self, persist_index: bool) {
            self._as.persist_index = persist_index;
        }
    }


pub(crate) fn ctx_to_vec(ctx: &[CTX]) -> Vec<aerospike_core::operations::cdt_context::CdtContext> {
    ctx.iter().map(|c| c.ctx.clone()).collect()
}
