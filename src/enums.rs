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

use pyo3::basic::CompareOp;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};
use aerospike_core::{ClientResultCode, ResultCode as CoreResultCode};

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  QueryDuration
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Expected query duration. The server treats the query in different ways depending on the expected duration.
    /// This enum is ignored for aggregation queries, background queries and server versions < 6.0.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum QueryDuration {
        /// Long specifies that the query is expected to return more than 100 records per node.
        #[pyo3(name = "LONG")]
        Long = 0,
        /// Short specifies that the query is expected to return less than 100 records per node.
        #[pyo3(name = "SHORT")]
        Short = 1,
        /// LongRelaxAP will treat query as a Long query, but relax read consistency for AP namespaces.
        #[pyo3(name = "LONG_RELAX_AP")]
        LongRelaxAP = 2,
    }

    impl From<&QueryDuration> for aerospike_core::policy::QueryDuration {
        fn from(input: &QueryDuration) -> Self {
            match input {
                QueryDuration::Long => aerospike_core::policy::QueryDuration::Long,
                QueryDuration::Short => aerospike_core::policy::QueryDuration::Short,
                QueryDuration::LongRelaxAP => aerospike_core::policy::QueryDuration::LongRelaxAp,
            }
        }
    }

    impl From<aerospike_core::policy::QueryDuration> for QueryDuration {
        fn from(input: aerospike_core::policy::QueryDuration) -> Self {
            match input {
                aerospike_core::policy::QueryDuration::Long => QueryDuration::Long,
                aerospike_core::policy::QueryDuration::Short => QueryDuration::Short,
                aerospike_core::policy::QueryDuration::LongRelaxAp => QueryDuration::LongRelaxAP,
            }
        }
    }

    #[pymethods]
    impl QueryDuration {
        fn __richcmp__(&self, other: &QueryDuration, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Replica
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

/// Priority of operations on database server.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Replica {
    #[pyo3(name = "MASTER")]
    Master,
    #[pyo3(name = "MASTER_PROLES")]
    MasterProles,
    #[pyo3(name = "RANDOM")]
    Random,
    #[pyo3(name = "SEQUENCE")]
    Sequence,
    #[pyo3(name = "PREFER_RACK")]
    PreferRack,
}

    impl From<&Replica> for aerospike_core::policy::Replica {
        fn from(input: &Replica) -> Self {
            match &input {
                Replica::Master => aerospike_core::policy::Replica::Master,
                Replica::MasterProles => aerospike_core::policy::Replica::MasterProles,
                Replica::Random => aerospike_core::policy::Replica::Random,
                Replica::Sequence => aerospike_core::policy::Replica::Sequence,
                Replica::PreferRack => aerospike_core::policy::Replica::PreferRack,
            }
        }
    }

    impl From<&aerospike_core::policy::Replica> for Replica {
        fn from(input: &aerospike_core::policy::Replica) -> Self {
            match input {
                aerospike_core::policy::Replica::Master => Replica::Master,
                aerospike_core::policy::Replica::MasterProles => Replica::MasterProles,
                aerospike_core::policy::Replica::Random => Replica::Random,
                aerospike_core::policy::Replica::Sequence => Replica::Sequence,
                aerospike_core::policy::Replica::PreferRack => Replica::PreferRack,
            }
        }
    }

    #[pymethods]
    impl Replica {
        fn __richcmp__(&self, other: &Replica, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Concurrency
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

/// Whether a command that spans multiple cluster nodes runs the per-node
/// requests one at a time or all at once. Applies to batch commands.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Concurrency {
    /// Issue the per-node requests sequentially, one node at a time.
    #[pyo3(name = "SEQUENTIAL")]
    Sequential,
    /// Issue the per-node requests in parallel across nodes.
    #[pyo3(name = "PARALLEL")]
    Parallel,
}

    impl From<&Concurrency> for aerospike_core::policy::Concurrency {
        fn from(input: &Concurrency) -> Self {
            match &input {
                Concurrency::Sequential => aerospike_core::policy::Concurrency::Sequential,
                Concurrency::Parallel => aerospike_core::policy::Concurrency::Parallel,
            }
        }
    }

    impl From<&aerospike_core::policy::Concurrency> for Concurrency {
        fn from(input: &aerospike_core::policy::Concurrency) -> Self {
            match input {
                aerospike_core::policy::Concurrency::Sequential => Concurrency::Sequential,
                aerospike_core::policy::Concurrency::Parallel => Concurrency::Parallel,
            }
        }
    }

    #[pymethods]
    impl Concurrency {
        fn __richcmp__(&self, other: &Concurrency, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ReadModeAP
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Read policy for AP (availability) namespaces.
    /// Indicates how duplicates should be consulted in a read operation.
    /// Only makes a difference during migrations and only applicable in AP mode.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ReadModeAP {
        /// A single node should be involved in the read operation.
        #[pyo3(name = "ONE")]
        One,
        /// All duplicates should be consulted in the read operation.
        #[pyo3(name = "ALL")]
        All,
    }

    #[pymethods]
    impl ReadModeAP {
        fn __richcmp__(&self, other: &ReadModeAP, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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

    impl From<&ReadModeAP> for aerospike_core::policy::ReadModeAp {
        fn from(input: &ReadModeAP) -> Self {
            match input {
                ReadModeAP::One => aerospike_core::policy::ReadModeAp::One,
                ReadModeAP::All => aerospike_core::policy::ReadModeAp::All,
            }
        }
    }

    impl From<&aerospike_core::policy::ReadModeAp> for ReadModeAP {
        fn from(input: &aerospike_core::policy::ReadModeAp) -> Self {
            match input {
                aerospike_core::policy::ReadModeAp::One => ReadModeAP::One,
                aerospike_core::policy::ReadModeAp::All => ReadModeAP::All,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ReadModeSC
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Read policy for SC (strong consistency) namespaces.
    /// Determines SC read consistency options.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ReadModeSC {
        /// Ensures this client will only see an increasing sequence of record versions.
        /// Client only reads from master. This is the default.
        #[pyo3(name = "SESSION")]
        Session,
        /// Ensures all clients will only see an increasing sequence of record versions.
        /// Client only reads from master.
        #[pyo3(name = "LINEARIZE")]
        Linearize,
        /// The client may read from master or any full (non-migrating) replica.
        /// Increasing sequence of record versions is not guaranteed.
        #[pyo3(name = "ALLOW_REPLICA")]
        AllowReplica,
        /// The client may read from master or any full (non-migrating) replica or from
        /// unavailable partitions. Increasing sequence of record versions is not guaranteed.
        #[pyo3(name = "ALLOW_UNAVAILABLE")]
        AllowUnavailable,
    }

    #[pymethods]
    impl ReadModeSC {
        fn __richcmp__(&self, other: &ReadModeSC, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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

    impl From<&ReadModeSC> for aerospike_core::policy::ReadModeSc {
        fn from(input: &ReadModeSC) -> Self {
            match input {
                ReadModeSC::Session => aerospike_core::policy::ReadModeSc::Session,
                ReadModeSC::Linearize => aerospike_core::policy::ReadModeSc::Linearize,
                ReadModeSC::AllowReplica => aerospike_core::policy::ReadModeSc::AllowReplica,
                ReadModeSC::AllowUnavailable => aerospike_core::policy::ReadModeSc::AllowUnavailable,
            }
        }
    }

    impl From<&aerospike_core::policy::ReadModeSc> for ReadModeSC {
        fn from(input: &aerospike_core::policy::ReadModeSc) -> Self {
            match input {
                aerospike_core::policy::ReadModeSc::Session => ReadModeSC::Session,
                aerospike_core::policy::ReadModeSc::Linearize => ReadModeSC::Linearize,
                aerospike_core::policy::ReadModeSc::AllowReplica => ReadModeSC::AllowReplica,
                aerospike_core::policy::ReadModeSc::AllowUnavailable => ReadModeSC::AllowUnavailable,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  RecordExistsAction
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// `RecordExistsAction` determines how to handle record writes based on record generation.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    pub enum RecordExistsAction {
        #[pyo3(name = "UPDATE")]
        Update,
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly,
        #[pyo3(name = "REPLACE")]
        Replace,
        #[pyo3(name = "REPLACE_ONLY")]
        ReplaceOnly,
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly,
    }

    #[pymethods]
    impl RecordExistsAction {
        fn __richcmp__(&self, other: &RecordExistsAction, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<&RecordExistsAction> for aerospike_core::policy::RecordExistsAction {
        fn from(input: &RecordExistsAction) -> Self {
            match &input {
                RecordExistsAction::Update => aerospike_core::policy::RecordExistsAction::Update,
                RecordExistsAction::UpdateOnly => {
                    aerospike_core::policy::RecordExistsAction::UpdateOnly
                }
                RecordExistsAction::Replace => aerospike_core::policy::RecordExistsAction::Replace,
                RecordExistsAction::ReplaceOnly => {
                    aerospike_core::policy::RecordExistsAction::ReplaceOnly
                }
                RecordExistsAction::CreateOnly => {
                    aerospike_core::policy::RecordExistsAction::CreateOnly
                }
            }
        }
    }

    impl From<&aerospike_core::policy::RecordExistsAction> for RecordExistsAction {
        fn from(input: &aerospike_core::policy::RecordExistsAction) -> Self {
            match input {
                aerospike_core::policy::RecordExistsAction::Update => RecordExistsAction::Update,
                aerospike_core::policy::RecordExistsAction::UpdateOnly => {
                    RecordExistsAction::UpdateOnly
                }
                aerospike_core::policy::RecordExistsAction::Replace => RecordExistsAction::Replace,
                aerospike_core::policy::RecordExistsAction::ReplaceOnly => {
                    RecordExistsAction::ReplaceOnly
                }
                aerospike_core::policy::RecordExistsAction::CreateOnly => {
                    RecordExistsAction::CreateOnly
                }
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  GenerationPolicy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    pub enum GenerationPolicy {
        #[pyo3(name = "NONE")]
        None,
        #[pyo3(name = "EXPECT_GEN_EQUAL")]
        ExpectGenEqual,
        #[pyo3(name = "EXPECT_GEN_GREATER")]
        ExpectGenGreater,
    }

    #[pymethods]
    impl GenerationPolicy {
        fn __richcmp__(&self, other: &GenerationPolicy, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<&GenerationPolicy> for aerospike_core::policy::GenerationPolicy {
        fn from(input: &GenerationPolicy) -> Self {
            match &input {
                GenerationPolicy::None => aerospike_core::policy::GenerationPolicy::None,
                GenerationPolicy::ExpectGenEqual => {
                    aerospike_core::policy::GenerationPolicy::ExpectGenEqual
                }
                GenerationPolicy::ExpectGenGreater => {
                    aerospike_core::policy::GenerationPolicy::ExpectGenGreater
                }
            }
        }
    }

    impl From<&aerospike_core::policy::GenerationPolicy> for GenerationPolicy {
        fn from(input: &aerospike_core::policy::GenerationPolicy) -> Self {
            match input {
                aerospike_core::policy::GenerationPolicy::None => GenerationPolicy::None,
                aerospike_core::policy::GenerationPolicy::ExpectGenEqual => {
                    GenerationPolicy::ExpectGenEqual
                }
                aerospike_core::policy::GenerationPolicy::ExpectGenGreater => {
                    GenerationPolicy::ExpectGenGreater
                }
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CommitLevel
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CommitLevel {
        #[pyo3(name = "COMMIT_ALL")]
        CommitAll,
        #[pyo3(name = "COMMIT_MASTER")]
        CommitMaster,
    }

    #[pymethods]
    impl CommitLevel {
        fn __richcmp__(&self, other: &CommitLevel, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<&CommitLevel> for aerospike_core::policy::CommitLevel {
        fn from(input: &CommitLevel) -> Self {
            match &input {
                CommitLevel::CommitAll => aerospike_core::policy::CommitLevel::CommitAll,
                CommitLevel::CommitMaster => aerospike_core::policy::CommitLevel::CommitMaster,
            }
        }
    }

    impl From<&aerospike_core::policy::CommitLevel> for CommitLevel {
        fn from(input: &aerospike_core::policy::CommitLevel) -> Self {
            match input {
                aerospike_core::policy::CommitLevel::CommitAll => CommitLevel::CommitAll,
                aerospike_core::policy::CommitLevel::CommitMaster => CommitLevel::CommitMaster,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Expiration
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "Expiration",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Expiration {
        v: _Expiration,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Expiration {
        #[classattr]
        const NAMESPACE_DEFAULT: Expiration = Expiration {
            v: _Expiration::NamespaceDefault,
        };

        #[classattr]
        const NEVER_EXPIRE: Expiration = Expiration {
            v: _Expiration::Never,
        };

        #[classattr]
        const DONT_UPDATE: Expiration = Expiration {
            v: _Expiration::DontUpdate,
        };

        #[staticmethod]
        pub fn seconds(s: u32) -> Expiration {
            Expiration {
                v: _Expiration::Seconds(s),
            }
        }

        fn __richcmp__(&self, other: &Expiration, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<&Expiration> for aerospike_core::Expiration {
        fn from(input: &Expiration) -> Self {
            match input.v {
                _Expiration::Seconds(s) => aerospike_core::Expiration::Seconds(s),
                _Expiration::NamespaceDefault => aerospike_core::Expiration::NamespaceDefault,
                _Expiration::Never => aerospike_core::Expiration::Never,
                _Expiration::DontUpdate => aerospike_core::Expiration::DontUpdate,
            }
        }
    }

    impl From<&aerospike_core::Expiration> for Expiration {
        fn from(input: &aerospike_core::Expiration) -> Self {
            match input {
                aerospike_core::Expiration::Seconds(s) => Expiration {
                    v: _Expiration::Seconds(*s),
                },
                aerospike_core::Expiration::NamespaceDefault => Expiration {
                    v: _Expiration::NamespaceDefault,
                },
                aerospike_core::Expiration::Never => Expiration {
                    v: _Expiration::Never,
                },
                aerospike_core::Expiration::DontUpdate => Expiration {
                    v: _Expiration::DontUpdate,
                },
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum _Expiration {
        Seconds(u32),
        NamespaceDefault,
        Never,
        DontUpdate,
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  IndexType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Underlying data type of secondary index.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub enum IndexType {
        #[pyo3(name = "NUMERIC")]
        Numeric,
        #[pyo3(name = "STRING")]
        String,
        #[pyo3(name = "GEO2D_SPHERE")]
        Geo2DSphere,
        /// Integer index, the server's own name for it from 8.1.3 on; older
        /// servers reject it, where ``NUMERIC`` still creates the same index.
        #[pyo3(name = "INTEGER")]
        Integer,
        /// Blob (byte-array) index. Requires server 7.0+.
        #[pyo3(name = "BLOB")]
        Blob,
    }


    impl From<&IndexType> for aerospike_core::query::IndexType {
        fn from(input: &IndexType) -> Self {
            match &input {
                IndexType::Numeric => aerospike_core::query::IndexType::Numeric,
                IndexType::String => aerospike_core::query::IndexType::String,
                IndexType::Geo2DSphere => aerospike_core::query::IndexType::Geo2DSphere,
                IndexType::Integer => aerospike_core::query::IndexType::Integer,
                IndexType::Blob => aerospike_core::query::IndexType::Blob,
            }
        }
    }

    #[pymethods]
    impl IndexType {
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CollectionIndexType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Secondary index collection type.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub enum CollectionIndexType {
        #[pyo3(name = "DEFAULT")]
        Default,
        #[pyo3(name = "LIST")]
        List,
        #[pyo3(name = "MAP_KEYS")]
        MapKeys,
        #[pyo3(name = "MAP_VALUES")]
        MapValues,
        /// A set index: record presence per set, with no bin or type.
        #[pyo3(name = "SET")]
        Set,
    }


    impl From<&CollectionIndexType> for aerospike_core::query::CollectionIndexType {
        fn from(input: &CollectionIndexType) -> Self {
            match &input {
                CollectionIndexType::Default => aerospike_core::query::CollectionIndexType::Default,
                CollectionIndexType::List => aerospike_core::query::CollectionIndexType::List,
                CollectionIndexType::MapKeys => aerospike_core::query::CollectionIndexType::MapKeys,
                CollectionIndexType::MapValues => {
                    aerospike_core::query::CollectionIndexType::MapValues
                }
                CollectionIndexType::Set => aerospike_core::query::CollectionIndexType::Set,
            }
        }
    }

    #[pymethods]
    impl CollectionIndexType {
    }
    /// User-defined function (UDF) language.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub enum UDFLang {
        /// Lua embedded programming language.
        #[pyo3(name = "LUA")]
        Lua,
    }

    impl From<&UDFLang> for aerospike_core::UdfLang {
        fn from(lang: &UDFLang) -> Self {
            match lang {
                UDFLang::Lua => aerospike_core::UdfLang::Lua,
            }
        }
    }

    impl From<UDFLang> for aerospike_core::UdfLang {
        fn from(lang: UDFLang) -> Self {
            match lang {
                UDFLang::Lua => aerospike_core::UdfLang::Lua,
            }
        }
    }
    /// Authentication mode for client connections.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AuthMode {
        /// No authentication will be performed.
        #[pyo3(name = "NONE")]
        None,

        /// Uses internal authentication when user/password defined. Hashed password is stored
        /// on the server. Do not send clear password. This is the default.
        #[pyo3(name = "INTERNAL")]
        Internal,

        /// Uses external authentication (like LDAP) when user/password defined. Specific external
        /// authentication is configured on server. If TLSConfig is defined, sends clear password
        /// on node login via TLS. Will return an error if TLSConfig is not defined.
        #[pyo3(name = "EXTERNAL")]
        External,

        /// Uses external authentication (like LDAP) when user/password defined. Specific external
        /// authentication is configured on server. Sends the clear password on node login whether
        /// or not TLS is defined. This mode should only be used for testing purposes because it is
        /// not secure authentication.
        #[pyo3(name = "EXTERNAL_INSECURE")]
        ExternalInsecure,

        /// Allows authentication and authorization based on a certificate. No user name or
        /// password needs to be configured. Requires TLS and a client certificate.
        /// Requires server version 5.7.0+
        #[pyo3(name = "PKI")]
        PKI,
    }

    #[pymethods]
    impl AuthMode {
        fn __richcmp__(&self, other: &AuthMode, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
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
    // Core keeps server and client codes as two enums; Python sees the one
    // flat namespace every Aerospike client shares. Holding the two core
    // enums rather than a bare i32 keeps the server half convertible to
    // core's type without a range check at each use.
    #[derive(Debug, Clone, Copy)]
    pub(crate) enum AnyCode {
        Server(CoreResultCode),
        Client(ClientResultCode),
    }

    /// One code from the result-code namespace every Aerospike client
    /// shares. Non-negative values are the codes the server returns;
    /// negative values are the codes the client assigns to a failure that
    /// never got a server answer (``TXN_FAILED``, ``SERVER_NOT_AVAILABLE``,
    /// ``MAX_RETRIES_EXCEEDED``, …).
    ///
    /// Behaves like an ``IntEnum`` member: ``repr`` shows the name and the
    /// wire value (``<ResultCode.KEY_EXISTS_ERROR: 5>``), ``str`` and ``int``
    /// give the value alone (``"5"``, ``5``), ``name`` / ``value`` expose the
    /// two halves, and ``description`` is the code's descriptive string.
    /// Compares equal to another member with the same code and to that
    /// code as an ``int`` (``ResultCode.KEY_EXISTS_ERROR == 5``), and hashes
    /// like that ``int``.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, name = "ResultCode", module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub struct ResultCode(pub(crate) AnyCode);

    impl From<CoreResultCode> for ResultCode {
        fn from(rc: CoreResultCode) -> Self {
            ResultCode(AnyCode::Server(rc))
        }
    }

    impl From<ClientResultCode> for ResultCode {
        fn from(rc: ClientResultCode) -> Self {
            ResultCode(AnyCode::Client(rc))
        }
    }

    impl ResultCode {
        /// The member for a numeric code as core reports it: negative is a
        /// client code, the byte range is a server code.
        pub(crate) fn from_i32(code: i32) -> ResultCode {
            match u8::try_from(code) {
                Ok(byte) => ResultCode::from(CoreResultCode::from(byte)),
                Err(_) => ResultCode::from(ClientResultCode::from(code)),
            }
        }

        /// The numeric code in the shared namespace.
        pub(crate) fn code(&self) -> i32 {
            match self.0 {
                AnyCode::Server(rc) => i32::from(u8::from(rc)),
                AnyCode::Client(rc) => i32::from(rc),
            }
        }

        /// The server half, for the paths that only make sense for a code
        /// the server returned (server errors, per-code metrics).
        pub(crate) fn server(&self) -> Option<CoreResultCode> {
            match self.0 {
                AnyCode::Server(rc) => Some(rc),
                AnyCode::Client(_) => None,
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    #[allow(non_snake_case)]  // Class attributes use PascalCase to match Rust enum variants
    impl ResultCode {
        // Compares by wire value, against another member or a plain int, the
        // way an ``IntEnum`` member does. Comparing discriminants would make
        // every ``Unknown(n)`` equal to every other ``Unknown(m)``.
        fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<Py<PyAny>> {
            let py = other.py();
            let mine = i64::from(self.code());
            let theirs: Option<i64> = if let Ok(rc) = other.extract::<ResultCode>() {
                Some(i64::from(rc.code()))
            } else {
                other.extract::<i64>().ok()
            };
            let Some(theirs) = theirs else {
                return Ok(py.NotImplemented());
            };
            let eq = mine == theirs;
            match op {
                CompareOp::Eq => Ok(eq.into_pyobject(py)?.to_owned().into_any().unbind()),
                CompareOp::Ne => Ok((!eq).into_pyobject(py)?.to_owned().into_any().unbind()),
                _ => Ok(py.NotImplemented()),
            }
        }

        // Same hash as the int of equal value, so ``{5: ...}[rc]`` and
        // ``{-17: ...}[rc]`` resolve. PyO3 folds a -1 result to -2 the way
        // CPython does for ``hash(-1)``.
        fn __hash__(&self) -> isize {
            self.code() as isize
        }

        /// The constant's name, as declared on this class (``"KEY_EXISTS_ERROR"``);
        /// ``"UNKNOWN"`` for a wire value this build has no constant for.
        #[getter]
        fn name(&self) -> &'static str {
            let rc = match self.0 {
                AnyCode::Server(rc) => rc,
                // ASYNC_QUEUE_FULL is reserved in core and never produced, so
                // like the other clients this class declares no constant for it.
                AnyCode::Client(rc) => {
                    return match rc {
                        ClientResultCode::TxnAlreadyAborted => "TXN_ALREADY_ABORTED",
                        ClientResultCode::TxnAlreadyCommitted => "TXN_ALREADY_COMMITTED",
                        ClientResultCode::TxnFailed => "TXN_FAILED",
                        ClientResultCode::BatchFailed => "BATCH_FAILED",
                        ClientResultCode::NoResponse => "NO_RESPONSE",
                        ClientResultCode::MaxErrorRate => "MAX_ERROR_RATE",
                        ClientResultCode::MaxRetriesExceeded => "MAX_RETRIES_EXCEEDED",
                        ClientResultCode::SerializeError => "SERIALIZE_ERROR",
                        ClientResultCode::ServerNotAvailable => "SERVER_NOT_AVAILABLE",
                        ClientResultCode::NoMoreConnections => "NO_MORE_CONNECTIONS",
                        ClientResultCode::QueryTerminated => "QUERY_TERMINATED",
                        ClientResultCode::ScanTerminated => "SCAN_TERMINATED",
                        ClientResultCode::InvalidNodeError => "INVALID_NODE_ERROR",
                        ClientResultCode::ParseError => "PARSE_ERROR",
                        ClientResultCode::ClientError => "CLIENT_ERROR",
                        ClientResultCode::AsyncQueueFull | ClientResultCode::Unknown(_) => "UNKNOWN",
                        _ => "UNKNOWN",
                    };
                }
            };
            match rc {
                CoreResultCode::Ok => "OK",
                CoreResultCode::ServerError => "SERVER_ERROR",
                CoreResultCode::KeyNotFoundError => "KEY_NOT_FOUND_ERROR",
                CoreResultCode::GenerationError => "GENERATION_ERROR",
                CoreResultCode::ParameterError => "PARAMETER_ERROR",
                CoreResultCode::KeyExistsError => "KEY_EXISTS_ERROR",
                CoreResultCode::BinExistsError => "BIN_EXISTS_ERROR",
                CoreResultCode::ClusterKeyMismatch => "CLUSTER_KEY_MISMATCH",
                CoreResultCode::ServerMemError => "SERVER_MEM_ERROR",
                CoreResultCode::Timeout => "TIMEOUT",
                CoreResultCode::AlwaysForbidden => "ALWAYS_FORBIDDEN",
                CoreResultCode::PartitionUnavailable => "PARTITION_UNAVAILABLE",
                CoreResultCode::BinTypeError => "BIN_TYPE_ERROR",
                CoreResultCode::RecordTooBig => "RECORD_TOO_BIG",
                CoreResultCode::KeyBusy => "KEY_BUSY",
                CoreResultCode::ScanAbort => "SCAN_ABORT",
                CoreResultCode::UnsupportedFeature => "UNSUPPORTED_FEATURE",
                CoreResultCode::BinNotFound => "BIN_NOT_FOUND",
                CoreResultCode::DeviceOverload => "DEVICE_OVERLOAD",
                CoreResultCode::KeyMismatch => "KEY_MISMATCH",
                CoreResultCode::InvalidNamespace => "INVALID_NAMESPACE",
                CoreResultCode::BinNameTooLong => "BIN_NAME_TOO_LONG",
                CoreResultCode::FailForbidden => "FAIL_FORBIDDEN",
                CoreResultCode::ElementNotFound => "ELEMENT_NOT_FOUND",
                CoreResultCode::ElementExists => "ELEMENT_EXISTS",
                CoreResultCode::EnterpriseOnly => "ENTERPRISE_ONLY",
                CoreResultCode::OpNotApplicable => "OP_NOT_APPLICABLE",
                CoreResultCode::FilteredOut => "FILTERED_OUT",
                CoreResultCode::LostConflict => "LOST_CONFLICT",
                CoreResultCode::InvalidEncoding => "INVALID_ENCODING",
                CoreResultCode::XdrKeyBusy => "XDR_KEY_BUSY",
                CoreResultCode::QueryEnd => "QUERY_END",
                CoreResultCode::SecurityNotSupported => "SECURITY_NOT_SUPPORTED",
                CoreResultCode::SecurityNotEnabled => "SECURITY_NOT_ENABLED",
                CoreResultCode::NotAuthenticated => "NOT_AUTHENTICATED",
                CoreResultCode::SecuritySchemeNotSupported => "SECURITY_SCHEME_NOT_SUPPORTED",
                CoreResultCode::InvalidCommand => "INVALID_COMMAND",
                CoreResultCode::InvalidField => "INVALID_FIELD",
                CoreResultCode::IllegalState => "ILLEGAL_STATE",
                CoreResultCode::InvalidUser => "INVALID_USER",
                CoreResultCode::UserAlreadyExists => "USER_ALREADY_EXISTS",
                CoreResultCode::ForbiddenPassword => "FORBIDDEN_PASSWORD",
                CoreResultCode::UdfBadResponse => "UDF_BAD_RESPONSE",
                CoreResultCode::IndexFound => "INDEX_FOUND",
                CoreResultCode::IndexNotFound => "INDEX_NOT_FOUND",
                CoreResultCode::IndexOom => "INDEX_OOM",
                CoreResultCode::IndexNotReadable => "INDEX_NOT_READABLE",
                CoreResultCode::IndexGeneric => "INDEX_GENERIC",
                CoreResultCode::IndexNameMaxLen => "INDEX_NAME_MAX_LEN",
                CoreResultCode::IndexMaxCount => "INDEX_MAX_COUNT",
                CoreResultCode::QueryAborted => "QUERY_ABORTED",
                CoreResultCode::QueryQueueFull => "QUERY_QUEUE_FULL",
                CoreResultCode::QueryTimeout => "QUERY_TIMEOUT",
                CoreResultCode::QueryGeneric => "QUERY_GENERIC",
                CoreResultCode::MrtBlocked => "MRT_BLOCKED",
                CoreResultCode::MrtVersionMismatch => "MRT_VERSION_MISMATCH",
                CoreResultCode::MrtExpired => "MRT_EXPIRED",
                CoreResultCode::MrtTooManyWrites => "MRT_TOO_MANY_WRITES",
                CoreResultCode::MrtCommitted => "MRT_COMMITTED",
                CoreResultCode::MrtAborted => "MRT_ABORTED",
                CoreResultCode::MrtAlreadyLocked => "MRT_ALREADY_LOCKED",
                CoreResultCode::MrtMonitorExists => "MRT_MONITOR_EXISTS",
                CoreResultCode::InvalidPassword => "INVALID_PASSWORD",
                CoreResultCode::ExpiredPassword => "EXPIRED_PASSWORD",
                CoreResultCode::InvalidCredential => "INVALID_CREDENTIAL",
                CoreResultCode::ExpiredSession => "EXPIRED_SESSION",
                CoreResultCode::InvalidRole => "INVALID_ROLE",
                CoreResultCode::RoleAlreadyExists => "ROLE_ALREADY_EXISTS",
                CoreResultCode::InvalidPrivilege => "INVALID_PRIVILEGE",
                CoreResultCode::InvalidAllowlist => "INVALID_ALLOWLIST",
                CoreResultCode::RoleViolation => "ROLE_VIOLATION",
                CoreResultCode::NotAllowlisted => "NOT_ALLOWLISTED",
                CoreResultCode::QuotasNotEnabled => "QUOTAS_NOT_ENABLED",
                CoreResultCode::InvalidQuota => "INVALID_QUOTA",
                CoreResultCode::QuotaExceeded => "QUOTA_EXCEEDED",
                CoreResultCode::BatchDisabled => "BATCH_DISABLED",
                CoreResultCode::BatchMaxRequestsExceeded => "BATCH_MAX_REQUESTS_EXCEEDED",
                CoreResultCode::BatchQueuesFull => "BATCH_QUEUES_FULL",
                CoreResultCode::InvalidGeojson => "INVALID_GEOJSON",
                CoreResultCode::QueryNetioErr => "QUERY_NETIO_ERR",
                CoreResultCode::QueryDuplicate => "QUERY_DUPLICATE",
                CoreResultCode::Unknown(_) => "UNKNOWN",
                _ => "UNKNOWN",
            }
        }

        /// The numeric value (``5`` for ``KEY_EXISTS_ERROR``, ``-17`` for
        /// ``TXN_FAILED``), the number server logs and the result-code
        /// reference show.
        #[getter]
        fn value(&self) -> i32 {
            self.code()
        }

        /// The code's descriptive string (``"Key already exists"``), the same
        /// text a failure carries as its base message.
        #[getter]
        fn description(&self) -> String {
            match self.0 {
                AnyCode::Server(rc) => rc.into_string(),
                AnyCode::Client(rc) => rc.into_string(),
            }
        }

        fn __int__(&self) -> i32 {
            self.code()
        }

        fn __index__(&self) -> i32 {
            self.code()
        }

        fn __str__(&self) -> String {
            self.code().to_string()
        }

        fn __repr__(&self) -> String {
            format!("<ResultCode.{}: {}>", self.name(), self.code())
        }

        // Expose enum instances as class attributes (UPPER_SNAKE_CASE for Pythonic constants)
        #[classattr]
        fn OK() -> ResultCode { ResultCode::from(CoreResultCode::Ok) }
        #[classattr]
        fn SERVER_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::ServerError) }
        #[classattr]
        fn KEY_NOT_FOUND_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::KeyNotFoundError) }
        #[classattr]
        fn GENERATION_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::GenerationError) }
        #[classattr]
        fn PARAMETER_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::ParameterError) }
        #[classattr]
        fn KEY_EXISTS_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::KeyExistsError) }
        #[classattr]
        fn BIN_EXISTS_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::BinExistsError) }
        #[classattr]
        fn CLUSTER_KEY_MISMATCH() -> ResultCode { ResultCode::from(CoreResultCode::ClusterKeyMismatch) }
        #[classattr]
        fn SERVER_MEM_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::ServerMemError) }
        #[classattr]
        fn TIMEOUT() -> ResultCode { ResultCode::from(CoreResultCode::Timeout) }
        #[classattr]
        fn ALWAYS_FORBIDDEN() -> ResultCode { ResultCode::from(CoreResultCode::AlwaysForbidden) }
        #[classattr]
        fn PARTITION_UNAVAILABLE() -> ResultCode { ResultCode::from(CoreResultCode::PartitionUnavailable) }
        #[classattr]
        fn BIN_TYPE_ERROR() -> ResultCode { ResultCode::from(CoreResultCode::BinTypeError) }
        #[classattr]
        fn RECORD_TOO_BIG() -> ResultCode { ResultCode::from(CoreResultCode::RecordTooBig) }
        #[classattr]
        fn KEY_BUSY() -> ResultCode { ResultCode::from(CoreResultCode::KeyBusy) }
        #[classattr]
        fn SCAN_ABORT() -> ResultCode { ResultCode::from(CoreResultCode::ScanAbort) }
        #[classattr]
        fn UNSUPPORTED_FEATURE() -> ResultCode { ResultCode::from(CoreResultCode::UnsupportedFeature) }
        #[classattr]
        fn BIN_NOT_FOUND() -> ResultCode { ResultCode::from(CoreResultCode::BinNotFound) }
        #[classattr]
        fn DEVICE_OVERLOAD() -> ResultCode { ResultCode::from(CoreResultCode::DeviceOverload) }
        #[classattr]
        fn KEY_MISMATCH() -> ResultCode { ResultCode::from(CoreResultCode::KeyMismatch) }
        #[classattr]
        fn INVALID_NAMESPACE() -> ResultCode { ResultCode::from(CoreResultCode::InvalidNamespace) }
        #[classattr]
        fn BIN_NAME_TOO_LONG() -> ResultCode { ResultCode::from(CoreResultCode::BinNameTooLong) }
        #[classattr]
        fn FAIL_FORBIDDEN() -> ResultCode { ResultCode::from(CoreResultCode::FailForbidden) }
        #[classattr]
        fn ELEMENT_NOT_FOUND() -> ResultCode { ResultCode::from(CoreResultCode::ElementNotFound) }
        #[classattr]
        fn ELEMENT_EXISTS() -> ResultCode { ResultCode::from(CoreResultCode::ElementExists) }
        #[classattr]
        fn ENTERPRISE_ONLY() -> ResultCode { ResultCode::from(CoreResultCode::EnterpriseOnly) }
        #[classattr]
        fn OP_NOT_APPLICABLE() -> ResultCode { ResultCode::from(CoreResultCode::OpNotApplicable) }
        #[classattr]
        fn FILTERED_OUT() -> ResultCode { ResultCode::from(CoreResultCode::FilteredOut) }
        #[classattr]
        fn LOST_CONFLICT() -> ResultCode { ResultCode::from(CoreResultCode::LostConflict) }
        #[classattr]
        fn INVALID_ENCODING() -> ResultCode { ResultCode::from(CoreResultCode::InvalidEncoding) }
        #[classattr]
        fn XDR_KEY_BUSY() -> ResultCode { ResultCode::from(CoreResultCode::XdrKeyBusy) }
        #[classattr]
        fn QUERY_END() -> ResultCode { ResultCode::from(CoreResultCode::QueryEnd) }
        #[classattr]
        fn SECURITY_NOT_SUPPORTED() -> ResultCode { ResultCode::from(CoreResultCode::SecurityNotSupported) }
        #[classattr]
        fn SECURITY_NOT_ENABLED() -> ResultCode { ResultCode::from(CoreResultCode::SecurityNotEnabled) }
        #[classattr]
        fn NOT_AUTHENTICATED() -> ResultCode { ResultCode::from(CoreResultCode::NotAuthenticated) }
        #[classattr]
        fn SECURITY_SCHEME_NOT_SUPPORTED() -> ResultCode { ResultCode::from(CoreResultCode::SecuritySchemeNotSupported) }
        #[classattr]
        fn INVALID_COMMAND() -> ResultCode { ResultCode::from(CoreResultCode::InvalidCommand) }
        #[classattr]
        fn INVALID_FIELD() -> ResultCode { ResultCode::from(CoreResultCode::InvalidField) }
        #[classattr]
        fn ILLEGAL_STATE() -> ResultCode { ResultCode::from(CoreResultCode::IllegalState) }
        #[classattr]
        fn INVALID_USER() -> ResultCode { ResultCode::from(CoreResultCode::InvalidUser) }
        #[classattr]
        fn USER_ALREADY_EXISTS() -> ResultCode { ResultCode::from(CoreResultCode::UserAlreadyExists) }
        #[classattr]
        fn FORBIDDEN_PASSWORD() -> ResultCode { ResultCode::from(CoreResultCode::ForbiddenPassword) }
        #[classattr]
        fn UDF_BAD_RESPONSE() -> ResultCode { ResultCode::from(CoreResultCode::UdfBadResponse) }
        #[classattr]
        fn INDEX_FOUND() -> ResultCode { ResultCode::from(CoreResultCode::IndexFound) }
        #[classattr]
        fn INDEX_NOT_FOUND() -> ResultCode { ResultCode::from(CoreResultCode::IndexNotFound) }
        #[classattr]
        fn INDEX_OOM() -> ResultCode { ResultCode::from(CoreResultCode::IndexOom) }
        #[classattr]
        fn INDEX_NOT_READABLE() -> ResultCode { ResultCode::from(CoreResultCode::IndexNotReadable) }
        #[classattr]
        fn INDEX_GENERIC() -> ResultCode { ResultCode::from(CoreResultCode::IndexGeneric) }
        #[classattr]
        fn INDEX_NAME_MAX_LEN() -> ResultCode { ResultCode::from(CoreResultCode::IndexNameMaxLen) }
        #[classattr]
        fn INDEX_MAX_COUNT() -> ResultCode { ResultCode::from(CoreResultCode::IndexMaxCount) }
        #[classattr]
        fn QUERY_ABORTED() -> ResultCode { ResultCode::from(CoreResultCode::QueryAborted) }
        #[classattr]
        fn QUERY_QUEUE_FULL() -> ResultCode { ResultCode::from(CoreResultCode::QueryQueueFull) }
        #[classattr]
        fn QUERY_TIMEOUT() -> ResultCode { ResultCode::from(CoreResultCode::QueryTimeout) }
        #[classattr]
        fn QUERY_GENERIC() -> ResultCode { ResultCode::from(CoreResultCode::QueryGeneric) }
        #[classattr]
        fn MRT_BLOCKED() -> ResultCode { ResultCode::from(CoreResultCode::MrtBlocked) }
        #[classattr]
        fn MRT_VERSION_MISMATCH() -> ResultCode { ResultCode::from(CoreResultCode::MrtVersionMismatch) }
        #[classattr]
        fn MRT_EXPIRED() -> ResultCode { ResultCode::from(CoreResultCode::MrtExpired) }
        #[classattr]
        fn MRT_TOO_MANY_WRITES() -> ResultCode { ResultCode::from(CoreResultCode::MrtTooManyWrites) }
        #[classattr]
        fn MRT_COMMITTED() -> ResultCode { ResultCode::from(CoreResultCode::MrtCommitted) }
        #[classattr]
        fn MRT_ABORTED() -> ResultCode { ResultCode::from(CoreResultCode::MrtAborted) }
        #[classattr]
        fn MRT_ALREADY_LOCKED() -> ResultCode { ResultCode::from(CoreResultCode::MrtAlreadyLocked) }
        #[classattr]
        fn MRT_MONITOR_EXISTS() -> ResultCode { ResultCode::from(CoreResultCode::MrtMonitorExists) }

        // Security family
        #[classattr]
        fn INVALID_PASSWORD() -> ResultCode { ResultCode::from(CoreResultCode::InvalidPassword) }
        #[classattr]
        fn EXPIRED_PASSWORD() -> ResultCode { ResultCode::from(CoreResultCode::ExpiredPassword) }
        #[classattr]
        fn INVALID_CREDENTIAL() -> ResultCode { ResultCode::from(CoreResultCode::InvalidCredential) }
        #[classattr]
        fn EXPIRED_SESSION() -> ResultCode { ResultCode::from(CoreResultCode::ExpiredSession) }
        #[classattr]
        fn INVALID_ROLE() -> ResultCode { ResultCode::from(CoreResultCode::InvalidRole) }
        #[classattr]
        fn ROLE_ALREADY_EXISTS() -> ResultCode { ResultCode::from(CoreResultCode::RoleAlreadyExists) }
        #[classattr]
        fn INVALID_PRIVILEGE() -> ResultCode { ResultCode::from(CoreResultCode::InvalidPrivilege) }
        #[classattr]
        fn INVALID_ALLOWLIST() -> ResultCode { ResultCode::from(CoreResultCode::InvalidAllowlist) }
        #[classattr]
        fn ROLE_VIOLATION() -> ResultCode { ResultCode::from(CoreResultCode::RoleViolation) }
        #[classattr]
        fn NOT_ALLOWLISTED() -> ResultCode { ResultCode::from(CoreResultCode::NotAllowlisted) }

        // Quota family
        #[classattr]
        fn QUOTAS_NOT_ENABLED() -> ResultCode { ResultCode::from(CoreResultCode::QuotasNotEnabled) }
        #[classattr]
        fn INVALID_QUOTA() -> ResultCode { ResultCode::from(CoreResultCode::InvalidQuota) }
        #[classattr]
        fn QUOTA_EXCEEDED() -> ResultCode { ResultCode::from(CoreResultCode::QuotaExceeded) }

        // Batch family
        #[classattr]
        fn BATCH_DISABLED() -> ResultCode { ResultCode::from(CoreResultCode::BatchDisabled) }
        #[classattr]
        fn BATCH_MAX_REQUESTS_EXCEEDED() -> ResultCode { ResultCode::from(CoreResultCode::BatchMaxRequestsExceeded) }
        #[classattr]
        fn BATCH_QUEUES_FULL() -> ResultCode { ResultCode::from(CoreResultCode::BatchQueuesFull) }

        // Remaining server codes
        #[classattr]
        fn INVALID_GEOJSON() -> ResultCode { ResultCode::from(CoreResultCode::InvalidGeojson) }
        #[classattr]
        fn QUERY_NETIO_ERR() -> ResultCode { ResultCode::from(CoreResultCode::QueryNetioErr) }
        #[classattr]
        fn QUERY_DUPLICATE() -> ResultCode { ResultCode::from(CoreResultCode::QueryDuplicate) }

        // Client-assigned codes: the negative half of the namespace.
        #[classattr]
        fn TXN_ALREADY_ABORTED() -> ResultCode { ResultCode::from(ClientResultCode::TxnAlreadyAborted) }
        #[classattr]
        fn TXN_ALREADY_COMMITTED() -> ResultCode { ResultCode::from(ClientResultCode::TxnAlreadyCommitted) }
        #[classattr]
        fn TXN_FAILED() -> ResultCode { ResultCode::from(ClientResultCode::TxnFailed) }
        #[classattr]
        fn BATCH_FAILED() -> ResultCode { ResultCode::from(ClientResultCode::BatchFailed) }
        #[classattr]
        fn NO_RESPONSE() -> ResultCode { ResultCode::from(ClientResultCode::NoResponse) }
        #[classattr]
        fn MAX_ERROR_RATE() -> ResultCode { ResultCode::from(ClientResultCode::MaxErrorRate) }
        #[classattr]
        fn MAX_RETRIES_EXCEEDED() -> ResultCode { ResultCode::from(ClientResultCode::MaxRetriesExceeded) }
        #[classattr]
        fn SERIALIZE_ERROR() -> ResultCode { ResultCode::from(ClientResultCode::SerializeError) }
        #[classattr]
        fn SERVER_NOT_AVAILABLE() -> ResultCode { ResultCode::from(ClientResultCode::ServerNotAvailable) }
        #[classattr]
        fn NO_MORE_CONNECTIONS() -> ResultCode { ResultCode::from(ClientResultCode::NoMoreConnections) }
        #[classattr]
        fn QUERY_TERMINATED() -> ResultCode { ResultCode::from(ClientResultCode::QueryTerminated) }
        #[classattr]
        fn SCAN_TERMINATED() -> ResultCode { ResultCode::from(ClientResultCode::ScanTerminated) }
        #[classattr]
        fn INVALID_NODE_ERROR() -> ResultCode { ResultCode::from(ClientResultCode::InvalidNodeError) }
        #[classattr]
        fn PARSE_ERROR() -> ResultCode { ResultCode::from(ClientResultCode::ParseError) }
        #[classattr]
        fn CLIENT_ERROR() -> ResultCode { ResultCode::from(ClientResultCode::ClientError) }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  TxnState
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TxnState {
        #[pyo3(name = "OPEN")]
        Open,
        #[pyo3(name = "VERIFIED")]
        Verified,
        #[pyo3(name = "COMMITTED")]
        Committed,
        #[pyo3(name = "ABORTED")]
        Aborted,
        /// A commit failed with an in-doubt outcome. The server may still be
        /// rolling the transaction forward, so abort is refused; retry the
        /// commit instead.
        #[pyo3(name = "COMMIT_FAILED")]
        CommitFailed,
    }

    #[pymethods]
    impl TxnState {
        fn __richcmp__(&self, other: &TxnState, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<aerospike_core::TxnState> for TxnState {
        fn from(s: aerospike_core::TxnState) -> Self {
            match s {
                aerospike_core::TxnState::Open => TxnState::Open,
                aerospike_core::TxnState::Verified => TxnState::Verified,
                aerospike_core::TxnState::Committed => TxnState::Committed,
                aerospike_core::TxnState::Aborted => TxnState::Aborted,
                aerospike_core::TxnState::CommitFailed => TxnState::CommitFailed,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CommitStatus
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CommitStatus {
        #[pyo3(name = "OK")]
        Ok,
        #[pyo3(name = "ALREADY_COMMITTED")]
        AlreadyCommitted,
        #[pyo3(name = "ROLL_FORWARD_ABANDONED")]
        RollForwardAbandoned,
        #[pyo3(name = "CLOSE_ABANDONED")]
        CloseAbandoned,
    }

    #[pymethods]
    impl CommitStatus {
        fn __richcmp__(&self, other: &CommitStatus, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<aerospike_core::CommitStatus> for CommitStatus {
        fn from(s: aerospike_core::CommitStatus) -> Self {
            match s {
                aerospike_core::CommitStatus::Ok => CommitStatus::Ok,
                aerospike_core::CommitStatus::AlreadyCommitted => CommitStatus::AlreadyCommitted,
                aerospike_core::CommitStatus::RollForwardAbandoned => CommitStatus::RollForwardAbandoned,
                aerospike_core::CommitStatus::CloseAbandoned => CommitStatus::CloseAbandoned,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CommitErrorType
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Which stage of a transaction commit failed.
    ///
    /// Carried on :exc:`aerospike_native.exceptions.CommitFailedError` as
    /// ``commit_error_type``. The distinction matters for recovery: a plain
    /// verify failure leaves nothing applied; ``MARK_ROLL_FORWARD_ABANDONED``
    /// means the server will abort; ``ROLL_FORWARD_ABANDONED`` means the
    /// writes are not yet visible and the server will eventually commit.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CommitErrorType {
        #[pyo3(name = "VERIFY_FAIL")]
        VerifyFail,
        #[pyo3(name = "VERIFY_FAIL_CLOSE_ABANDONED")]
        VerifyFailCloseAbandoned,
        #[pyo3(name = "VERIFY_FAIL_ABORT_ABANDONED")]
        VerifyFailAbortAbandoned,
        #[pyo3(name = "MARK_ROLL_FORWARD_ABANDONED")]
        MarkRollForwardAbandoned,
        #[pyo3(name = "ROLL_FORWARD_ABANDONED")]
        RollForwardAbandoned,
    }

    #[pymethods]
    impl CommitErrorType {
        fn __richcmp__(
            &self, other: &CommitErrorType, op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
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

    impl From<aerospike_core::txn::CommitErrorType> for CommitErrorType {
        fn from(s: aerospike_core::txn::CommitErrorType) -> Self {
            match s {
                aerospike_core::txn::CommitErrorType::VerifyFail => CommitErrorType::VerifyFail,
                aerospike_core::txn::CommitErrorType::VerifyFailCloseAbandoned => {
                    CommitErrorType::VerifyFailCloseAbandoned
                }
                aerospike_core::txn::CommitErrorType::VerifyFailAbortAbandoned => {
                    CommitErrorType::VerifyFailAbortAbandoned
                }
                aerospike_core::txn::CommitErrorType::MarkRollForwardAbandoned => {
                    CommitErrorType::MarkRollForwardAbandoned
                }
                aerospike_core::txn::CommitErrorType::RollForwardAbandoned => {
                    CommitErrorType::RollForwardAbandoned
                }
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  AbortStatus
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AbortStatus {
        #[pyo3(name = "OK")]
        Ok,
        #[pyo3(name = "ALREADY_ABORTED")]
        AlreadyAborted,
        #[pyo3(name = "ROLL_BACK_ABANDONED")]
        RollBackAbandoned,
        #[pyo3(name = "CLOSE_ABANDONED")]
        CloseAbandoned,
        /// Abort was refused because a commit already failed on this
        /// transaction with an in-doubt outcome. Retry the commit instead.
        #[pyo3(name = "COMMIT_FAILED")]
        CommitFailed,
    }

    #[pymethods]
    impl AbortStatus {
        fn __richcmp__(&self, other: &AbortStatus, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

    impl From<aerospike_core::AbortStatus> for AbortStatus {
        fn from(s: aerospike_core::AbortStatus) -> Self {
            match s {
                aerospike_core::AbortStatus::Ok => AbortStatus::Ok,
                aerospike_core::AbortStatus::AlreadyAborted => AbortStatus::AlreadyAborted,
                aerospike_core::AbortStatus::RollBackAbandoned => AbortStatus::RollBackAbandoned,
                aerospike_core::AbortStatus::CloseAbandoned => AbortStatus::CloseAbandoned,
                aerospike_core::AbortStatus::CommitFailed => AbortStatus::CommitFailed,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  LoopVarPart
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Identifies which element of a loop variable to access in path expressions.
    ///
    /// Used with loop-variable expression constructors such as
    /// ``FilterExpression.int_loop_var``, ``FilterExpression.map_loop_var``, etc.
    ///
    /// Requires Aerospike Server version >= 8.1.1.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, name = "LoopVarPart", module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LoopVarPart(pub i64);

    #[gen_stub_pymethods]
    #[pymethods]
    impl LoopVarPart {
        /// Map key part of the loop variable.
        #[classattr]
        const MAP_KEY: LoopVarPart = LoopVarPart(0);
        /// Value part of the loop variable (list element or map value).
        #[classattr]
        const VALUE: LoopVarPart = LoopVarPart(1);
        /// Index part of the loop variable (parent list index).
        #[classattr]
        const INDEX: LoopVarPart = LoopVarPart(2);

        fn __richcmp__(&self, other: &LoopVarPart, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
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

        fn __repr__(&self) -> String {
            let name = match self.0 {
                0 => "MAP_KEY",
                1 => "VALUE",
                2 => "INDEX",
                _ => "UNKNOWN",
            };
            format!("LoopVarPart.{}", name)
        }
    }

    impl From<&LoopVarPart> for aerospike_core::expressions::LoopVarPart {
        fn from(p: &LoopVarPart) -> Self {
            aerospike_core::expressions::LoopVarPart::from_bits(p.0)
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  SelectFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Flags controlling the return value of a ``CdtOperation.select_by_path`` operation.
    ///
    /// Namespace of plain ``int`` constants. Combine with bitwise OR
    /// (``SelectFlags.VALUE | SelectFlags.NO_FAIL``) — the result is a regular
    /// ``int`` and can be passed directly to ``CdtOperation.select_by_path(..., flag=...)``.
    ///
    /// Requires Aerospike Server version >= 8.1.1.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(name = "SelectFlags", module = "_native")]
    pub struct SelectFlags;

    #[gen_stub_pymethods]
    #[pymethods]
    impl SelectFlags {
        /// Return the full matching subtree (root to leaf), keeping only matched nodes.
        #[classattr]
        const MATCHING_TREE: i64 = 0;
        /// Return the values of the finally-selected nodes.
        #[classattr]
        const VALUE: i64 = 1;
        /// Synonym for ``VALUE`` — clarifies list element expectations.
        #[classattr]
        const LIST_VALUE: i64 = 1;
        /// Synonym for ``VALUE`` — clarifies map value expectations.
        #[classattr]
        const MAP_VALUE: i64 = 1;
        /// Return only the map keys of the finally-selected nodes.
        #[classattr]
        const MAP_KEY: i64 = 2;
        /// Return map key-value pairs of the finally-selected nodes.
        #[classattr]
        const MAP_KEY_VALUE: i64 = 3;
        /// Ignore type mismatches instead of failing.
        #[classattr]
        const NO_FAIL: i64 = 0x10;
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ModifyFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Flags controlling the behavior of a ``CdtOperation.modify_by_path`` operation.
    ///
    /// Namespace of plain ``int`` constants. Combine with bitwise OR
    /// (``ModifyFlags.DEFAULT | ModifyFlags.NO_FAIL``) — the result is a regular
    /// ``int`` and can be passed directly to ``CdtOperation.modify_by_path(..., flag=...)``.
    ///
    /// Requires Aerospike Server version >= 8.1.1.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(name = "ModifyFlags", module = "_native")]
    pub struct ModifyFlags;

    #[gen_stub_pymethods]
    #[pymethods]
    impl ModifyFlags {
        /// Default behavior — fails on type mismatches.
        #[classattr]
        const DEFAULT: i64 = 0;
        /// Ignore type errors instead of failing.
        #[classattr]
        const NO_FAIL: i64 = 0x10;
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Top-K: OrderByType, Order
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Declared type of a Top-K order key, used with :meth:`Statement.set_order_by`.
    ///
    /// A record whose order-key bin is missing or holds another type ranks last.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum OrderByType {
        /// 64-bit signed integer.
        #[pyo3(name = "INTEGER")]
        Integer,
        /// 64-bit float. NaN ranks after every other value; ``-0.0`` equals ``0.0``.
        #[pyo3(name = "DOUBLE")]
        Double,
        /// Bytes, compared as unsigned bytes; a prefix ranks before the longer value.
        #[pyo3(name = "BYTES")]
        Bytes,
    }

    impl From<OrderByType> for aerospike_core::OrderByType {
        fn from(input: OrderByType) -> Self {
            match input {
                OrderByType::Integer => aerospike_core::OrderByType::Integer,
                OrderByType::Double => aerospike_core::OrderByType::Double,
                OrderByType::Bytes => aerospike_core::OrderByType::Bytes,
            }
        }
    }

    impl From<aerospike_core::OrderByType> for OrderByType {
        fn from(input: aerospike_core::OrderByType) -> Self {
            match input {
                aerospike_core::OrderByType::Integer => OrderByType::Integer,
                aerospike_core::OrderByType::Double => OrderByType::Double,
                aerospike_core::OrderByType::Bytes => OrderByType::Bytes,
                aerospike_core::OrderByType::String => {
                    unreachable!("order keys are only set through OrderByType, which has no STRING")
                }
            }
        }
    }

    /// Sort direction of a Top-K order key: which ``k`` records are kept and the
    /// order they are returned in.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Order {
        /// Smallest values first.
        #[pyo3(name = "ASC")]
        Asc,
        /// Largest values first.
        #[pyo3(name = "DESC")]
        Desc,
    }

    impl From<Order> for aerospike_core::Order {
        fn from(input: Order) -> Self {
            match input {
                Order::Asc => aerospike_core::Order::Asc,
                Order::Desc => aerospike_core::Order::Desc,
            }
        }
    }

    impl From<aerospike_core::Order> for Order {
        fn from(input: aerospike_core::Order) -> Self {
            match input {
                aerospike_core::Order::Asc => Order::Asc,
                aerospike_core::Order::Desc => Order::Desc,
            }
        }
    }

    macro_rules! impl_simple_enum_eq_hash {
        ($($ty:ty),*) => {$(
            #[pymethods]
            impl $ty {
                fn __richcmp__(&self, other: &$ty, op: CompareOp) -> PyResult<bool> {
                    match op {
                        CompareOp::Eq => Ok(self == other),
                        CompareOp::Ne => Ok(self != other),
                        _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                            "Only == and != comparisons are supported",
                        )),
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
        )*};
    }

    impl_simple_enum_eq_hash!(OrderByType, Order);
