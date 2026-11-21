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

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::CollectionIndexType;
use crate::Filter;
use crate::errors::RustClientError;

/// Server query plan selection inferred from an explain response.
#[gen_stub_pyclass_enum]
#[pyclass(from_py_object, module = "aerospike_native._native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuerySelection {
    #[pyo3(name = "PRIMARY_INDEX")]
    PrimaryIndex,
    #[pyo3(name = "SECONDARY_INDEX")]
    SecondaryIndex,
    #[pyo3(name = "FILTERED_OUT")]
    FilteredOut,
}

impl QuerySelection {
    fn py_name(self) -> &'static str {
        match self {
            QuerySelection::PrimaryIndex => "PRIMARY_INDEX",
            QuerySelection::SecondaryIndex => "SECONDARY_INDEX",
            QuerySelection::FilteredOut => "FILTERED_OUT",
        }
    }
}

impl From<aerospike_core::query::QuerySelection> for QuerySelection {
    fn from(value: aerospike_core::query::QuerySelection) -> Self {
        match value {
            aerospike_core::query::QuerySelection::PrimaryIndex => QuerySelection::PrimaryIndex,
            aerospike_core::query::QuerySelection::SecondaryIndex => {
                QuerySelection::SecondaryIndex
            }
            aerospike_core::query::QuerySelection::FilteredOut => QuerySelection::FilteredOut,
        }
    }
}

fn opt_repr(value: Option<&str>) -> String {
    match value {
        Some(v) => format!("{v:?}"),
        None => "None".to_string(),
    }
}

fn core_collection_index_type(cit: &aerospike_core::CollectionIndexType) -> CollectionIndexType {
    match cit {
        aerospike_core::CollectionIndexType::Default => CollectionIndexType::Default,
        aerospike_core::CollectionIndexType::List => CollectionIndexType::List,
        aerospike_core::CollectionIndexType::MapKeys => CollectionIndexType::MapKeys,
        aerospike_core::CollectionIndexType::MapValues => CollectionIndexType::MapValues,
        aerospike_core::CollectionIndexType::Set => CollectionIndexType::Set,
    }
}

/// Result of a server query explain (phase 1).
///
/// Combines the client-authored field ``44`` explain payload (AEL and flags) with
/// the server's index selection (``selection``, ``index_name``, index range, and
/// ``index_type``). :meth:`Client.query_with_plan` replays the stored field ``44``
/// payload on execute with the explain flag cleared.
#[gen_stub_pyclass]
#[pyclass(from_py_object, name = "QueryPlan", module = "aerospike_native._native")]
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub(crate) _as: aerospike_core::QueryPlan,
    ael: String,
    where_flags: u8,
}

impl QueryPlan {
    /// ``where_flags`` are the effective field ``44`` flags the explain was sent
    /// with; core's stored payload is private, so callers pass what they resolved.
    pub(crate) fn from_core(plan: aerospike_core::QueryPlan, where_flags: u8) -> PyResult<Self> {
        let ael = plan.ael().map_err(|e| PyErr::from(RustClientError(e)))?;
        Ok(QueryPlan {
            _as: plan,
            ael,
            where_flags,
        })
    }
}

pub(crate) fn validate_plan_matches_statement(
    statement: &aerospike_core::Statement,
    plan: &aerospike_core::QueryPlan,
) -> PyResult<()> {
    if statement.namespace != plan.namespace() {
        return Err(PyErr::from(RustClientError(
            aerospike_core::Error::invalid_argument(format!(
                "Query plan namespace '{}' does not match statement namespace '{}'",
                plan.namespace(),
                statement.namespace,
            )),
        )));
    }
    let stmt_set = if statement.set_name.is_empty() {
        None
    } else {
        Some(statement.set_name.as_str())
    };
    if stmt_set != plan.set_name() {
        return Err(PyErr::from(RustClientError(
            aerospike_core::Error::invalid_argument(format!(
                "Query plan set '{}' does not match statement set '{}'",
                plan.set_name().unwrap_or(""),
                stmt_set.unwrap_or(""),
            )),
        )));
    }
    if statement.filter.is_some() {
        return Err(PyErr::from(RustClientError(
            aerospike_core::Error::invalid_argument(
                "Statement must not carry filters when executing a query plan; \
                 the plan supplies the index filter",
            ),
        )));
    }
    Ok(())
}

#[gen_stub_pymethods]
#[pymethods]
impl QueryPlan {
    #[getter]
    pub fn selection(&self) -> QuerySelection {
        self._as.selection().into()
    }

    #[getter]
    pub fn namespace(&self) -> &str {
        self._as.namespace()
    }

    #[getter]
    pub fn set_name(&self) -> Option<&str> {
        self._as.set_name()
    }

    /// AEL passed to :meth:`Client.query_explain`; stored on the plan for execute replay.
    #[getter]
    pub fn ael(&self) -> &str {
        &self.ael
    }

    /// Secondary-index name when :attr:`selection` is ``SECONDARY_INDEX``; ``None`` otherwise (PI or filtered-out).
    #[getter]
    pub fn index_name(&self) -> Option<&str> {
        self._as.index_name()
    }

    /// Opaque ``INDEX_RANGE`` payload (explain field ``22``) describing the
    /// secondary-index range the server chose; ``None`` when no index was chosen
    /// (PI or filtered-out). Diagnostic only — the layout is server-defined.
    #[getter]
    pub fn index_range_bytes<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self._as.index_range_bytes().map(|bytes| PyBytes::new(py, bytes))
    }

    #[getter]
    pub fn index_type(&self) -> CollectionIndexType {
        core_collection_index_type(self._as.index_type())
    }

    #[getter]
    pub fn is_primary_index(&self) -> bool {
        self._as.is_primary_index()
    }

    #[getter]
    pub fn is_secondary_index(&self) -> bool {
        self._as.is_secondary_index()
    }

    #[getter]
    pub fn is_filtered_out(&self) -> bool {
        self._as.is_filtered_out()
    }

    /// Effective field ``44`` WHERE flags the explain was sent with
    /// (bitwise-OR of ``EXPLAIN`` / ``REQUIRE_INDEX`` / ``HARD_HINT``).
    /// Diagnostic — mirrors the ``explain_where_flags`` the plan resolved from.
    #[getter]
    pub fn where_flags(&self) -> u8 {
        self.where_flags
    }

    /// Returns the secondary-index ``Filter`` the server selected, or ``None``.
    ///
    /// Diagnostic only — :meth:`Client.query_with_plan` derives this internally,
    /// so callers never pass it. Use it to inspect which index range the plan
    /// resolved to. Returns ``None`` for primary-index and filtered-out plans.
    /// Rebuilds the filter on each call.
    pub fn filter_for_execute(&self) -> PyResult<Option<Filter>> {
        match self._as.filter_for_execute() {
            Ok(Some(filter)) => Ok(Some(Filter { _as: filter })),
            Ok(None) => Ok(None),
            Err(e) => Err(PyErr::from(RustClientError(e))),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "QueryPlan(selection={}, namespace={:?}, set_name={}, index_name={}, \
             where_flags={:#04x}, ael={:?})",
            self.selection().py_name(),
            self._as.namespace(),
            opt_repr(self._as.set_name()),
            opt_repr(self._as.index_name()),
            self.where_flags,
            self.ael,
        )
    }
}

/// Field ``44`` (WHERE) flag bits for server query explain (phase 1).
///
/// Combine with bitwise OR and pass to :meth:`Client.query_explain` /
/// :meth:`Client.query_explain_blocking` as ``explain_where_flags``.
/// Omit the argument (or pass ``None``) for default explain
/// (``QueryWhereFlags.EXPLAIN`` only).
///
/// Requires Aerospike Server version >= 8.2.0. Callers must verify
/// :meth:`Version.supports_query_selection` before use.
#[gen_stub_pyclass]
#[pyclass(name = "QueryWhereFlags", module = "aerospike_native._native")]
pub struct QueryWhereFlags;

#[gen_stub_pymethods]
#[pymethods]
impl QueryWhereFlags {
    /// Reserved for wire continuation; passing it raises
    /// :exc:`aerospike_native.exceptions.ValueError`.
    #[classattr]
    const ENC_VARINT: i64 = aerospike_core::FLAG_ENC_VARINT as i64;

    /// Explain phase — server runs index planner only (always set on explain).
    #[classattr]
    const EXPLAIN: i64 = aerospike_core::FLAG_EXPLAIN as i64;

    /// Reject primary-index fallback on explain when combined with ``EXPLAIN``.
    #[classattr]
    const REQUIRE_INDEX: i64 = aerospike_core::FLAG_REQUIRE_INDEX as i64;

    /// Require field ``21`` index name hint; fail if hint missing or not selected.
    #[classattr]
    const HARD_HINT: i64 = aerospike_core::FLAG_HARD_HINT as i64;
}
